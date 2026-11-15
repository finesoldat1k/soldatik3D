//! Core types for the widget system.

use std::any::Any;
use std::collections::HashMap;
use glam::Vec2;

/// Unique identifier for a widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WidgetId(pub u64);

impl WidgetId {
    pub fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        WidgetId(COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    pub fn from_str(s: &str) -> Self {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        s.hash(&mut hasher);
        WidgetId(hasher.finish())
    }
}

/// Rectangle in screen space (pixels, top-left origin).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w: w.max(0.0), h: h.max(0.0) }
    }

    pub fn contains(&self, point: Vec2) -> bool {
        point.x >= self.x && point.x < self.x + self.w &&
        point.y >= self.y && point.y < self.y + self.h
    }

    pub fn center(&self) -> Vec2 {
        Vec2::new(self.x + self.w * 0.5, self.y + self.h * 0.5)
    }

    pub fn inset(&self, margin: f32) -> Self {
        Self {
            x: self.x + margin,
            y: self.y + margin,
            w: (self.w - margin * 2.0).max(0.0),
            h: (self.h - margin * 2.0).max(0.0),
        }
    }

    pub fn intersect(&self, other: Rect) -> Option<Rect> {
        let x1 = self.x.max(other.x);
        let y1 = self.y.max(other.y);
        let x2 = (self.x + self.w).min(other.x + other.w);
        let y2 = (self.y + self.h).min(other.y + other.h);
        if x1 < x2 && y1 < y2 {
            Some(Rect::new(x1, y1, x2 - x1, y2 - y1))
        } else {
            None
        }
    }
}

/// Flexbox-style layout properties.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutProps {
    pub flex_direction: FlexDirection,
    pub align_items: AlignItems,
    pub justify_content: JustifyContent,
    pub gap: f32,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: FlexBasis,
    pub padding: Rect,
    pub margin: Rect,
    pub min_size: Vec2,
    pub max_size: Vec2,
    pub flex: bool,
}

impl Default for LayoutProps {
    fn default() -> Self {
        Self {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            justify_content: JustifyContent::Start,
            gap: 0.0,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_basis: FlexBasis::Auto,
            padding: Rect::new(0.0, 0.0, 0.0, 0.0),
            margin: Rect::new(0.0, 0.0, 0.0, 0.0),
            min_size: Vec2::ZERO,
            max_size: Vec2::new(f32::INFINITY, f32::INFINITY),
            flex: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FlexDirection { Row, Column, RowReverse, ColumnReverse }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AlignItems { Start, Center, End, Stretch, Baseline }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JustifyContent { Start, Center, End, SpaceBetween, SpaceAround, SpaceEvenly }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FlexBasis { Auto, Content, Length(f32) }

/// Computed layout result for a widget.
#[derive(Debug, Clone, PartialEq)]
pub struct ComputedLayout {
    pub rect: Rect,
    pub content_rect: Rect,
    pub children: Vec<ComputedLayout>,
}

/// Pointer input state for hit testing.
#[derive(Debug, Clone, Copy)]
pub struct PointerState {
    pub position: Vec2,
    pub pressed: bool,
    pub just_pressed: bool,
    pub just_released: bool,
    pub prev_position: Vec2,
    pub id: u64,
}

impl PointerState {
    pub fn drag_delta(&self) -> Vec2 {
        self.position - self.prev_position
    }
}

/// Interaction event types.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InteractionEvent {
    HoverEnter,
    HoverExit,
    Press,
    Release,
    Click,
    DragStart,
    Drag,
    DragEnd,
    Scroll(f32),
}

/// Event propagation control.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EventPropagation {
    Continue,
    Stop,
    Handled,
}

/// Base trait for UI components (ECS-style).
pub trait Component: Any + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// Component storage for a widget.
#[derive(Default)]
pub struct ComponentStore {
    components: HashMap<std::any::TypeId, Box<dyn Component>>,
}

impl ComponentStore {
    pub fn new() -> Self { Self::default() }

    pub fn add<C: Component>(&mut self, component: C) {
        self.components.insert(std::any::TypeId::of::<C>(), Box::new(component));
    }

