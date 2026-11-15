//! Dropdown/Select widget.

use std::any::Any;
use glam::Vec2;

use crate::widget_system::core::*;
use crate::widget_system::tree::*;

/// Dropdown component for state.
#[derive(Debug, Clone, Default)]
pub struct DropdownComponent {
    pub open: bool,
    pub selected_index: usize,
    pub hovered_index: Option<usize>,
}

impl Component for DropdownComponent {
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

/// Dropdown widget builder.
pub struct Dropdown {
    pub id: WidgetId,
    pub label: String,
    pub items: Vec<String>,
    pub selected: usize,
    pub scale: f32,
    pub on_change: Option<Box<dyn FnMut(usize, &mut WidgetNode) + Send + Sync>>,
    pub layout: LayoutProps,
    pub style: DropdownStyle,
}

#[derive(Debug, Clone)]
pub struct DropdownStyle {
    pub button_bg: [f32; 4],
    pub button_hovered: [f32; 4],
    pub button_pressed: [f32; 4],
    pub list_bg: [f32; 4],
    pub list_border: [f32; 4],
    pub item_hovered: [f32; 4],
    pub item_selected: [f32; 4],
    pub text_color: [f32; 4],
    pub radius: f32,
    pub max_height: f32,
}

impl Default for DropdownStyle {
    fn default() -> Self {
        Self {
            button_bg: [0.08, 0.09, 0.11, 0.95],
            button_hovered: [0.12, 0.14, 0.17, 0.95],
            button_pressed: [0.16, 0.18, 0.22, 0.95],
            list_bg: [0.06, 0.07, 0.09, 0.98],
            list_border: [0.2, 0.22, 0.25, 1.0],
            item_hovered: [0.15, 0.18, 0.22, 0.95],
            item_selected: [0.2, 0.3, 0.4, 0.95],
            text_color: [0.95, 0.95, 0.98, 1.0],
            radius: 6.0,
            max_height: 200.0,
        }
    }
}

impl Dropdown {
    pub fn new(label: impl Into<String>, items: Vec<String>) -> Self {
        Self { id: WidgetId::new(), label: label.into(), items, selected: 0, scale: 1.0,
            on_change: None, layout: LayoutProps::default(), style: DropdownStyle::default() }
    }

    pub fn id(mut self, id: WidgetId) -> Self { self.id = id; self }
    pub fn selected(mut self, index: usize) -> Self { self.selected = index.min(self.items.len().saturating_sub(1)); self }
    pub fn scale(mut self, scale: f32) -> Self { self.scale = scale; self }
    pub fn on_change<F>(mut self, f: F) -> Self
    where F: FnMut(usize, &mut WidgetNode) + Send + Sync + 'static
    { self.on_change = Some(Box::new(f)); self }
    pub fn layout(mut self, layout: LayoutProps) -> Self { self.layout = layout; self }
    pub fn style(mut self, style: DropdownStyle) -> Self { self.style = style; self }

    pub fn build(self) -> Box<dyn Widget> {
        Box::new(DropdownWidget {
            id: self.id, label: self.label, items: self.items, selected: self.selected,
            scale: self.scale, on_change: self.on_change, layout: self.layout, style: self.style,
        })
    }
}

struct DropdownWidget {
    id: WidgetId, label: String, items: Vec<String>, selected: usize,
    scale: f32, on_change: Option<Box<dyn FnMut(usize, &mut WidgetNode) + Send + Sync>>,
    layout: LayoutProps, style: DropdownStyle,
}

impl Widget for DropdownWidget {
    fn id(&self) -> WidgetId { self.id }

    fn update(&mut self, node: &mut WidgetNode, ctx: &mut UpdateContext) {
        let mut dd = node.components.get_mut::<DropdownComponent>()
            .cloned()
            .unwrap_or_else(DropdownComponent::default);

        if let Some(pointer) = ctx.primary_pointer {
            if pointer.just_pressed && dd.open {
                let button_rect = Rect::new(node.computed.rect.x, node.computed.rect.y, node.computed.rect.w, 35.0);
                if !button_rect.contains(pointer.position) { dd.open = false; }
            }
        }
        node.components.add(dd);
    }

