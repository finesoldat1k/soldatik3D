//! Container/Flex container widget.

use std::any::Any;
use glam::Vec2;

use crate::widget_system::core::*;
use crate::widget_system::tree::*;

/// Container widget that uses flexbox layout for children.
pub struct Container {
    pub id: WidgetId,
    pub layout: LayoutProps,
    pub style: ContainerStyle,
    pub children: Vec<Box<dyn Widget>>,
}

#[derive(Debug, Clone)]
pub struct ContainerStyle {
    pub background: [f32; 4],
    pub border: [f32; 4],
    pub radius: f32,
    pub shadow: bool,
}

impl Default for ContainerStyle {
    fn default() -> Self {
        Self { background: [0.05, 0.06, 0.08, 0.95],
            border: [0.15, 0.18, 0.22, 1.0], radius: 12.0, shadow: true }
    }
}

impl Container {
    pub fn new() -> Self {
        Self { id: WidgetId::new(), layout: LayoutProps::default(),
            style: ContainerStyle::default(), children: Vec::new() }
    }

    pub fn id(mut self, id: WidgetId) -> Self { self.id = id; self }
    pub fn layout(mut self, layout: LayoutProps) -> Self { self.layout = layout; self }
    pub fn style(mut self, style: ContainerStyle) -> Self { self.style = style; self }
    pub fn child(mut self, child: impl Into<Box<dyn Widget>>) -> Self { self.children.push(child.into()); self }
    pub fn children(mut self, children: Vec<Box<dyn Widget>>) -> Self { self.children.extend(children); self }

    pub fn build(self) -> Box<dyn Widget> {
        Box::new(ContainerWidget {
            id: self.id, layout: self.layout, style: self.style, children: self.children,
        })
    }
}

struct ContainerWidget {
    id: WidgetId, layout: LayoutProps, style: ContainerStyle, children: Vec<Box<dyn Widget>>,
}

impl Widget for ContainerWidget {
    fn id(&self) -> WidgetId { self.id }

    fn on_mount(&mut self, node: &mut WidgetNode) {
        for child in self.children.drain(..) {
            let child_id = child.id();
            let child_node = WidgetNode::new(child_id, child);
            node.add_child(child_node);
        }
    }

    fn update(&mut self, _node: &mut WidgetNode, _ctx: &mut UpdateContext) {}

    fn compute_layout(&mut self, node: &mut WidgetNode, constraints: LayoutConstraints) -> Vec2 {
        let w = node.layout.min_size.x.clamp(constraints.min_width, constraints.max_width);
        let h = node.layout.min_size.y.clamp(constraints.min_height, constraints.max_height);
        Vec2::new(w.max(1.0), h.max(1.0))
    }

    fn render(&mut self, node: &WidgetNode, draw: &mut dyn DrawList) {
        let rect = node.computed.rect;
        if self.style.shadow {
            draw.rect_rounded(rect.x + 3.0, rect.y + 3.0, rect.w, rect.h, self.style.radius, [0.0, 0.0, 0.0, 0.3]);
        }
        draw.rect_rounded(rect.x, rect.y, rect.w, rect.h, self.style.radius, self.style.background);
        draw.rect_rounded(rect.x, rect.y, rect.w, rect.h, self.style.radius, self.style.border);
    }

    fn handle_event(&mut self, _node: &mut WidgetNode, _event: &InputEvent) -> EventPropagation {
        EventPropagation::Continue
    }
}