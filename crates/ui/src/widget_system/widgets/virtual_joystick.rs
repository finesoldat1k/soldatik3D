//! Virtual joystick widget for touch input.

use std::any::Any;
use glam::Vec2;

use crate::widget_system::core::*;
use crate::widget_system::tree::*;

/// Virtual joystick component for state.
#[derive(Debug, Clone, Default)]
pub struct VirtualJoystickComponent {
    pub active: bool,
    pub center: Vec2,
    pub stick_pos: Vec2,
    pub output: Vec2,
    pub touch_id: Option<u64>,
}

impl Component for VirtualJoystickComponent {
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

/// Virtual joystick widget builder.
pub struct VirtualJoystick {
    pub id: WidgetId,
    pub area: Rect,
    pub deadzone: f32,
    pub on_change: Option<Box<dyn FnMut(Vec2, &mut WidgetNode) + Send + Sync>>,
    pub style: VirtualJoystickStyle,
}

#[derive(Debug, Clone)]
pub struct VirtualJoystickStyle {
    pub base_color: [f32; 4],
    pub base_active_color: [f32; 4],
    pub handle_color: [f32; 4],
    pub handle_border: [f32; 4],
    pub radius: f32,
    pub handle_radius_ratio: f32,
}

impl Default for VirtualJoystickStyle {
    fn default() -> Self {
        Self { base_color: [0.15, 0.18, 0.22, 0.5],
            base_active_color: [0.2, 0.3, 0.45, 0.7], handle_color: [0.3, 0.6, 1.0, 0.9],
            handle_border: [0.5, 0.8, 1.0, 1.0], radius: 80.0, handle_radius_ratio: 0.45 }
    }
}

impl VirtualJoystick {
    pub fn new(area: Rect) -> Self {
        Self { id: WidgetId::new(), area, deadzone: 0.15, on_change: None, style: VirtualJoystickStyle::default() }
    }

    pub fn id(mut self, id: WidgetId) -> Self { self.id = id; self }
    pub fn area(mut self, area: Rect) -> Self { self.area = area; self }
    pub fn deadzone(mut self, deadzone: f32) -> Self { self.deadzone = deadzone; self }
    pub fn on_change<F>(mut self, f: F) -> Self
    where F: FnMut(Vec2, &mut WidgetNode) + Send + Sync + 'static
    { self.on_change = Some(Box::new(f)); self }
    pub fn style(mut self, style: VirtualJoystickStyle) -> Self { self.style = style; self }

    pub fn build(self) -> Box<dyn Widget> {
        Box::new(VirtualJoystickWidget {
            id: self.id, area: self.area, deadzone: self.deadzone,
            on_change: self.on_change, style: self.style,
        })
    }
}

struct VirtualJoystickWidget {
    id: WidgetId, area: Rect, deadzone: f32,
    on_change: Option<Box<dyn FnMut(Vec2, &mut WidgetNode) + Send + Sync>>,
    style: VirtualJoystickStyle,
}

impl Widget for VirtualJoystickWidget {
    fn id(&self) -> WidgetId { self.id }

    fn update(&mut self, node: &mut WidgetNode, ctx: &mut UpdateContext) {
        let mut joy = node.components.get_mut::<VirtualJoystickComponent>()
            .cloned()
            .unwrap_or_else(VirtualJoystickComponent::default);

        let pointer = ctx.pointers.iter().find(|p| {
            (p.id == 0 || p.position.x < ctx.screen_size.x * 0.5) && self.area.contains(p.position)
        });

        if let Some(p) = pointer {
            if p.just_pressed {
                joy.active = true; joy.touch_id = Some(p.id);
                joy.center = self.area.center(); joy.stick_pos = Vec2::ZERO;
            } else if joy.active && joy.touch_id == Some(p.id) {
                if p.pressed {
                    let delta = p.position - joy.center;
                    let max_r = self.style.radius * 0.9;
                    let dist = delta.length();
                    joy.stick_pos = if dist > max_r { delta.normalize() * max_r } else { delta };
                    joy.output = if joy.stick_pos.length() < max_r * self.deadzone {
                        Vec2::ZERO } else { joy.stick_pos / max_r };
                    if let Some(cb) = &mut self.on_change { cb(joy.output, node); }
                } else {
                    joy.active = false; joy.touch_id = None;
                    joy.stick_pos = Vec2::ZERO; joy.output = Vec2::ZERO;
                }
            }
        } else if joy.active {
            joy.active = false; joy.touch_id = None;
            joy.stick_pos = Vec2::ZERO; joy.output = Vec2::ZERO;
        }
        node.components.add(joy);
    }

    fn compute_layout(&mut self, _node: &mut WidgetNode, _constraints: LayoutConstraints) -> Vec2 {
        let d = self.style.radius * 2.0; Vec2::new(d, d)
    }

    fn render(&mut self, node: &WidgetNode, draw: &mut dyn DrawList) {
        let default = VirtualJoystickComponent::default();
        let joy = node.components.get::<VirtualJoystickComponent>().unwrap_or(&default);
        let center = self.area.center();
        let radius = self.style.radius * 0.9;

        let base_color = if joy.active { self.style.base_active_color } else { self.style.base_color };
        draw.rect_rounded(center.x - radius, center.y - radius, radius * 2.0, radius * 2.0, radius, base_color);

        if joy.active {
            let handle_r = radius * self.style.handle_radius_ratio;
            let handle_x = center.x + joy.stick_pos.x - handle_r;
            let handle_y = center.y + joy.stick_pos.y - handle_r;
            draw.rect_rounded(handle_x, handle_y, handle_r * 2.0, handle_r * 2.0, handle_r, self.style.handle_color);
            draw.rect_rounded(handle_x, handle_y, handle_r * 2.0, 2.0, handle_r, self.style.handle_border);
        } else {
            let dot = 8.0;
            draw.rect_rounded(center.x - dot * 0.5, center.y - dot * 0.5, dot, dot, dot * 0.5, [0.4, 0.5, 0.6, 0.4]);
        }
    }

    fn handle_event(&mut self, _node: &mut WidgetNode, _event: &InputEvent) -> EventPropagation {
        EventPropagation::Continue
    }
}