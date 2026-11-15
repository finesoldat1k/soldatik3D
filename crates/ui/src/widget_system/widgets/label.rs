//! Label/Text widget.

use std::any::Any;
use glam::Vec2;

use crate::widget_system::core::*;
use crate::widget_system::tree::*;

/// Label widget for displaying text.
pub struct Label {
    pub id: WidgetId,
    pub text: String,
    pub scale: f32,
    pub color: [f32; 4],
    pub layout: LayoutProps,
    pub alignment: TextAlignment,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextAlignment { Left, Center, Right }

impl Label {
    pub fn new(text: impl Into<String>) -> Self {
        Self { id: WidgetId::new(), text: text.into(), scale: 1.0,
            color: [0.95, 0.95, 0.98, 1.0], layout: LayoutProps::default(),
            alignment: TextAlignment::Left }
    }

    pub fn id(mut self, id: WidgetId) -> Self { self.id = id; self }
    pub fn text(mut self, text: impl Into<String>) -> Self { self.text = text.into(); self }
    pub fn scale(mut self, scale: f32) -> Self { self.scale = scale; self }
    pub fn color(mut self, color: [f32; 4]) -> Self { self.color = color; self }
    pub fn layout(mut self, layout: LayoutProps) -> Self { self.layout = layout; self }
    pub fn alignment(mut self, alignment: TextAlignment) -> Self { self.alignment = alignment; self }

    pub fn build(self) -> Box<dyn Widget> {
        Box::new(LabelWidget {
            id: self.id, text: self.text, scale: self.scale, color: self.color,
            layout: self.layout, alignment: self.alignment,
        })
    }
}

struct LabelWidget {
    id: WidgetId, text: String, scale: f32, color: [f32; 4],
    layout: LayoutProps, alignment: TextAlignment,
}

impl Widget for LabelWidget {
    fn id(&self) -> WidgetId { self.id }
    fn update(&mut self, _node: &mut WidgetNode, _ctx: &mut UpdateContext) {}

    fn compute_layout(&mut self, node: &mut WidgetNode, constraints: LayoutConstraints) -> Vec2 {
        let text_w = text_width_static(&self.text, self.scale);
        let text_h = 7.0 * self.scale;
        let w = (text_w + node.layout.padding.x + node.layout.padding.w)
            .max(node.layout.min_size.x).clamp(constraints.min_width, constraints.max_width);
        let h = (text_h + node.layout.padding.y + node.layout.padding.h)
            .max(node.layout.min_size.y).clamp(constraints.min_height, constraints.max_height);
        Vec2::new(w, h)
    }

    fn render(&mut self, node: &WidgetNode, draw: &mut dyn DrawList) {
        let rect = node.computed.content_rect;
        let text_w = text_width_static(&self.text, self.scale);
        let text_h = 7.0 * self.scale;
        let x = match self.alignment {
            TextAlignment::Left => rect.x,
            TextAlignment::Center => rect.x + (rect.w - text_w) * 0.5,
            TextAlignment::Right => rect.x + rect.w - text_w,
        };
        let y = rect.y + (rect.h - text_h) * 0.5;
        draw.text(&self.text, x, y, self.scale, self.color);
    }

    fn handle_event(&mut self, _node: &mut WidgetNode, _event: &InputEvent) -> EventPropagation {
        EventPropagation::Continue
    }
}

fn text_width_static(text: &str, scale: f32) -> f32 {
    text.chars().filter(|c| (32..=126).contains(&(*c as u32)))
        .count() as f32 * 7.0 * scale
}