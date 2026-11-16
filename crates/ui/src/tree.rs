//! Persistent arena tree used by the immediate API.
//!
//! Every frame the immediate widget calls rebuild the visible part of the
//! tree. Nodes are keyed by stable ids, so per-widget state (scroll offset,
//! dropdown open flag, active pointer) survives between frames without any
//! `Rc<RefCell<_>>` machinery.

use std::collections::HashMap;

use glam::Vec2;

use crate::geometry::Rect;
use crate::layout::{AlignItems, JustifyContent, Layout, Size, UI_ROOT_ID, UiId};

/// Discriminator used for automatic ids and rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WidgetKind {
    Container,
    Label,
    Button,
    Toggle,
    Slider,
    Progress,
    Dropdown,
    ScrollPanel,
    Spacer,
    Separator,
}

impl WidgetKind {
    pub fn auto_id(parent: UiId, index: u32, kind: WidgetKind) -> UiId {
        crate::layout::stable_id((parent, index, kind as u32))
    }
}

/// Interaction capabilities requested by a widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Sense {
    pub click: bool,
    pub drag: bool,
}

impl Sense {
    pub const NONE: Self = Self {
        click: false,
        drag: false,
    };
    pub const CLICK: Self = Self {
        click: true,
        drag: false,
    };
    pub const DRAG: Self = Self {
        click: true,
        drag: true,
    };
}

/// Result of a widget's interaction for the current frame.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Response {
    pub id: UiId,
    pub hovered: bool,
    pub pressed: bool,
    pub active: bool,
    pub clicked: bool,
    pub secondary_clicked: bool,
    pub released: bool,
    pub dragged: bool,
    pub changed: bool,
    pub drag_delta: Vec2,
}

impl Response {
    pub fn has_focus(&self) -> bool {
        self.active
    }
}

/// One node of the retained tree.
#[derive(Debug, Clone)]
pub struct Node {
    pub id: UiId,
    pub kind: WidgetKind,
    pub parent: Option<UiId>,
    pub children: Vec<UiId>,
    pub layout: Layout,

    /// Rectangle computed by the last layout pass. Until the first layout it
    /// is empty, which also means the node cannot be interacted with yet.
    pub rect: Rect,
    pub clip: Rect,
    /// Intrinsic content size reported by the widget (text, joystick, ...).
    pub content_size: Vec2,
    /// Size including padding produced by the last measure pass.
    pub measured: Vec2,

    pub used: bool,
    pub visible: bool,
    pub interactive: bool,
    pub sense: Sense,

    pub hovered: bool,
    pub pressed: bool,
    pub active: bool,
    pub clicked: bool,
    pub released: bool,
    pub dragged: bool,
    pub changed: bool,
    pub drag_delta: Vec2,

    pub text: String,
    pub font_size: f32,
    pub text_color: [f32; 4],
    pub text_align: text::TextAlign,
    pub text_wrap: Option<f32>,

    pub background: Option<[f32; 4]>,
    pub hover_color: Option<[f32; 4]>,
    pub pressed_color: Option<[f32; 4]>,
    pub fill_color: Option<[f32; 4]>,
    pub border: Option<[f32; 4]>,
    pub radius: f32,

    /// Generic scalar used by sliders and progress bars.
    pub value: f32,
    pub min: f32,
    pub max_value: f32,
    pub toggled: bool,
    pub show_value: bool,

    pub scroll: f32,
    pub scroll_max: f32,
    pub scrollbar_rect: Rect,

    pub open: bool,
    pub items: Vec<String>,
    pub selected: usize,
    pub hovered_item: Option<usize>,
    pub popup_rect: Rect,
}

