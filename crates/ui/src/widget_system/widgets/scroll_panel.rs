//! Scroll panel widget.

use std::any::Any;
use glam::Vec2;

use crate::widget_system::core::*;
use crate::widget_system::tree::*;

/// Scroll panel component for state.
#[derive(Debug, Clone, Default)]
pub struct ScrollPanelComponent {
    pub scroll: f32,
    pub max_scroll: f32,
    pub dragging_scrollbar: bool,
    pub content_height: f32,
}

impl Component for ScrollPanelComponent {
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

/// Scroll panel widget builder.
pub struct ScrollPanel {
    pub id: WidgetId,
    pub layout: LayoutProps,
    pub style: ScrollPanelStyle,
    pub children: Vec<Box<dyn Widget>>,
}

#[derive(Debug, Clone)]
pub struct ScrollPanelStyle {
    pub background: [f32; 4],
    pub border: [f32; 4],
    pub radius: f32,
    pub scrollbar_width: f32,
    pub scrollbar_color: [f32; 4],
    pub scrollbar_hovered: [f32; 4],
    pub scrollbar_track: [f32; 4],
}

impl Default for ScrollPanelStyle {
    fn default() -> Self {
        Self {
            background: [0.05, 0.06, 0.08, 0.95],
            border: [0.15, 0.18, 0.22, 1.0],
            radius: 8.0,
            scrollbar_width: 8.0,
            scrollbar_color: [0.4, 0.5, 0.6, 0.8],
            scrollbar_hovered: [0.5, 0.6, 0.7, 0.9],
            scrollbar_track: [0.1, 0.12, 0.15, 0.5],
        }
    }
}

impl ScrollPanel {
    pub fn new() -> Self {
        Self { id: WidgetId::new(), layout: LayoutProps::default(),
            style: ScrollPanelStyle::default(), children: Vec::new() }
    }

    pub fn id(mut self, id: WidgetId) -> Self { self.id = id; self }
    pub fn layout(mut self, layout: LayoutProps) -> Self { self.layout = layout; self }
    pub fn style(mut self, style: ScrollPanelStyle) -> Self { self.style = style; self }
    pub fn child(mut self, child: impl Into<Box<dyn Widget>>) -> Self { self.children.push(child.into()); self }
    pub fn children(mut self, children: Vec<Box<dyn Widget>>) -> Self { self.children.extend(children); self }

    pub fn build(self) -> Box<dyn Widget> {
        Box::new(ScrollPanelWidget {
            id: self.id, layout: self.layout, style: self.style, children: self.children,
        })
    }
}

struct ScrollPanelWidget {
    id: WidgetId, layout: LayoutProps, style: ScrollPanelStyle, children: Vec<Box<dyn Widget>>,
}

impl Widget for ScrollPanelWidget {
    fn id(&self) -> WidgetId { self.id }

    fn on_mount(&mut self, node: &mut WidgetNode) {
        for child in self.children.drain(..) {
            let child_id = child.id();
            let child_node = WidgetNode::new(child_id, child);
            node.add_child(child_node);
        }
    }

    fn update(&mut self, node: &mut WidgetNode, ctx: &mut UpdateContext) {
        let mut scroll = node.components.get_mut::<ScrollPanelComponent>()
            .cloned()
            .unwrap_or_else(ScrollPanelComponent::default);

        if scroll.dragging_scrollbar {
            if let Some(pointer) = ctx.primary_pointer {
                if pointer.pressed {
                    let track_h = node.computed.rect.h - 8.0;
                    let rel_y = (pointer.position.y - node.computed.rect.y - 4.0) / track_h;
                    scroll.scroll = (rel_y * scroll.max_scroll).clamp(0.0, scroll.max_scroll);
                } else { scroll.dragging_scrollbar = false; }
            }
        }
        node.components.add(scroll);
    }

    fn compute_layout(&mut self, node: &mut WidgetNode, constraints: LayoutConstraints) -> Vec2 {
        let mut total_h: f32 = 0.0; let mut max_w: f32 = 0.0;

        for child_rc in &node.children {
            let child = child_rc.borrow();
            total_h += child.computed.rect.h;
            max_w = max_w.max(child.computed.rect.w);
        }

        let w = (max_w + node.layout.padding.x + node.layout.padding.w)
            .max(node.layout.min_size.x).clamp(constraints.min_width, constraints.max_width);
        let h = (node.layout.min_size.y.max(200.0)).clamp(constraints.min_height, constraints.max_height);

        let mut scroll = node.components.get_mut::<ScrollPanelComponent>()
            .cloned()
            .unwrap_or_else(ScrollPanelComponent::default);
        scroll.content_height = total_h;
        scroll.max_scroll = (total_h - h).max(0.0);
        scroll.scroll = scroll.scroll.clamp(0.0, scroll.max_scroll);
        node.components.add(scroll);

        Vec2::new(w, h)
    }

    fn render(&mut self, node: &WidgetNode, draw: &mut dyn DrawList) {
        let rect = node.computed.rect;
        let default_scroll = ScrollPanelComponent::default();
        let scroll = node.components.get::<ScrollPanelComponent>().unwrap_or(&default_scroll);

        let max_scroll = scroll.max_scroll;
        let content_height = scroll.content_height;
        let scroll_pos = scroll.scroll;

        draw.rect_rounded(rect.x, rect.y, rect.w, rect.h, self.style.radius, self.style.background);
        draw.rect_rounded(rect.x, rect.y, rect.w, rect.h, self.style.radius, self.style.border);

        if max_scroll > 0.0 {
            let bar_w = self.style.scrollbar_width;
            let bar_x = rect.x + rect.w - bar_w - 4.0;
            let track_h = rect.h - 8.0;
            let bar_h = (rect.h / content_height * rect.h).max(30.0).min(track_h);
            let bar_y = rect.y + 4.0 + scroll_pos / max_scroll * (track_h - bar_h);

            draw.rect_rounded(bar_x, rect.y + 4.0, bar_w, track_h, bar_w * 0.5, self.style.scrollbar_track);
            draw.rect_rounded(bar_x, bar_y, bar_w, bar_h, bar_w * 0.5, self.style.scrollbar_color);
        }
    }

    fn handle_event(&mut self, node: &mut WidgetNode, event: &InputEvent) -> EventPropagation {
        let mut scroll = node.components.get_mut::<ScrollPanelComponent>()
            .cloned()
            .unwrap_or_else(ScrollPanelComponent::default);

        match event.event {
            InteractionEvent::Press => {
                let rect = node.computed.rect;
                let bar_w = self.style.scrollbar_width;
                let bar_x = rect.x + rect.w - bar_w - 4.0;
                let track_h = rect.h - 8.0;
                let bar_h = (rect.h / scroll.content_height * rect.h).max(30.0).min(track_h);
                let bar_y = rect.y + 4.0 + scroll.scroll / scroll.max_scroll * (track_h - bar_h);
                let thumb_rect = Rect::new(bar_x, bar_y, bar_w, bar_h);

                if thumb_rect.contains(event.pointer.position) {
                    scroll.dragging_scrollbar = true;
                    return EventPropagation::Handled;
                }
                EventPropagation::Continue
            }
            InteractionEvent::Release => { scroll.dragging_scrollbar = false; EventPropagation::Continue }
            _ => EventPropagation::Continue,
        }
    }
}