    pub fn get<C: Component>(&self) -> Option<&C> {
        self.components.get(&std::any::TypeId::of::<C>())
            .map(|b| b.as_any().downcast_ref::<C>().unwrap())
    }

    pub fn get_mut<C: Component>(&mut self) -> Option<&mut C> {
        self.components.get_mut(&std::any::TypeId::of::<C>())
            .map(|b| b.as_any_mut().downcast_mut::<C>().unwrap())
    }

    pub fn remove<C: Component>(&mut self) -> Option<Box<C>> {
        self.components.remove(&std::any::TypeId::of::<C>())
            .and_then(|b| {
                // Need to convert &dyn Any to Box<dyn Any> for downcast
                // This is a limitation - we can't easily downcast a reference
                // So we'll return None for now
                None::<Box<C>>
            })
    }

    pub fn has<C: Component>(&self) -> bool {
        self.components.contains_key(&std::any::TypeId::of::<C>())
    }
}

/// Helper to add a component to store, taking ownership.
pub fn add_component<C: Component>(store: &mut ComponentStore, component: C) {
    store.add(component);
}

/// Widget state that persists across frames.
#[derive(Default)]
pub struct WidgetState {
    pub hovered: bool,
    pub active: bool,
    pub focused: bool,
    pub last_pointer_id: Option<u64>,
    #[allow(clippy::type_complexity)]
    pub user_data: HashMap<String, Box<dyn Any + Send + Sync>>,
}

impl WidgetState {
    pub fn new() -> Self { Self::default() }
}

/// Layout constraints for compute_layout.
#[derive(Debug, Clone, Copy)]
pub struct LayoutConstraints {
    pub min_width: f32,
    pub max_width: f32,
    pub min_height: f32,
    pub max_height: f32,
}

impl Default for LayoutConstraints {
    fn default() -> Self {
        Self { min_width: 0.0, max_width: f32::INFINITY, min_height: 0.0, max_height: f32::INFINITY }
    }
}

/// Input event with target widget.
#[derive(Debug, Clone)]
pub struct InputEvent {
    pub event: InteractionEvent,
    pub target: WidgetId,
    pub pointer: PointerState,
    pub propagation: EventPropagation,
}

/// Trait for drawing operations (backend-agnostic).
pub trait DrawList {
    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]);
    fn rect_rounded(&mut self, x: f32, y: f32, w: f32, h: f32, radius: f32, color: [f32; 4]);
    fn text(&mut self, text: &str, x: f32, y: f32, scale: f32, color: [f32; 4]);
    fn text_width(&self, text: &str, scale: f32) -> f32;
}

/// Theme for consistent styling.
#[derive(Debug, Clone)]
pub struct Theme {
    pub bg: [f32; 4],
    pub bg_hovered: [f32; 4],
    pub bg_pressed: [f32; 4],
    pub border: [f32; 4],
    pub border_focused: [f32; 4],
    pub text: [f32; 4],
    pub text_disabled: [f32; 4],
    pub accent: [f32; 4],
    pub accent_hovered: [f32; 4],
    pub accent_pressed: [f32; 4],
    pub danger: [f32; 4],
    pub success: [f32; 4],
    pub shadow: [f32; 4],
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            bg: [0.08, 0.09, 0.11, 0.95],
            bg_hovered: [0.12, 0.14, 0.17, 0.95],
            bg_pressed: [0.16, 0.18, 0.22, 0.95],
            border: [0.2, 0.22, 0.25, 1.0],
            border_focused: [1.0, 0.6, 0.1, 1.0],
            text: [0.95, 0.95, 0.98, 1.0],
            text_disabled: [0.45, 0.48, 0.52, 1.0],
            accent: [1.0, 0.55, 0.1, 1.0],
            accent_hovered: [1.0, 0.65, 0.2, 1.0],
            accent_pressed: [0.9, 0.5, 0.05, 1.0],
            danger: [0.85, 0.25, 0.2, 1.0],
            success: [0.2, 0.75, 0.3, 1.0],
            shadow: [0.0, 0.0, 0.0, 0.4],
        }
    }
}