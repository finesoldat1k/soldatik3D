//! High-level UI frame: immediate API, interaction, layout and painting.
//!
//! ```no_run
//! # use ui::{Layout, Theme, Ui};
//! # use ui::input::UiInput;
//! # fn frame(ui: &mut Ui) {
//! ui.begin_frame_snapshot(UiInput::default(), ui::Vec2::new(1280.0, 720.0));
//! ui.container(Layout::column().padding(12.0), |ui| {
//!     ui.label("Hello");
//!     if ui.button("Play").clicked {
//!         // start the game
//!     }
//! });
//! ui.end_frame();
//! # }
//! ```

use glam::Vec2;
use text::{TextAlign, TextStyle};

use crate::draw::UiDrawList;
use crate::geometry::Rect;
use crate::input::{TouchInputProvider, UiInput, UiPointer};
use crate::layout::{Layout, UI_ROOT_ID, UiId};
use crate::theme::Theme;
use crate::tree::{Node, Response, Sense, UiTree, WidgetKind};

#[derive(Debug, Clone, Copy)]
struct Cursor {
    parent: UiId,
    next: u32,
}

/// Root object of the UI system.
///
/// A single `Ui` lives for the whole game session. It owns the retained tree,
/// the theme, the text system and the current frame's draw list.
pub struct Ui {
    pub(crate) tree: UiTree,
    pub(crate) theme: Theme,
    pub(crate) draw: UiDrawList,
    pub(crate) input: UiInput,
    cursors: Vec<Cursor>,
    /// Pointer currently owning a widget: `(pointer id, node id)`.
    pub(crate) active: Option<(u64, UiId)>,
    /// Pointer currently dragging a scrollbar.
    pub(crate) active_scrollbar: Option<(u64, UiId)>,
    /// Popup rectangles that block widgets below them. Kept for one frame so
    /// both the frame that opens a popup and the next frame are covered.
    pub(crate) blockers_prev: Vec<Rect>,
    pub(crate) blockers_next: Vec<Rect>,
}

impl Default for Ui {
    fn default() -> Self {
        Self::new()
    }
}

impl Ui {
    pub fn new() -> Self {
        Self::with_theme(Theme::dark())
    }