impl Node {
    fn new(id: UiId, kind: WidgetKind, parent: Option<UiId>) -> Self {
        Self {
            id,
            kind,
            parent,
            children: Vec::new(),
            layout: Layout::default(),
            rect: Rect::ZERO,
            clip: Rect::ZERO,
            content_size: Vec2::ZERO,
            measured: Vec2::ZERO,
            used: false,
            visible: true,
            interactive: false,
            sense: Sense::NONE,
            hovered: false,
            pressed: false,
            active: false,
            clicked: false,
            released: false,
            dragged: false,
            changed: false,
            drag_delta: Vec2::ZERO,
            text: String::new(),
            font_size: 16.0,
            text_color: [1.0, 1.0, 1.0, 1.0],
            text_align: text::TextAlign::Left,
            text_wrap: None,
            background: None,
            hover_color: None,
            pressed_color: None,
            fill_color: None,
            border: None,
            radius: 0.0,
            value: 0.0,
            min: 0.0,
            max_value: 1.0,
            toggled: false,
            show_value: false,
            scroll: 0.0,
            scroll_max: 0.0,
            scrollbar_rect: Rect::ZERO,
            open: false,
            items: Vec::new(),
            selected: 0,
            hovered_item: None,
            popup_rect: Rect::ZERO,
        }
    }

    pub fn response(&self) -> Response {
        Response {
            id: self.id,
            hovered: self.hovered,
            pressed: self.pressed,
            active: self.active,
            clicked: self.clicked,
            secondary_clicked: false,
            released: self.released,
            dragged: self.dragged,
            changed: self.changed,
            drag_delta: self.drag_delta,
        }
    }

    fn reset_frame_state(&mut self) {
        self.used = false;
        self.visible = true;
        self.interactive = false;
        self.sense = Sense::NONE;
        self.children.clear();
        self.hovered = false;
        self.pressed = false;
        self.active = false;
        self.clicked = false;
        self.released = false;
        self.dragged = false;
        self.changed = false;
        self.drag_delta = Vec2::ZERO;
        self.content_size = Vec2::ZERO;
        self.text.clear();
        self.show_value = false;
        self.background = None;
        self.hover_color = None;
        self.pressed_color = None;
        self.fill_color = None;
        self.border = None;
        self.text_wrap = None;
        self.hovered_item = None;
        self.items.clear();
        self.popup_rect = Rect::ZERO;
    }
}

/// Arena tree with per-frame layout.
#[derive(Debug)]
pub struct UiTree {
    nodes: HashMap<UiId, Node>,
    roots: Vec<UiId>,
    order: Vec<UiId>,
    scroll_requests: Vec<(UiId, f32)>,
    pub viewport: Rect,
}

impl Default for UiTree {
    fn default() -> Self {
        Self::new()
    }
}