    fn compute_layout(&mut self, node: &mut WidgetNode, constraints: LayoutConstraints) -> Vec2 {
        let text_w = text_width_static(&self.label, self.scale);
        let w = (text_w + 50.0).max(node.layout.min_size.x).clamp(constraints.min_width, constraints.max_width);
        Vec2::new(w, 35.0)
    }

    fn render(&mut self, node: &WidgetNode, draw: &mut dyn DrawList) {
        let default = DropdownComponent::default();
        let dd = node.components.get::<DropdownComponent>().unwrap_or(&default);
        let rect = node.computed.rect;

        let (bg, border) = if dd.open {
            (self.style.button_pressed, self.style.list_border)
        } else if node.state.hovered { (self.style.button_hovered, self.style.list_border) }
        else { (self.style.button_bg, [0.2, 0.22, 0.25, 1.0]) };

        draw.rect_rounded(rect.x, rect.y, rect.w, rect.h, self.style.radius, bg);
        draw.rect_rounded(rect.x, rect.y, rect.w, rect.h, self.style.radius, border);

        let display_text = self.items.get(self.selected).map(|s| s.as_str()).unwrap_or(&self.label);
        draw.text(display_text, rect.x + 12.0, rect.y + (rect.h - 7.0 * self.scale) * 0.5, self.scale, self.style.text_color);

        let arrow = if dd.open { "▲" } else { "▼" };
        let arrow_w = text_width_static(arrow, self.scale);
        draw.text(arrow, rect.x + rect.w - arrow_w - 12.0, rect.y + (rect.h - 7.0 * self.scale) * 0.5, self.scale, self.style.text_color);

        if dd.open {
            // Inline render_list
            let item_h = 30.0;
            let list_w = rect.w;
            let list_h = (self.items.len() as f32 * item_h).min(self.style.max_height);
            let list_x = rect.x;
            let list_y = rect.y + rect.h + 2.0;

            draw.rect_rounded(list_x, list_y, list_w, list_h, self.style.radius, self.style.list_bg);
            draw.rect_rounded(list_x, list_y, list_w, list_h, self.style.radius, self.style.list_border);

            for (i, item) in self.items.iter().enumerate() {
                let item_y = list_y + i as f32 * item_h;
                let item_rect = Rect::new(list_x, item_y, list_w, item_h);
                let is_selected = i == self.selected;
                let is_hovered = dd.hovered_index == Some(i);

                if is_selected { draw.rect(item_rect.x, item_rect.y, item_rect.w, item_rect.h, self.style.item_selected); }
                else if is_hovered { draw.rect(item_rect.x, item_rect.y, item_rect.w, item_rect.h, self.style.item_hovered); }

                if is_selected { draw.rect(item_rect.x, item_rect.y, 3.0, item_rect.h, [1.0, 0.55, 0.1, 1.0]); }
                draw.text(item, item_rect.x + 12.0, item_y + (item_h - 7.0 * self.scale) * 0.5, self.scale, self.style.text_color);
            }
        }
    }

    fn handle_event(&mut self, node: &mut WidgetNode, event: &InputEvent) -> EventPropagation {
        let mut dd = node.components.get_mut::<DropdownComponent>()
            .cloned()
            .unwrap_or_else(DropdownComponent::default);

        match event.event {
            InteractionEvent::Click => {
                if !dd.open { dd.open = true; }
                else { dd.open = false; }
                EventPropagation::Handled
            }
            InteractionEvent::HoverEnter => { node.state.hovered = true; EventPropagation::Continue }
            InteractionEvent::HoverExit => { node.state.hovered = false; EventPropagation::Continue }
            _ => EventPropagation::Continue,
        }
    }
}

fn text_width_static(text: &str, scale: f32) -> f32 {
    text.chars().filter(|c| (32..=126).contains(&(*c as u32)))
        .count() as f32 * 7.0 * scale
}