    pub fn with_theme(theme: Theme) -> Self {
        Self {
            tree: UiTree::new(),
            theme,
            draw: UiDrawList::new(1, 1),
            input: UiInput::default(),
            cursors: Vec::new(),
            active: None,
            active_scrollbar: None,
            blockers_prev: Vec::new(),
            blockers_next: Vec::new(),
        }
    }

    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
    }

    pub fn draw(&self) -> &UiDrawList {
        &self.draw
    }

    pub fn draw_mut(&mut self) -> &mut UiDrawList {
        &mut self.draw
    }

    pub fn tree(&self) -> &UiTree {
        &self.tree
    }

    pub fn input(&self) -> &UiInput {
        &self.input
    }

    pub fn viewport(&self) -> Rect {
        self.draw.viewport()
    }

    pub fn width(&self) -> f32 {
        self.draw.width()
    }

    pub fn height(&self) -> f32 {
        self.draw.height()
    }

    /// Begins a frame from any platform input provider.
    pub fn begin_frame<I: TouchInputProvider>(&mut self, input: &I, viewport: Vec2) {
        let snapshot = UiInput::from_provider(input);
        self.begin_frame_snapshot(snapshot, viewport);
    }

    /// Begins a frame from an already-built input snapshot (useful for tests,
    /// remote play and replay systems).
    pub fn begin_frame_snapshot(&mut self, input: UiInput, viewport: Vec2) {
        let width = viewport.x.max(1.0) as u32;
        let height = viewport.y.max(1.0) as u32;
        self.draw.resize(width, height);
        self.draw.clear();
        self.tree.begin_frame(self.draw.viewport());
        self.input = input;

        self.cursors.clear();
        self.cursors.push(Cursor {
            parent: UI_ROOT_ID,
            next: 0,
        });
        self.blockers_prev = std::mem::take(&mut self.blockers_next);

        if let Some((pointer, _)) = self.active
            && self.input.pointer(pointer).is_none()
        {
            self.active = None;
        }
        if let Some((pointer, _)) = self.active_scrollbar
            && self.input.pointer(pointer).is_none()
        {
            self.active_scrollbar = None;
        }
    }

    /// Lays out and paints the frame. Call after all widgets have been built.
    pub fn end_frame(&mut self) {
        self.tree.layout();
        self.refresh_visuals();

        let viewport = self.draw.viewport();
        self.draw.reset_clip();
        let roots = self.tree.roots().to_vec();
        for root in roots {
            self.render_node(root, viewport);
        }
        self.render_popups();
        self.draw.reset_clip();

        // If the owning widget disappeared while the pointer was down, no
        // widget will consume the release. Drop stale ownership here so it
        // cannot block the whole UI on the next frame.
        if let Some((pointer, _)) = self.active
            && self
                .input
                .pointer(pointer)
                .is_none_or(|pointer| !pointer.down)
        {
            self.active = None;
        }
        if let Some((pointer, _)) = self.active_scrollbar
            && self
                .input
                .pointer(pointer)
                .is_none_or(|pointer| !pointer.down)
        {
            self.active_scrollbar = None;
        }

        self.tree.end_frame();
        self.cursors.clear();
    }

    /// Whether the game should suppress camera look / world clicks because the
    /// UI is currently using the pointer.
    pub fn captures_pointer(&self) -> bool {
        self.active.is_some() || self.active_scrollbar.is_some()
    }

    /// Primary pointer position, if any.
    pub fn pointer_position(&self) -> Option<Vec2> {
        self.input.primary().map(|pointer| pointer.position)
    }

    /// True when a pointer is inside `rect` and not blocked by a popup.
    pub fn pointer_inside(&self, rect: Rect) -> bool {
        self.input.primary().is_some_and(|pointer| {
            rect.contains(pointer.position) && !self.position_blocked(pointer.position)
        })
    }

    // ----- layout containers ------------------------------------------------

    /// Starts a transparent layout container.
    pub fn begin_container(&mut self, layout: Layout) -> UiId {
        self.begin_node(WidgetKind::Container, layout)
    }

    /// Transparent container with a closure body.
    pub fn container(&mut self, layout: Layout, body: impl FnOnce(&mut Self)) -> Response {
        self.begin_container(layout);
        body(self);
        self.end_container()
    }

    /// Themed panel with background, border and rounded corners.
    pub fn panel(&mut self, layout: Layout, body: impl FnOnce(&mut Self)) -> Response {
        let background = self.theme.window;
        let border = self.theme.panel_border;
        let radius = self.theme.radius;
        self.begin_styled_container(layout, background, border, radius);
        body(self);
        self.end_container()
    }

    /// Container with explicit decoration.
    pub fn begin_styled_container(
        &mut self,
        layout: Layout,
        background: [f32; 4],
        border: [f32; 4],
        radius: f32,
    ) -> UiId {
        let id = self.begin_container(layout);
        if let Some(node) = self.tree.node_mut(id) {
            node.background = Some(background);
            node.border = Some(border);
            node.radius = radius;
        }
        id
    }

    pub fn end_container(&mut self) -> Response {
        let id = self.end_node();
        self.tree.node(id).map(Node::response).unwrap_or_default()
    }

    // ----- core helpers used by the widget modules --------------------------

    pub(crate) fn begin_node(&mut self, kind: WidgetKind, layout: Layout) -> UiId {
        let id = self.allocate_id(kind, layout.id);
        let parent = self.current_parent();
        self.tree.create(id, kind, parent, layout);
        self.cursors.push(Cursor {
            parent: id,
            next: 0,
        });
        id
    }

    pub(crate) fn end_node(&mut self) -> UiId {
        let cursor = self.cursors.pop().unwrap_or(Cursor {
            parent: UI_ROOT_ID,
            next: 0,
        });
        cursor.parent
    }

    pub(crate) fn current_parent(&self) -> UiId {
        self.cursors
            .last()
            .map(|cursor| cursor.parent)
            .unwrap_or(UI_ROOT_ID)
    }

    pub(crate) fn allocate_id(&mut self, kind: WidgetKind, explicit: Option<UiId>) -> UiId {
        let auto = match self.cursors.last_mut() {
            Some(cursor) => {
                let index = cursor.next;
                cursor.next = cursor.next.wrapping_add(1);
                WidgetKind::auto_id(cursor.parent, index, kind)
            }
            None => WidgetKind::auto_id(UI_ROOT_ID, 0, kind),
        };
        explicit.unwrap_or(auto)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn add_leaf(
        &mut self,
        kind: WidgetKind,
        layout: Layout,
        content_size: Vec2,
        text: String,
        font_size: f32,
        color: [f32; 4],
        align: TextAlign,
    ) -> UiId {
        let id = self.allocate_id(kind, layout.id);
        let parent = self.current_parent();
        let node = self.tree.create(id, kind, parent, layout);
        node.content_size = content_size;
        node.text = text;
        node.font_size = font_size;
        node.text_color = color;
        node.text_align = align;
        id
    }

    /// Computes hover/press/click for a node using last frame's rectangle.
    pub(crate) fn interact_node(&mut self, id: UiId, sense: Sense) -> Response {
        let Some(node) = self.tree.node(id) else {
            return Response {
                id,
                ..Response::default()
            };
        };
        let rect = node.rect;
        let mut response = Response {
            id,
            ..Response::default()
        };
        if rect.is_empty() || !sense.click {
            return response;
        }

        let primary = self.input.primary().copied();
        let active_here = self.active.filter(|&(_, active_id)| active_id == id);

        // Press acquisition.
        if self.active.is_none() && sense.click {
            let pressed = self.input.pointers.iter().find(|pointer| {
                pointer.pressed
                    && rect.contains(pointer.position)
                    && !self.position_blocked(pointer.position)
            });
            if let Some(pointer) = pressed {
                self.active = Some((pointer.id, id));
                response.pressed = true;
                response.hovered = true;
            }
        }

        // Held/released state.
        if let Some((pointer_id, _)) = self.active
            && self.active.is_some_and(|(_, active_id)| active_id == id)
            && let Some(pointer) = self.input.pointer(pointer_id)
        {
            response.hovered = rect.contains(pointer.position) || response.hovered;
            response.active = true;
            response.pressed = pointer.down;
            if sense.drag {
                response.dragged = pointer.down && pointer.delta() != Vec2::ZERO;
                response.drag_delta = pointer.delta();
            }
            if pointer.released {
                response.released = true;
                response.clicked = rect.contains(pointer.position);
                self.active = None;
            }
        }

        // Hover without an active interaction.
        if self.active.is_none()
            && !response.hovered
            && let Some(pointer) = primary
            && rect.contains(pointer.position)
            && !self.position_blocked(pointer.position)
        {
            response.hovered = true;
        }

        // A quick tap can press and release inside a single frame; `active_here`
        // still describes this node on the frame after the press, so ensure a
        // release is not lost if the active marker was cleared above.
        if let Some((_, active_id)) = active_here
            && active_id == id
            && !response.active
            && let Some(pointer) = primary
            && pointer.released
        {
            response.released = true;
            response.clicked = rect.contains(pointer.position);
        }

        if let Some(node) = self.tree.node_mut(id) {
            node.hovered = response.hovered;
            node.pressed = response.pressed;
            node.active = response.active;
            node.clicked = response.clicked;
            node.released = response.released;
            node.dragged = response.dragged;
            node.changed = response.changed;
            node.drag_delta = response.drag_delta;
        }
        response
    }

    pub(crate) fn register_blocker(&mut self, rect: Rect) {
        if !rect.is_empty() {
            self.blockers_next.push(rect);
        }
    }

    pub(crate) fn position_blocked(&self, position: Vec2) -> bool {
        self.blockers_prev
            .iter()
            .chain(self.blockers_next.iter())
            .any(|rect| rect.contains(position))
    }

    pub(crate) fn primary_pointer(&self) -> Option<UiPointer> {
        self.input.primary().copied()
    }

    /// Pointer that currently owns a widget, falling back to the primary
    /// pointer. Sliders use it to keep tracking the same finger while dragging.
    pub(crate) fn interaction_pointer(&self) -> Option<UiPointer> {
        if let Some((pointer_id, _)) = self.active
            && let Some(pointer) = self.input.pointer(pointer_id)
        {
            return Some(*pointer);
        }
        self.primary_pointer()
    }

    /// Reserves a pointer for a non-node control such as the joystick.
    /// Returns `false` when another widget or control already owns a pointer.
    pub(crate) fn claim_pointer(&mut self, pointer: u64, owner: UiId) -> bool {
        if self.active.is_none() {
            self.active = Some((pointer, owner));
            true
        } else {
            false
        }
    }

    pub(crate) fn release_pointer(&mut self, pointer: u64, owner: UiId) {
        if self.active == Some((pointer, owner)) {
            self.active = None;
        }
    }

    // ----- raw drawing facade ----------------------------------------------

    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
        self.draw.rect(x, y, w, h, color);
    }

    pub fn rect_rounded(&mut self, x: f32, y: f32, w: f32, h: f32, radius: f32, color: [f32; 4]) {
        self.draw.rect_rounded(x, y, w, h, radius, color);
    }

    pub fn rect_outline(&mut self, rect: Rect, width: f32, color: [f32; 4]) {
        self.draw.rect_outline(rect, width, color);
    }

    pub fn text(&mut self, text: &str, x: f32, y: f32, size: f32, color: [f32; 4]) -> Vec2 {
        self.draw.text(text, x, y, size, color)
    }

    pub fn text_styled(&mut self, text: &str, style: &TextStyle, origin: Vec2) -> Vec2 {
        self.draw.text_styled(text, style, origin)
    }

    pub fn text_width(&self, text: &str, size: f32) -> f32 {
        self.draw.text_width(text, size)
    }

    pub fn measure_text(&self, text: &str, size: f32) -> Vec2 {
        self.draw.measure(text, size)
    }

    pub fn set_clip(&mut self, rect: Rect) {
        self.draw.set_clip(rect);
    }

    pub fn reset_clip(&mut self) {
        self.draw.reset_clip();
    }

    pub fn crosshair(&mut self) {
        self.draw.crosshair();
    }

    // ----- painting ---------------------------------------------------------

    #[allow(clippy::too_many_arguments)]
    fn paint_text(
        &mut self,
        text: &str,
        size: f32,
        color: [f32; 4],
        align: TextAlign,
        rect: Rect,
        wrap: Option<f32>,
        vcenter: bool,
    ) {
        if text.is_empty() || rect.is_empty() {
            return;
        }
        let mut style = TextStyle::new(size).with_color(color).align(align);
        if let Some(width) = wrap {
            style = style.wrap(width.max(1.0));
        }
        let measured = self.draw.measure_styled(text, &style);
        let x = match (wrap, align) {
            (Some(width), TextAlign::Center) => rect.x + (rect.w - width).max(0.0) * 0.5,
            (Some(_), _) => rect.x,
            (None, TextAlign::Left) => rect.x,
            (None, TextAlign::Center) => rect.x + (rect.w - measured.x).max(0.0) * 0.5,
            (None, TextAlign::Right) => rect.right() - measured.x,
        };
        let y = if vcenter {
            rect.y + (rect.h - measured.y).max(0.0) * 0.5
        } else {
            rect.y
        };
        self.draw.text_styled(text, &style, Vec2::new(x, y));
    }

    fn paint_surface(
        &mut self,
        rect: Rect,
        background: Option<[f32; 4]>,
        border: Option<[f32; 4]>,
        radius: f32,
    ) {
        if let Some(background) = background {
            self.draw
                .rect_rounded(rect.x, rect.y, rect.w, rect.h, radius, background);
        }
        if let Some(border) = border {
            self.draw.rect_outline(rect, 1.0, border);
        }
    }

    fn widget_color(&self, node: &Node) -> Option<[f32; 4]> {
        if node.active || node.pressed {
            node.pressed_color.or(node.background)
        } else if node.hovered {
            node.hover_color.or(node.background)
        } else {
            node.background
        }
    }

    fn render_node(&mut self, id: UiId, inherited_clip: Rect) {
        let Some(node) = self.tree.node(id).cloned() else {
            return;
        };
        if !node.visible {
            return;
        }
        let own_clip = inherited_clip.intersect(node.rect).unwrap_or(Rect::ZERO);
        self.draw.set_clip(own_clip);

        let content = node.rect.inset_edges(node.layout.padding);
        match node.kind {
            WidgetKind::Container | WidgetKind::ScrollPanel => {
                self.paint_surface(node.rect, node.background, node.border, node.radius);
            }
            WidgetKind::Label => {
                self.paint_text(
                    &node.text,
                    node.font_size,
                    node.text_color,
                    node.text_align,
                    content,
                    node.text_wrap,
                    true,
                );
            }
            WidgetKind::Button => {
                let background = self.widget_color(&node);
                self.paint_surface(node.rect, background, node.border, node.radius);
                let text_rect = content;
                self.paint_text(
                    &node.text,
                    node.font_size,
                    node.text_color,
                    TextAlign::Center,
                    text_rect,
                    None,
                    true,
                );
            }
            WidgetKind::Toggle => {
                self.paint_toggle(&node, content);
            }
            WidgetKind::Slider => {
                self.paint_slider(&node, content);
            }
            WidgetKind::Progress => {
                self.paint_progress(&node);
            }
            WidgetKind::Dropdown => {
                let background = self.widget_color(&node);
                self.paint_surface(node.rect, background, node.border, node.radius);
                let arrow_space = node.font_size + 12.0;
                let text_rect = Rect::new(
                    content.x,
                    content.y,
                    (content.w - arrow_space).max(0.0),
                    content.h,
                );
                self.paint_text(
                    &node.text,
                    node.font_size,
                    node.text_color,
                    TextAlign::Left,
                    text_rect,
                    None,
                    true,
                );
                let arrow_rect = Rect::new(
                    content.right() - arrow_space,
                    content.y,
                    arrow_space,
                    content.h,
                );
                self.paint_text(
                    "v",
                    node.font_size * 0.85,
                    self.theme.text_muted,
                    TextAlign::Center,
                    arrow_rect,
                    None,
                    true,
                );
            }
            WidgetKind::Separator => {
                let y = node.rect.center().y;
                self.draw
                    .line_h(node.rect.x, y, node.rect.w, 1.0, node.text_color);
            }
            WidgetKind::Spacer => {}
        }

        let child_clip = if node.layout.clip_contents {
            content.intersect(own_clip).unwrap_or(Rect::ZERO)
        } else {
            inherited_clip
        };
        self.draw.set_clip(child_clip);
        for child in &node.children {
            self.render_node(*child, child_clip);
        }

        self.draw.set_clip(own_clip);
        if node.kind == WidgetKind::ScrollPanel {
            self.paint_scrollbar(&node);
        }
    }

    fn paint_scrollbar(&mut self, node: &Node) {
        let bar = node.scrollbar_rect;
        if bar.is_empty() {
            return;
        }
        let color = [
            self.theme.text_muted[0],
            self.theme.text_muted[1],
            self.theme.text_muted[2],
            0.55,
        ];
        self.draw
            .rect_rounded(bar.x, bar.y, bar.w, bar.h, bar.w * 0.5, color);
    }

    fn paint_toggle(&mut self, node: &Node, content: Rect) {
        let track_height = (content.h * 0.55).clamp(22.0, 32.0);
        let track_width = (track_height * 1.85).clamp(40.0, 64.0);
        let track = Rect::new(
            content.right() - track_width,
            content.y + (content.h - track_height) * 0.5,
            track_width,
            track_height,
        );
        let label_rect = Rect::new(
            content.x,
            content.y,
            (content.w - track_width - self.theme.spacing).max(0.0),
            content.h,
        );
        self.paint_text(
            &node.text,
            node.font_size,
            node.text_color,
            TextAlign::Left,
            label_rect,
            None,
            true,
        );

        let off = node.background.unwrap_or(self.theme.toggle_off);
        let on = node.fill_color.unwrap_or(self.theme.toggle_on);
        let track_color = if node.toggled { on } else { off };
        self.draw.rect_rounded(
            track.x,
            track.y,
            track.w,
            track.h,
            track.h * 0.5,
            track_color,
        );

        let knob_radius = (track.h * 0.5 - 3.0).max(7.0);
        let knob_x = if node.toggled {
            track.right() - knob_radius - 3.0
        } else {
            track.x + knob_radius + 3.0
        };
        let knob_y = track.center().y;
        self.draw.rect_rounded(
            knob_x - knob_radius,
            knob_y - knob_radius,
            knob_radius * 2.0,
            knob_radius * 2.0,
            knob_radius,
            self.theme.knob,
        );
    }

    fn paint_slider(&mut self, node: &Node, content: Rect) {
        let has_label = !node.text.is_empty();
        let label_height = if has_label { node.font_size + 4.0 } else { 0.0 };
        if has_label {
            self.paint_text(
                &node.text,
                node.font_size,
                node.text_color,
                TextAlign::Left,
                Rect::new(content.x, content.y, content.w, label_height),
                None,
                true,
            );
        }
        if node.show_value {
            let value = format!("{:.2}", node.value);
            self.paint_text(
                &value,
                self.theme.small_font_size,
                self.theme.text_muted,
                TextAlign::Right,
                Rect::new(content.x, content.y, content.w, label_height),
                None,
                true,
            );
        }

        let track_height = 6.0;
        let track_y = if has_label {
            content.y + label_height + (content.h - label_height) * 0.5 - track_height * 0.5
        } else {
            content.center().y - track_height * 0.5
        };
        let track = Rect::new(content.x, track_y, content.w, track_height);
        self.draw.rect_rounded(
            track.x,
            track.y,
            track.w,
            track.h,
            track.h * 0.5,
            node.background.unwrap_or(self.theme.track),
        );

        let range = (node.max_value - node.min).max(f32::EPSILON);
        let t = ((node.value - node.min) / range).clamp(0.0, 1.0);
        let knob_radius = 10.0;
        let knob_x = track.x + t * track.w;
        let fill_width = (knob_x - track.x).max(0.0);
        if fill_width > 0.0 {
            self.draw.rect_rounded(
                track.x,
                track.y,
                fill_width,
                track.h,
                track.h * 0.5,
                node.fill_color.unwrap_or(self.theme.track_fill),
            );
        }
        self.draw.rect_rounded(
            knob_x - knob_radius,
            track.center().y - knob_radius,
            knob_radius * 2.0,
            knob_radius * 2.0,
            knob_radius,
            self.theme.knob,
        );
    }

    fn paint_progress(&mut self, node: &Node) {
        let rect = node.rect;
        self.draw.rect_rounded(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            rect.h * 0.5,
            node.background.unwrap_or(self.theme.track),
        );
        let range = (node.max_value - node.min).max(f32::EPSILON);
        let t = ((node.value - node.min) / range).clamp(0.0, 1.0);
        if t > 0.0 {
            let width = (rect.w * t).max(rect.h);
            self.draw.rect_rounded(
                rect.x,
                rect.y,
                width.min(rect.w),
                rect.h,
                rect.h * 0.5,
                node.fill_color.unwrap_or(self.theme.track_fill),
            );
        }
        if !node.text.is_empty() {
            self.paint_text(
                &node.text,
                self.theme.small_font_size,
                self.theme.text,
                TextAlign::Center,
                rect,
                None,
                true,
            );
        }
    }

    fn render_popups(&mut self) {
        let dropdowns: Vec<UiId> = self
            .tree
            .nodes_in_order()
            .filter(|node| {
                node.kind == WidgetKind::Dropdown && node.open && !node.popup_rect.is_empty()
            })
            .map(|node| node.id)
            .collect();

        for id in dropdowns {
            let Some(node) = self.tree.node(id).cloned() else {
                continue;
            };
            let popup = node.popup_rect;
            self.draw.reset_clip();
            self.draw.rect(
                popup.x + 2.0,
                popup.y + 3.0,
                popup.w,
                popup.h,
                self.theme.shadow,
            );
            self.paint_surface(
                popup,
                Some(self.theme.panel),
                Some(self.theme.panel_border),
                self.theme.radius,
            );
            self.register_blocker(popup);

            let count = node.items.len().max(1);
            let item_height = popup.h / count as f32;
            for (index, item) in node.items.iter().enumerate() {
                let item_rect = Rect::new(
                    popup.x + 4.0,
                    popup.y + index as f32 * item_height + 2.0,
                    (popup.w - 8.0).max(0.0),
                    (item_height - 4.0).max(0.0),
                );
                let selected = index == node.selected;
                let hovered = node.hovered_item == Some(index);
                if selected {
                    self.draw.rect_rounded(
                        item_rect.x,
                        item_rect.y,
                        item_rect.w,
                        item_rect.h,
                        self.theme.radius * 0.6,
                        self.theme.accent,
                    );
                } else if hovered {
                    self.draw.rect_rounded(
                        item_rect.x,
                        item_rect.y,
                        item_rect.w,
                        item_rect.h,
                        self.theme.radius * 0.6,
                        self.theme.button_hovered,
                    );
                }
                let color = if selected {
                    self.theme.text
                } else {
                    node.text_color
                };
                self.paint_text(
                    item,
                    node.font_size,
                    color,
                    TextAlign::Left,
                    item_rect.inset_edges(crate::geometry::EdgeInsets::symmetric(10.0, 0.0)),
                    None,
                    true,
                );
            }
        }
    }

    /// Recomputes hover/pressed flags against the freshly laid-out rectangles
    /// so the visible state matches the frame instead of lagging by one frame.
    fn refresh_visuals(&mut self) {
        let primary = self.input.primary().copied();
        let blockers = std::mem::take(&mut self.blockers_next);
        let active = self.active;
        let mut updates = Vec::new();
        for node in self.tree.nodes_in_order() {
            if !node.visible || node.kind == WidgetKind::Container {
                continue;
            }
            let hovered = primary.is_some_and(|pointer| {
                node.rect.contains(pointer.position)
                    && !blockers
                        .iter()
                        .chain(self.blockers_prev.iter())
                        .any(|rect| rect.contains(pointer.position))
                    && active.is_none_or(|(_, id)| id == node.id)
            });
            let pressed = active.is_some_and(|(pointer, id)| {
                id == node.id
                    && primary.is_some_and(|pointer_state| {
                        pointer_state.id == pointer && pointer_state.down
                    })
            });
            updates.push((node.id, hovered, pressed));
        }
        for (id, hovered, pressed) in updates {
            if let Some(node) = self.tree.node_mut(id) {
                node.hovered = hovered;
                node.pressed = pressed;
                node.active = active.is_some_and(|(_, active_id)| active_id == id);
            }
        }
        self.blockers_next = blockers;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::PointerKind;
    use crate::widgets::{ButtonStyle, TouchCameraState, TouchCameraStyle};

    fn pointer(position: Vec2, down: bool, pressed: bool, released: bool) -> UiInput {
        UiInput {
            pointers: vec![UiPointer {
                id: 0,
                kind: PointerKind::Mouse,
                position,
                prev_position: position,
                down,
                pressed,
                released,
            }],
            ..UiInput::default()
        }
    }

    fn frame_button(ui: &mut Ui, rect: Rect) -> Response {
        // Absolute placement only applies to children, so wrap the button in
        // a full-screen HUD layer as a game would.
        ui.begin_container(Layout::default().fill());
        let style = ButtonStyle::from_theme(ui.theme());
        let response = ui.button_with(Layout::default().absolute(rect), "OK", style);
        ui.end_container();
        response
    }

    #[test]
    fn button_clicks_on_release() {
        let viewport = Vec2::new(240.0, 160.0);
        let rect = Rect::new(20.0, 20.0, 100.0, 40.0);
        let center = rect.center();
        let mut ui = Ui::new();

        ui.begin_frame_snapshot(UiInput::default(), viewport);
        frame_button(&mut ui, rect);
        ui.end_frame();

        ui.begin_frame_snapshot(pointer(center, true, true, false), viewport);
        let pressed = frame_button(&mut ui, rect);
        ui.end_frame();
        assert!(pressed.pressed);

        ui.begin_frame_snapshot(pointer(center, false, false, true), viewport);
        let released = frame_button(&mut ui, rect);
        ui.end_frame();
        assert!(released.clicked);
    }

    #[test]
    fn touch_camera_does_not_steal_a_claimed_button() {
        let viewport = Vec2::new(400.0, 300.0);
        let rect = Rect::new(300.0, 200.0, 80.0, 80.0);
        let center = rect.center();
        let mut ui = Ui::new();
        let mut camera = TouchCameraState::default();
        let style = TouchCameraStyle::from_theme(ui.theme());

        ui.begin_frame_snapshot(UiInput::default(), viewport);
        frame_button(&mut ui, rect);
        ui.touch_camera(&mut camera, &style);
        ui.end_frame();

        ui.begin_frame_snapshot(pointer(center, true, true, false), viewport);
        frame_button(&mut ui, rect);
        ui.touch_camera(&mut camera, &style);
        ui.end_frame();
        assert!(!camera.dragging);

        ui.begin_frame_snapshot(pointer(center, false, false, true), viewport);
        let released = frame_button(&mut ui, rect);
        ui.touch_camera(&mut camera, &style);
        ui.end_frame();
        assert!(released.clicked);
        assert!(!camera.dragging);
    }
}