impl UiTree {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            roots: Vec::new(),
            order: Vec::new(),
            scroll_requests: Vec::new(),
            viewport: Rect::ZERO,
        }
    }

    pub fn begin_frame(&mut self, viewport: Rect) {
        self.viewport = viewport;
        self.roots.clear();
        self.order.clear();
        self.scroll_requests.clear();
        for node in self.nodes.values_mut() {
            node.reset_frame_state();
        }
        let root = self
            .nodes
            .entry(UI_ROOT_ID)
            .or_insert_with(|| Node::new(UI_ROOT_ID, WidgetKind::Container, None));
        root.used = true;
        root.rect = viewport;
        root.clip = viewport;
    }

    pub fn end_frame(&mut self) {
        self.nodes.retain(|_, node| node.used);
    }

    pub fn node(&self, id: UiId) -> Option<&Node> {
        self.nodes.get(&id)
    }

    pub fn node_mut(&mut self, id: UiId) -> Option<&mut Node> {
        self.nodes.get_mut(&id)
    }

    pub fn nodes_in_order(&self) -> impl Iterator<Item = &Node> {
        self.order.iter().filter_map(|id| self.nodes.get(id))
    }

    pub fn roots(&self) -> &[UiId] {
        &self.roots
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.len() <= 1
    }

    /// Creates or updates a node and attaches it to `parent`.
    pub fn create(
        &mut self,
        id: UiId,
        kind: WidgetKind,
        parent: UiId,
        layout: Layout,
    ) -> &mut Node {
        let node = self
            .nodes
            .entry(id)
            .or_insert_with(|| Node::new(id, kind, Some(parent)));
        node.kind = kind;
        node.parent = if parent == UI_ROOT_ID {
            None
        } else {
            Some(parent)
        };
        node.layout = layout;
        node.used = true;
        node.visible = true;

        if parent == UI_ROOT_ID {
            if !self.roots.contains(&id) {
                self.roots.push(id);
            }
        } else {
            let attach = self
                .nodes
                .get(&parent)
                .map(|parent_node| !parent_node.children.contains(&id))
                .unwrap_or(false);
            if attach && let Some(parent_node) = self.nodes.get_mut(&parent) {
                parent_node.children.push(id);
            }
        }

        if !self.order.contains(&id) {
            self.order.push(id);
        }
        self.nodes.get_mut(&id).expect("node was just inserted")
    }

    /// Requests that the wheel/touch scroll delta is applied to `id`.
    pub fn request_scroll(&mut self, id: UiId, delta: f32) {
        if delta != 0.0 {
            self.scroll_requests.push((id, delta));
        }
    }

    /// Computes sizes and positions for every visible node.
    pub fn layout(&mut self) {
        self.apply_scroll_requests();

        let viewport = self.viewport;
        let roots = self.roots.clone();
        for root in roots {
            let (width_size, height_size) = self
                .nodes
                .get(&root)
                .map(|node| (node.layout.width, node.layout.height))
                .unwrap_or((Size::Fill, Size::Fill));
            let measured = self.measure_node(root, viewport.size());
            let width = if matches!(width_size, Size::Auto | Size::Fill) {
                viewport.w
            } else {
                width_size.resolve(viewport.w, measured.x).min(viewport.w)
            };
            let height = if matches!(height_size, Size::Auto | Size::Fill) {
                viewport.h
            } else {
                height_size.resolve(viewport.h, measured.y).min(viewport.h)
            };
            // A root with an explicit size becomes a centered panel; a
            // full-size root is a regular HUD layer anchored at the origin.
            let center_x = !matches!(width_size, Size::Auto | Size::Fill);
            let center_y = !matches!(height_size, Size::Auto | Size::Fill);
            let origin = Vec2::new(
                viewport.x
                    + if center_x {
                        (viewport.w - width) * 0.5
                    } else {
                        0.0
                    },
                viewport.y
                    + if center_y {
                        (viewport.h - height) * 0.5
                    } else {
                        0.0
                    },
            );
            self.arrange_node(root, Rect::new(origin.x, origin.y, width, height), viewport);
        }
    }

    fn apply_scroll_requests(&mut self) {
        if self.scroll_requests.is_empty() {
            return;
        }
        // Nested scroll panels receive the wheel: pick the innermost one.
        let mut best: Option<(usize, UiId, f32)> = None;
        for &(id, delta) in &self.scroll_requests {
            let depth = self.depth(id);
            if best.is_none_or(|(best_depth, _, _)| depth > best_depth) {
                best = Some((depth, id, delta));
            }
        }
        if let Some((_, id, delta)) = best
            && let Some(node) = self.nodes.get_mut(&id)
        {
            node.scroll += delta * 48.0;
        }
        self.scroll_requests.clear();
    }

    fn depth(&self, id: UiId) -> usize {
        let mut depth = 0;
        let mut current = self.nodes.get(&id).and_then(|node| node.parent);
        while let Some(parent) = current {
            depth += 1;
            current = self.nodes.get(&parent).and_then(|node| node.parent);
        }
        depth
    }

    fn measure_node(&mut self, id: UiId, available: Vec2) -> Vec2 {
        let Some((layout, content_size, flow_children, absolute_children)) =
            self.nodes.get(&id).map(|node| {
                let flow: Vec<UiId> = node
                    .children
                    .iter()
                    .copied()
                    .filter(|child| {
                        self.nodes
                            .get(child)
                            .is_none_or(|child| child.layout.absolute.is_none())
                    })
                    .collect();
                let absolute: Vec<UiId> = node
                    .children
                    .iter()
                    .copied()
                    .filter(|child| {
                        self.nodes
                            .get(child)
                            .is_some_and(|child| child.layout.absolute.is_some())
                    })
                    .collect();
                (node.layout.clone(), node.content_size, flow, absolute)
            })
        else {
            return Vec2::ZERO;
        };

        let padding = layout.padding;
        let inner_available = Vec2::new(
            (available.x - padding.horizontal_sum()).max(0.0),
            (available.y - padding.vertical_sum()).max(0.0),
        );

        let is_row = layout.is_row();
        let mut inner = content_size;
        if !flow_children.is_empty() {
            let mut main = 0.0f32;
            let mut cross = 0.0f32;
            for (index, child) in flow_children.iter().enumerate() {
                let measured = self.measure_node(*child, inner_available);
                let child_layout = self
                    .nodes
                    .get(child)
                    .map(|node| node.layout.clone())
                    .unwrap_or_default();
                let margin = child_layout.margin;
                let main_size = if is_row {
                    outer_main(
                        child_layout.width,
                        measured.x,
                        inner_available.x,
                        margin.left + margin.right,
                    )
                } else {
                    outer_main(
                        child_layout.height,
                        measured.y,
                        inner_available.y,
                        margin.top + margin.bottom,
                    )
                };
                let cross_size = if is_row {
                    outer_cross(
                        child_layout.height,
                        measured.y,
                        inner_available.y,
                        margin.top + margin.bottom,
                    )
                } else {
                    outer_cross(
                        child_layout.width,
                        measured.x,
                        inner_available.x,
                        margin.left + margin.right,
                    )
                };
                main += main_size;
                if index + 1 < flow_children.len() {
                    main += layout.gap;
                }
                cross = cross.max(cross_size);
            }
            inner = if is_row {
                Vec2::new(main, cross)
            } else {
                Vec2::new(cross, main)
            };
        }

        for child in &absolute_children {
            self.measure_node(*child, inner_available);
        }

        let width = layout
            .width
            .resolve(available.x, inner.x + padding.horizontal_sum());
        let height = layout
            .height
            .resolve(available.y, inner.y + padding.vertical_sum());

        if let Some(node) = self.nodes.get_mut(&id) {
            node.measured = Vec2::new(width, height);
            node.measured
        } else {
            Vec2::new(width, height)
        }
    }

    fn arrange_node(&mut self, id: UiId, rect: Rect, inherited_clip: Rect) {
        let Some((layout, children)) = self
            .nodes
            .get(&id)
            .map(|node| (node.layout.clone(), node.children.clone()))
        else {
            return;
        };

        let clip = inherited_clip.intersect(rect).unwrap_or(Rect::ZERO);
        if let Some(node) = self.nodes.get_mut(&id) {
            node.rect = rect;
            node.clip = clip;
        }

        let content = rect.inset_edges(layout.padding);
        let content_clip = if layout.clip_contents { content } else { clip };
        if layout.clip_contents
            && let Some(node) = self.nodes.get_mut(&id)
        {
            node.clip = content_clip;
        }

        let is_row = layout.is_row();
        let available_main = if is_row { content.w } else { content.h };
        let available_cross = if is_row { content.h } else { content.w };

        // Absolute children are positioned relative to the content box.
        let mut flow_children = Vec::with_capacity(children.len());
        for child in &children {
            let Some(child_node) = self.nodes.get(child) else {
                continue;
            };
            let child_layout = child_node.layout.clone();
            let child_measured = child_node.measured;
            let margin = child_layout.margin;
            if let Some(absolute) = child_layout.absolute {
                let width = if absolute.w > 0.0 {
                    absolute.w
                } else {
                    child_measured.x
                };
                let height = if absolute.h > 0.0 {
                    absolute.h
                } else {
                    child_measured.y
                };
                let child_rect = Rect::new(
                    content.x + absolute.x,
                    content.y + absolute.y,
                    width,
                    height,
                );
                self.arrange_node(*child, child_rect, content_clip);
            } else {
                flow_children.push((*child, child_layout, child_measured, margin));
            }
        }

        if flow_children.is_empty() {
            if let Some(node) = self.nodes.get_mut(&id) {
                node.scroll_max = 0.0;
                node.scroll = 0.0;
                node.scrollbar_rect = Rect::ZERO;
            }
            return;
        }

        let mut sizes = Vec::with_capacity(flow_children.len());
        let mut margins = Vec::with_capacity(flow_children.len());
        let mut fill_total_grow = 0.0f32;
        let mut base_total = 0.0f32;

        for (_, child_layout, child_measured, child_margin) in &flow_children {
            let main_size = match if is_row {
                child_layout.width
            } else {
                child_layout.height
            } {
                Size::Fill => {
                    if layout.scrollable {
                        if is_row {
                            child_measured.x
                        } else {
                            child_measured.y
                        }
                    } else {
                        fill_total_grow += child_layout.grow.max(0.01);
                        0.0
                    }
                }
                Size::Auto => {
                    if is_row {
                        child_measured.x
                    } else {
                        child_measured.y
                    }
                }
                Size::Fixed(value) => value,
                Size::Percent(value) => available_main * value / 100.0,
            };
            sizes.push(main_size);
            margins.push((
                child_margin.left,
                child_margin.top,
                child_margin.right,
                child_margin.bottom,
            ));
            base_total += main_size + child_margin.main_sum(is_row);
        }

        let remaining = (available_main - base_total).max(0.0);
        if !layout.scrollable && fill_total_grow > 0.0 && remaining > 0.0 {
            for ((_, child_layout, _, _), size) in flow_children.iter().zip(sizes.iter_mut()) {
                let fill = if is_row {
                    child_layout.width
                } else {
                    child_layout.height
                };
                if fill.is_fill() {
                    *size = remaining * (child_layout.grow.max(0.01) / fill_total_grow);
                }
            }
        }

        let used_main: f32 = sizes
            .iter()
            .enumerate()
            .map(|(index, size)| {
                if is_row {
                    size + margins[index].0 + margins[index].2
                } else {
                    size + margins[index].1 + margins[index].3
                }
            })
            .sum();
        let total = used_main + layout.gap * flow_children.len().saturating_sub(1) as f32;
        let free = (available_main - total).max(0.0);
        let justify_extra = if layout.scrollable {
            0.0
        } else {
            match layout.justify_content {
                JustifyContent::Start => 0.0,
                JustifyContent::Center => free * 0.5,
                JustifyContent::End => free,
                JustifyContent::SpaceBetween => 0.0,
            }
        };
        let gap_extra = if !layout.scrollable
            && layout.justify_content == JustifyContent::SpaceBetween
            && flow_children.len() > 1
        {
            free / (flow_children.len() - 1) as f32
        } else {
            0.0
        };
        let gap_total = layout.gap + gap_extra;

        let flow_main = total;
        let scroll = if layout.scrollable {
            let max = (flow_main - available_main).max(0.0);
            if let Some(node) = self.nodes.get_mut(&id) {
                node.scroll_max = max;
                node.scroll = node.scroll.clamp(0.0, max);
                node.scroll
            } else {
                0.0
            }
        } else {
            0.0
        };

        let mut cursor = if is_row {
            content.x + justify_extra
        } else {
            content.y + justify_extra
        };

        for (index, (child, child_layout, child_measured, _margin)) in
            flow_children.iter().enumerate()
        {
            let (left, top, right, bottom) = margins[index];
            let size = sizes[index];
            let child_cross = match layout.align_items {
                AlignItems::Stretch => {
                    let cross_size = if is_row {
                        child_layout.height
                    } else {
                        child_layout.width
                    };
                    let margin_cross = if is_row { top + bottom } else { left + right };
                    match cross_size {
                        Size::Fixed(value) => value
                            .max(0.0)
                            .min((available_cross - margin_cross).max(0.0)),
                        _ => (available_cross - margin_cross).max(0.0),
                    }
                }
                _ => {
                    let cross_size = if is_row {
                        child_layout.height
                    } else {
                        child_layout.width
                    };
                    let measured = if is_row {
                        child_measured.y
                    } else {
                        child_measured.x
                    };
                    cross_size.resolve(available_cross, measured)
                }
            };

            let cross_offset = match layout.align_items {
                AlignItems::Start | AlignItems::Stretch => 0.0,
                AlignItems::Center => (available_cross - child_cross) * 0.5,
                AlignItems::End => available_cross - child_cross,
            };

            let child_rect = if is_row {
                Rect::new(
                    cursor + left,
                    content.y + top + cross_offset,
                    size,
                    child_cross,
                )
            } else {
                Rect::new(
                    content.x + left + cross_offset,
                    cursor + top - scroll,
                    child_cross,
                    size,
                )
            };
            self.arrange_node(*child, child_rect, content_clip);
            cursor += size + left + right + gap_total;
        }

        if let Some(node) = self.nodes.get_mut(&id) {
            if layout.scrollable && node.scroll_max > 0.0 && !content.is_empty() {
                let track = content;
                let ratio = (track.h / (track.h + node.scroll_max)).clamp(0.05, 1.0);
                let thumb_height = (track.h * ratio).min(track.h);
                let travel = (track.h - thumb_height).max(0.0);
                let progress = if node.scroll_max > 0.0 {
                    (node.scroll / node.scroll_max).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                node.scrollbar_rect = Rect::new(
                    track.right() - 6.0,
                    track.y + travel * progress,
                    6.0,
                    thumb_height,
                );
            } else {
                node.scrollbar_rect = Rect::ZERO;
            }
        }
    }
}

