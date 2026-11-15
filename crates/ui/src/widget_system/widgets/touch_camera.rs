//! Touch camera control widget for mobile.

use std::any::Any;
use glam::Vec2;

use crate::widget_system::core::*;
use crate::widget_system::tree::*;

/// Touch camera component for state.
#[derive(Debug, Clone, Default)]
pub struct TouchCameraComponent {
    pub active: bool,
    pub last_pos: Vec2,
    pub delta: Vec2,
    pub touch_id: Option<u64>,
}

impl Component for TouchCameraComponent {
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

/// Touch camera widget builder.
pub struct TouchCamera {
    pub id: WidgetId,
    pub area: Rect,
    pub sensitivity: f32,
    pub on_delta: Option<Box<dyn FnMut(Vec2, &mut WidgetNode) + Send + Sync>>,
    pub style: TouchCameraStyle,
}

#[derive(Debug, Clone)]
pub struct TouchCameraStyle {
    pub indicator_color: [f32; 4],
    pub indicator_size: f32,
}

impl Default for TouchCameraStyle {
    fn default() -> Self {
        Self { indicator_color: [1.0, 0.55, 0.1, 0.15], indicator_size: 40.0 }
    }
}

impl TouchCamera {
    pub fn new(area: Rect) -> Self {
        Self { id: WidgetId::new(), area, sensitivity: 0.005, on_delta: None, style: TouchCameraStyle::default() }
    }

    pub fn id(mut self, id: WidgetId) -> Self { self.id = id; self }
    pub fn area(mut self, area: Rect) -> Self { self.area = area; self }
    pub fn sensitivity(mut self, sensitivity: f32) -> Self { self.sensitivity = sensitivity; self }
    pub fn on_delta<F>(mut self, f: F) -> Self
    where F: FnMut(Vec2, &mut WidgetNode) + Send + Sync + 'static
    { self.on_delta = Some(Box::new(f)); self }
    pub fn style(mut self, style: TouchCameraStyle) -> Self { self.style = style; self }

    pub fn build(self) -> Box<dyn Widget> {
        Box::new(TouchCameraWidget {
            id: self.id, area: self.area, sensitivity: self.sensitivity,
            on_delta: self.on_delta, style: self.style,
        })
    }
}

struct TouchCameraWidget {
    id: WidgetId, area: Rect, sensitivity: f32,
    on_delta: Option<Box<dyn FnMut(Vec2, &mut WidgetNode) + Send + Sync>>,
    style: TouchCameraStyle,
}

impl Widget for TouchCameraWidget {
    fn id(&self) -> WidgetId { self.id }

    fn update(&mut self, node: &mut WidgetNode, ctx: &mut UpdateContext) {
        let mut cam = node.components.get_mut::<TouchCameraComponent>()
            .cloned()
            .unwrap_or_else(TouchCameraComponent::default);

        let pointer = ctx.pointers.iter().find(|p| {
            (p.id == 0 || p.position.x >= ctx.screen_size.x * 0.5) && self.area.contains(p.position)
        });

        if let Some(p) = pointer {
            if p.pressed && self.area.contains(p.position) {
                if cam.active && cam.touch_id == Some(p.id) {
                    cam.delta = p.drag_delta() * self.sensitivity;
                    if let Some(cb) = &mut self.on_delta { cb(cam.delta, node); }
                } else if !cam.active {
                    cam.active = true; cam.touch_id = Some(p.id);
                    cam.last_pos = p.position; cam.delta = Vec2::ZERO;
                }
            } else if cam.active && cam.touch_id == Some(p.id) && !p.pressed {
                cam.active = false; cam.touch_id = None; cam.delta = Vec2::ZERO;
            }
        } else if cam.active {
            cam.active = false; cam.touch_id = None; cam.delta = Vec2::ZERO;
        }
        node.components.add(cam);
    }

    fn compute_layout(&mut self, _node: &mut WidgetNode, _constraints: LayoutConstraints) -> Vec2 {
        Vec2::new(self.area.w, self.area.h)
    }

    fn render(&mut self, node: &WidgetNode, draw: &mut dyn DrawList) {
        let default = TouchCameraComponent::default();
        let cam = node.components.get::<TouchCameraComponent>().unwrap_or(&default);
        if cam.active {
            let x = self.area.x + 10.0; let y = self.area.y + 10.0;
            let s = self.style.indicator_size;
            draw.rect_rounded(x, y, s, s, 8.0, self.style.indicator_color);
        }
    }

    fn handle_event(&mut self, _node: &mut WidgetNode, _event: &InputEvent) -> EventPropagation {
        EventPropagation::Continue
    }
}