//! Button widget implementation.

use std::any::Any;
use glam::Vec2;

use crate::widget_system::core::*;
use crate::widget_system::tree::*;

/// Button component for clickable widgets.
#[derive(Debug, Clone, Default)]
pub struct ButtonComponent {
    pub pressed: bool,
    pub hovered: bool,
    pub clicked: bool,
    pub toggle: bool,
    pub toggled: bool,
}

impl Component for ButtonComponent {
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

/// Button widget builder.
pub struct Button {
    pub id: WidgetId,
    pub label: String,
    pub label_scale: f32,
    pub on_click: Option<Box<dyn FnMut(&mut WidgetNode) + Send + Sync>>,
    pub layout: LayoutProps,
    pub style: ButtonStyle,
}

#[derive(Debug, Clone)]
pub struct ButtonStyle {
    pub normal_bg: [f32; 4],
    pub hovered_bg: [f32; 4],
    pub pressed_bg: [f32; 4],
    pub border: [f32; 4],
    pub border_focused: [f32; 4],
    pub text_color: [f32; 4],
    pub radius: f32,
    pub shadow_color: [f32; 4],
    pub toggle: bool,
}

impl Default for ButtonStyle {
    fn default() -> Self {
        Self {
            normal_bg: [0.08, 0.09, 0.11, 0.95],
            hovered_bg: [0.12, 0.14, 0.17, 0.95],
            pressed_bg: [0.16, 0.18, 0.22, 0.95],
            border: [0.2, 0.22, 0.25, 1.0],
            border_focused: [1.0, 0.6, 0.1, 1.0],
            text_color: [0.95, 0.95, 0.98, 1.0],
            radius: 8.0,
            shadow_color: [0.0, 0.0, 0.0, 0.4],
            toggle: false,
        }
    }
}

impl Button {
    pub fn new(label: impl Into<String>) -> Self {
        Self { id: WidgetId::new(), label: label.into(), label_scale: 1.0,
            on_click: None, layout: LayoutProps::default(), style: ButtonStyle::default() }
    }

    pub fn id(mut self, id: WidgetId) -> Self { self.id = id; self }
    pub fn label(mut self, label: impl Into<String>) -> Self { self.label = label.into(); self }
    pub fn label_scale(mut self, scale: f32) -> Self { self.label_scale = scale; self }
    pub fn on_click<F>(mut self, f: F) -> Self
    where F: FnMut(&mut WidgetNode) + Send + Sync + 'static
    { self.on_click = Some(Box::new(f)); self }
    pub fn layout(mut self, layout: LayoutProps) -> Self { self.layout = layout; self }
    pub fn style(mut self, style: ButtonStyle) -> Self { self.style = style; self }

    pub fn build(self) -> Box<dyn Widget> {
        Box::new(ButtonWidget {
            id: self.id, label: self.label, label_scale: self.label_scale,
            on_click: self.on_click, layout: self.layout, style: self.style,
        })
    }
}

struct ButtonWidget {
    id: WidgetId, label: String, label_scale: f32,
    on_click: Option<Box<dyn FnMut(&mut WidgetNode) + Send + Sync>>,
    layout: LayoutProps, style: ButtonStyle,
}

impl Widget for ButtonWidget {
    fn id(&self) -> WidgetId { self.id }

    fn update(&mut self, node: &mut WidgetNode, _ctx: &mut UpdateContext) {
        let mut btn = node.components.get_mut::<ButtonComponent>()
            .cloned()
            .unwrap_or_else(|| ButtonComponent {
                pressed: false, hovered: false, clicked: false, toggle: self.style.toggle, toggled: false,
            });
        btn.hovered = node.state.hovered;
        btn.pressed = node.state.active;
        btn.clicked = false;

        if !node.state.active && btn.pressed && node.state.hovered {
            btn.clicked = true;
            if let Some(cb) = &mut self.on_click { cb(node); }
            if self.style.toggle { btn.toggled = !btn.toggled; }
        }
        btn.pressed = node.state.active;
        node.components.add(btn);
    }

    fn compute_layout(&mut self, node: &mut WidgetNode, constraints: LayoutConstraints) -> Vec2 {
        let text_w = text_width_static(&self.label, self.label_scale);
        let text_h = 7.0 * self.label_scale;
        let padding = 16.0;

        let w = (text_w + padding * 2.0).max(node.layout.min_size.x)
            .clamp(constraints.min_width, constraints.max_width);
        let h = (text_h + padding * 2.0).max(node.layout.min_size.y)
            .clamp(constraints.min_height, constraints.max_height);
        Vec2::new(w, h)
    }

    fn render(&mut self, node: &WidgetNode, draw: &mut dyn DrawList) {
        let rect = node.computed.rect;
        let default = ButtonComponent::default();
        let btn = node.components.get::<ButtonComponent>().unwrap_or(&default);

        let (bg, border) = if btn.pressed {
            (self.style.pressed_bg, self.style.border_focused)
        } else if btn.hovered || node.state.focused {
            (self.style.hovered_bg, self.style.border_focused)
        } else { (self.style.normal_bg, self.style.border) };

        draw.rect_rounded(rect.x + 2.0, rect.y + 2.0, rect.w, rect.h, self.style.radius, self.style.shadow_color);
        draw.rect_rounded(rect.x, rect.y, rect.w, rect.h, self.style.radius, bg);
        draw.rect_rounded(rect.x, rect.y, rect.w, rect.h, self.style.radius, border);

        let text_w = text_width_static(&self.label, self.label_scale);
        let text_h = 7.0 * self.label_scale;
        draw.text(&self.label, rect.x + (rect.w - text_w) * 0.5, rect.y + (rect.h - text_h) * 0.5, self.label_scale, self.style.text_color);
    }

    fn handle_event(&mut self, node: &mut WidgetNode, event: &InputEvent) -> EventPropagation {
        match event.event {
            InteractionEvent::HoverEnter => { node.state.hovered = true; EventPropagation::Continue }
            InteractionEvent::HoverExit => { node.state.hovered = false; EventPropagation::Continue }
            InteractionEvent::Press => { node.state.active = true; EventPropagation::Continue }
            InteractionEvent::Release => { node.state.active = false; EventPropagation::Continue }
            InteractionEvent::Click => EventPropagation::Handled,
            _ => EventPropagation::Continue,
        }
    }
}

fn text_width_static(text: &str, scale: f32) -> f32 {
    text.chars().filter(|c| (32..=126).contains(&(*c as u32)))
        .count() as f32 * 7.0 * scale
}