fn outer_main(size: Size, measured: f32, available: f32, margins: f32) -> f32 {
    match size {
        Size::Auto => measured + margins,
        Size::Fixed(value) => value + margins,
        Size::Percent(value) => available * value / 100.0,
        Size::Fill => 0.0,
    }
}

fn outer_cross(size: Size, measured: f32, available: f32, margins: f32) -> f32 {
    match size {
        Size::Auto | Size::Fill => measured + margins,
        Size::Fixed(value) => value + margins,
        Size::Percent(value) => available * value / 100.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::Layout;

    #[test]
    fn column_layout_stacks_children() {
        let mut tree = UiTree::new();
        let viewport = Rect::new(0.0, 0.0, 400.0, 300.0);
        tree.begin_frame(viewport);
        let root = tree.create(
            UI_ROOT_ID + 1,
            WidgetKind::Container,
            UI_ROOT_ID,
            Layout::column().padding(10.0),
        );
        let _ = root;
        let a = tree.create(
            1,
            WidgetKind::Spacer,
            UI_ROOT_ID + 1,
            Layout::default().size(100.0, 50.0),
        );
        a.content_size = Vec2::new(100.0, 50.0);
        let b = tree.create(
            2,
            WidgetKind::Spacer,
            UI_ROOT_ID + 1,
            Layout::default().size(100.0, 30.0),
        );
        b.content_size = Vec2::new(100.0, 30.0);
        tree.layout();
        let a = tree.node(1).unwrap();
        let b = tree.node(2).unwrap();
        assert_eq!(a.rect, Rect::new(10.0, 10.0, 100.0, 50.0));
        assert_eq!(b.rect, Rect::new(10.0, 60.0, 100.0, 30.0));
    }

    #[test]
    fn scroll_clamps_to_content_height() {
        let mut tree = UiTree::new();
        tree.begin_frame(Rect::new(0.0, 0.0, 200.0, 100.0));
        tree.create(10, WidgetKind::Container, UI_ROOT_ID, Layout::column());
        let scroll = tree.create(
            11,
            WidgetKind::ScrollPanel,
            10,
            Layout::column().height(100.0).scrollable(),
        );
        scroll.content_size = Vec2::new(200.0, 100.0);
        for index in 0..4 {
            let node = tree.create(
                20 + index as UiId,
                WidgetKind::Spacer,
                11,
                Layout::default().size(200.0, 60.0),
            );
            node.content_size = Vec2::new(200.0, 60.0);
        }
        tree.layout();
        let panel = tree.node(11).unwrap();
        assert_eq!(panel.scroll_max, 140.0);
        assert!(panel.scrollbar_rect.h < panel.rect.h);
    }
}
