//! Touch-friendly UI widget system for Soldatik3D.
//!
//! Provides immediate-mode widgets with full touch and mouse support.
//! All widgets return interaction state so games can respond to input.

use crate::{UiDrawList, UiVertex};
use glam::Vec2;

/// Unique widget identifier for tracking interaction state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WidgetId(pub u64);

impl WidgetId {
    /// Generates a new unique widget ID.
    pub fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        WidgetId(COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    /// Creates a widget ID from a string hash (stable across frames).
    pub fn from_str(s: &str) -> Self {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        s.hash(&mut hasher);
        WidgetId(hasher.finish())
    }
}

/// Interaction state returned by widgets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interaction {
    /// No interaction this frame.
    None,
    /// Widget is being hovered (mouse) or touched.
    Hovered,
    /// Widget was clicked/tapped this frame.
    Clicked,
    /// Widget is being dragged (touch or mouse held down).
    Dragged,
    /// Widget was released after drag.
    Released,
    /// Value changed (for sliders, toggles, etc.).
    Changed,
}

/// Touch/mouse input state for widget hit testing.
#[derive(Debug, Clone, Copy)]
pub struct PointerState {
    /// Current pointer position in screen coordinates.
    pub position: Vec2,
    /// Whether the pointer is currently pressed.
    pub pressed: bool,
    /// Whether the pointer was pressed this frame.
    pub just_pressed: bool,
    /// Whether the pointer was released this frame.
    pub just_released: bool,
    /// Previous frame position for drag delta.
    pub prev_position: Vec2,
    /// Unique pointer ID (for multi-touch).
    pub id: u64,
}

impl PointerState {
    /// Creates a pointer state from an input source that provides touch/mouse data.
    /// Uses duck typing via generics - input must have methods:
    /// - `first_touch() -> Option<&T>`
    /// - `touch(id: u64) -> Option<&T>`
    /// - `touches() -> &[T]`
    /// - `primary_clicked() -> bool`
    /// - `mouse_delta() -> Vec2`
    /// - `cursor_position() -> Vec2`
    /// And T must have: `position()`, `prev_position()`, `id()`, `started()`, `ended()`
    pub fn from_input<I, T>(input: &I, pointer_id: u64) -> Option<Self>
    where
        I: TouchInputMethods<Touch = T>,
        T: TouchStateMethods,
    {
        if pointer_id == 0 {
            if let Some(touch) = input.first_touch() {
                Some(Self {
                    position: touch.position(),
                    pressed: !touch.ended(),
                    just_pressed: touch.started(),
                    just_released: touch.ended(),
                    prev_position: touch.prev_position(),
                    id: touch.id(),
                })
            } else if input.primary_clicked() || input.mouse_delta() != Vec2::ZERO {
                Some(Self {
                    position: input.cursor_position(),
                    pressed: input.primary_clicked(),
                    just_pressed: input.primary_clicked(),
                    just_released: false,
                    prev_position: input.cursor_position() - input.mouse_delta(),
                    id: 0,
                })
            } else {
                None
            }
        } else {
            input.touch(pointer_id).map(|touch| Self {
                position: touch.position(),
                pressed: !touch.ended(),
                just_pressed: touch.started(),
                just_released: touch.ended(),
                prev_position: touch.prev_position(),
                id: touch.id(),
            })
        }
    }

    /// Returns the drag delta since last frame.
    pub fn drag_delta(&self) -> Vec2 {
        self.position - self.prev_position
    }
}

/// Marker trait for input types that provide touch/mouse methods.
/// This is automatically implemented for any type with the required methods.
pub trait TouchInputMethods {
    type Touch: TouchStateMethods;
    fn first_touch(&self) -> Option<&Self::Touch>;
    fn touch(&self, id: u64) -> Option<&Self::Touch>;
    fn touches(&self) -> &[Self::Touch];
    fn primary_clicked(&self) -> bool;
    fn mouse_delta(&self) -> Vec2;
    fn cursor_position(&self) -> Vec2;
}

/// Marker trait for touch state types.
pub trait TouchStateMethods {
    fn position(&self) -> Vec2;
    fn prev_position(&self) -> Vec2;
    fn id(&self) -> u64;
    fn started(&self) -> bool;
    fn ended(&self) -> bool;
}

// Automatically implement for any type with the required methods
impl<I, T> TouchInputMethods for I
where
    I: TouchInputProvider<Touch = T>,
    T: TouchStateProvider,
{
    type Touch = T;
    fn first_touch(&self) -> Option<&Self::Touch> {
        TouchInputProvider::first_touch(self)
    }
    fn touch(&self, id: u64) -> Option<&Self::Touch> {
        TouchInputProvider::touch(self, id)
    }
    fn touches(&self) -> &[Self::Touch] {
        TouchInputProvider::touches(self)
    }
    fn primary_clicked(&self) -> bool {
        TouchInputProvider::primary_clicked(self)
    }
    fn mouse_delta(&self) -> Vec2 {
        TouchInputProvider::mouse_delta(self)
    }
    fn cursor_position(&self) -> Vec2 {
        TouchInputProvider::cursor_position(self)
    }
}

impl<T> TouchStateMethods for T
where
    T: TouchStateProvider,
{
    fn position(&self) -> Vec2 {
        TouchStateProvider::position(self)
    }
    fn prev_position(&self) -> Vec2 {
        TouchStateProvider::prev_position(self)
    }
    fn id(&self) -> u64 {
        TouchStateProvider::id(self)
    }
    fn started(&self) -> bool {
        TouchStateProvider::started(self)
    }
    fn ended(&self) -> bool {
        TouchStateProvider::ended(self)
    }
}

/// Minimal trait for input providers to avoid cyclic dependencies.
/// Implement this for your engine's Input type.
pub trait TouchInputProvider {
    type Touch: TouchStateProvider;
    fn first_touch(&self) -> Option<&Self::Touch>;
    fn touch(&self, id: u64) -> Option<&Self::Touch>;
    fn touches(&self) -> &[Self::Touch];
    fn primary_clicked(&self) -> bool;
    fn mouse_delta(&self) -> Vec2;
    fn cursor_position(&self) -> Vec2;
}

/// Minimal trait for touch state providers.
pub trait TouchStateProvider {
    fn position(&self) -> Vec2;
    fn prev_position(&self) -> Vec2;
    fn id(&self) -> u64;
    fn started(&self) -> bool;
    fn ended(&self) -> bool;
}

/// Rectangle helper for hit testing and layout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub fn contains(&self, point: Vec2) -> bool {
        point.x >= self.x
            && point.x <= self.x + self.w
            && point.y >= self.y
            && point.y <= self.y + self.h
    }

    pub fn center(&self) -> Vec2 {
        Vec2::new(self.x + self.w * 0.5, self.y + self.h * 0.5)
    }

    pub fn inset(&self, margin: f32) -> Self {
        Self {
            x: self.x + margin,
            y: self.y + margin,
            w: self.w - margin * 2.0,
            h: self.h - margin * 2.0,
        }
    }
}

/// Color theme for consistent styling.
#[derive(Debug, Clone, Copy)]
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

impl Theme {
    /// Dark theme with orange accent (default).
    pub fn dark() -> Self {
        Self::default()
    }

    /// Light theme.
    pub fn light() -> Self {
        Self {
            bg: [0.92, 0.93, 0.95, 0.95],
            bg_hovered: [0.85, 0.87, 0.9, 0.95],
            bg_pressed: [0.78, 0.8, 0.84, 0.95],
            border: [0.65, 0.68, 0.72, 1.0],
            border_focused: [1.0, 0.55, 0.1, 1.0],
            text: [0.1, 0.12, 0.15, 1.0],
            text_disabled: [0.5, 0.52, 0.55, 1.0],
            accent: [1.0, 0.55, 0.1, 1.0],
            accent_hovered: [1.0, 0.65, 0.2, 1.0],
            accent_pressed: [0.9, 0.5, 0.05, 1.0],
            danger: [0.8, 0.2, 0.15, 1.0],
            success: [0.15, 0.7, 0.25, 1.0],
            shadow: [0.0, 0.0, 0.0, 0.25],
        }
    }
}

/// UI context for a single frame, managing widget state and input.
pub struct UiContext {
    pub draw: UiDrawList,
    pub theme: Theme,
    pub pointers: Vec<PointerState>,
    pub hot_widget: Option<WidgetId>,
    pub active_widget: Option<WidgetId>,
    pub focus_widget: Option<WidgetId>,
    pub last_click_time: f64,
    pub double_click_interval: f64,
}

impl UiContext {
    pub fn new(draw: UiDrawList, theme: Theme) -> Self {
        Self {
            draw,
            theme,
            pointers: Vec::new(),
            hot_widget: None,
            active_widget: None,
            focus_widget: None,
            last_click_time: 0.0,
            double_click_interval: 0.3,
        }
    }

    pub fn width(&self) -> f32 {
        self.draw.width()
    }

    pub fn height(&self) -> f32 {
        self.draw.height()
    }

    /// Updates pointer states from engine input.
    pub fn update_pointers<I, T>(&mut self, input: &I)
    where
        I: TouchInputMethods<Touch = T>,
        T: TouchStateMethods,
    {
        self.pointers.clear();

        // Primary pointer (mouse or first touch)
        if let Some(pointer) = PointerState::from_input(input, 0) {
            self.pointers.push(pointer);
        }

        // Additional touches
        for touch in input.touches() {
            if touch.id() != 0 {
                self.pointers.push(PointerState {
                    position: touch.position(),
                    pressed: !touch.ended(),
                    just_pressed: touch.started(),
                    just_released: touch.ended(),
                    prev_position: touch.prev_position(),
                    id: touch.id(),
                });
            }
        }
    }

    /// Gets the primary pointer if any.
    pub fn primary_pointer(&self) -> Option<&PointerState> {
        self.pointers
            .iter()
            .find(|p| p.id == 0)
            .or_else(|| self.pointers.first())
    }

    /// Tests if a rectangle is hovered/pressed by any pointer.
    fn test_rect(&self, rect: Rect) -> (bool, bool, Option<u64>) {
        for pointer in &self.pointers {
            if rect.contains(pointer.position) {
                return (true, pointer.pressed, Some(pointer.id));
            }
        }
        (false, false, None)
    }

    /// Begins interaction with a widget.
    fn begin_interaction(&mut self, id: WidgetId, _pointer_id: u64) {
        self.active_widget = Some(id);
        self.hot_widget = Some(id);
    }

    /// Ends interaction.
    fn end_interaction(&mut self, id: WidgetId) {
        if self.active_widget == Some(id) {
            self.active_widget = None;
        }
        if self.hot_widget == Some(id) {
            self.hot_widget = None;
        }
    }

    /// Checks if widget is hot (hovered).
    fn is_hot(&self, id: WidgetId) -> bool {
        self.hot_widget == Some(id)
    }

    /// Checks if widget is active (being dragged/pressed).
    fn is_active(&self, id: WidgetId, _pointer_id: u64) -> bool {
        self.active_widget == Some(id)
    }

    /// Draws a rounded rectangle approximation.
    pub fn rect_rounded(&mut self, x: f32, y: f32, w: f32, h: f32, _radius: f32, color: [f32; 4]) {
        // Draw as filled rect with corner cutouts (simplified)
        self.draw.rect(x, y, w, h, color);

        // Could add proper rounded corners with more vertices here
    }

    /// Draws a drop shadow behind a rect.
    pub fn rect_shadow(&mut self, x: f32, y: f32, w: f32, h: f32, radius: f32, offset: f32) {
        let shadow_color = self.theme.shadow;
        for i in 0..4 {
            let alpha = shadow_color[3] * (1.0 - i as f32 * 0.2);
            let color = [shadow_color[0], shadow_color[1], shadow_color[2], alpha];
            let o = offset + i as f32 * 1.5;
            self.rect_rounded(x + o, y + o, w, h, radius, color);
        }
    }

    // ============================================================
    // WIDGETS
    // ============================================================

    /// Button widget with touch support.
    pub fn button(&mut self, id: WidgetId, rect: Rect, label: &str, scale: f32) -> Interaction {
        let (hovered, pressed, pointer_id) = self.test_rect(rect);
        let mut interaction = Interaction::None;

        let (bg_color, border_color) = if self.is_active(id, pointer_id.unwrap_or(0)) && pressed {
            (self.theme.bg_pressed, self.theme.border_focused)
        } else if hovered || self.is_hot(id) {
            (self.theme.bg_hovered, self.theme.border_focused)
        } else {
            (self.theme.bg, self.theme.border)
        };

        // Shadow
        self.rect_shadow(rect.x, rect.y, rect.w, rect.h, 8.0, 2.0);

        // Background
        self.draw
            .rect_rounded(rect.x, rect.y, rect.w, rect.h, 8.0, bg_color);

        // Border
        self.draw
            .rect_rounded(rect.x, rect.y, rect.w, rect.h, 8.0, border_color);

        // Text
        let text_w = UiDrawList::text_width(label, scale);
        let text_x = rect.x + (rect.w - text_w) * 0.5;
        let text_y = rect.y + (rect.h - 7.0 * scale) * 0.5;
        self.draw
            .text(label, text_x, text_y, scale, self.theme.text);

        if pressed && hovered {
            if self.active_widget.is_none() {
                self.begin_interaction(id, pointer_id.unwrap_or(0));
            }
            interaction = Interaction::Dragged;
        } else if self.is_active(id, pointer_id.unwrap_or(0)) && !pressed {
            if hovered {
                interaction = Interaction::Clicked;
            }
            self.end_interaction(id);
        } else if hovered && !self.is_active(id, pointer_id.unwrap_or(0)) {
            self.hot_widget = Some(id);
            interaction = Interaction::Hovered;
        }

        interaction
    }

    /// Toggle button (on/off state).
    pub fn toggle(
        &mut self,
        id: WidgetId,
        rect: Rect,
        _label: &str,
        scale: f32,
        state: &mut bool,
    ) -> Interaction {
        let interaction = self.button(id, rect, if *state { "ON" } else { "OFF" }, scale);
        if interaction == Interaction::Clicked {
            *state = !*state;
            return Interaction::Changed;
        }
        interaction
    }

    /// Label/text widget.
    pub fn label(&mut self, rect: Rect, label: &str, scale: f32, color: Option<[f32; 4]>) {
        let text_color = color.unwrap_or(self.theme.text);
        let text_w = UiDrawList::text_width(label, scale);
        let text_x = rect.x + (rect.w - text_w) * 0.5;
        let text_y = rect.y + (rect.h - 7.0 * scale) * 0.5;
        self.draw.text(label, text_x, text_y, scale, text_color);
    }

    /// Progress bar widget.
    pub fn progress_bar(&mut self, rect: Rect, value: f32, show_text: bool) {
        let clamped = value.clamp(0.0, 1.0);

        // Background track
        self.draw
            .rect_rounded(rect.x, rect.y, rect.w, rect.h, rect.h * 0.5, self.theme.bg);

        // Fill
        let fill_w = rect.w * clamped;
        if fill_w > 0.0 {
            self.draw.rect_rounded(
                rect.x,
                rect.y,
                fill_w,
                rect.h,
                rect.h * 0.5,
                self.theme.accent,
            );
        }

        if show_text {
            let text = format!("{:.0}%", clamped * 100.0);
            let text_w = UiDrawList::text_width(&text, 1.0);
            let text_x = rect.x + (rect.w - text_w) * 0.5;
            let text_y = rect.y + (rect.h - 7.0) * 0.5;
            self.draw.text(&text, text_x, text_y, 1.0, self.theme.text);
        }
    }

    /// Slider widget.
    pub fn slider(
        &mut self,
        id: WidgetId,
        rect: Rect,
        value: &mut f32,
        min: f32,
        max: f32,
    ) -> Interaction {
        let (hovered, pressed, pointer_id) = self.test_rect(rect);
        let mut interaction = Interaction::None;

        let track_h = 6.0;
        let track_y = rect.y + (rect.h - track_h) * 0.5;
        let handle_r = rect.h * 0.5;
        let val = *value;
        let handle_x = rect.x + (rect.w - handle_r * 2.0) * (val - min) / (max - min) + handle_r;

        // Track
        self.draw.rect_rounded(
            rect.x,
            track_y,
            rect.w,
            track_h,
            track_h * 0.5,
            self.theme.bg,
        );
        self.draw.rect_rounded(
            rect.x,
            track_y,
            handle_x - rect.x + handle_r,
            track_h,
            track_h * 0.5,
            self.theme.accent,
        );

        // Handle
        let handle_color = if self.is_active(id, pointer_id.unwrap_or(0)) {
            self.theme.accent_pressed
        } else if hovered {
            self.theme.accent_hovered
        } else {
            self.theme.accent
        };
        self.draw.rect_rounded(
            handle_x - handle_r,
            rect.y,
            handle_r * 2.0,
            rect.h,
            handle_r,
            handle_color,
        );

        if pressed && hovered {
            self.begin_interaction(id, pointer_id.unwrap_or(0));
            interaction = Interaction::Dragged;
        } else if self.is_active(id, pointer_id.unwrap_or(0)) {
            if pressed {
                // Update value from pointer position
                let primary = self.primary_pointer();
                if let Some(p) = primary {
                    let t = ((p.position.x - rect.x) / rect.w).clamp(0.0, 1.0);
                    *value = min + t * (max - min);
                }
                interaction = Interaction::Changed;
            } else {
                self.end_interaction(id);
                interaction = Interaction::Released;
            }
        } else if hovered {
            self.hot_widget = Some(id);
            interaction = Interaction::Hovered;
        }

        interaction
    }

    /// Dropdown/menu widget.
    pub fn dropdown(
        &mut self,
        id: WidgetId,
        rect: Rect,
        label: &str,
        items: &[&str],
        selected: &mut usize,
        scale: f32,
    ) -> Interaction {
        let (_hovered, _pressed, _pointer_id) = self.test_rect(rect);
        let is_open = self.focus_widget == Some(id);
        let mut interaction = Interaction::None;

        // Main button
        let btn_interaction = self.button(id, rect, label, scale);
        if btn_interaction == Interaction::Clicked {
            if is_open {
                self.focus_widget = None;
            } else {
                self.focus_widget = Some(id);
            }
            return Interaction::Changed;
        }

        // Dropdown list
        if is_open {
            let item_h = rect.h;
            let list_h = items.len() as f32 * item_h;
            let list_rect = Rect::new(rect.x, rect.y + rect.h, rect.w, list_h);

            // Shadow and background
            self.rect_shadow(list_rect.x, list_rect.y, list_rect.w, list_rect.h, 8.0, 3.0);
            self.draw.rect_rounded(
                list_rect.x,
                list_rect.y,
                list_rect.w,
                list_rect.h,
                8.0,
                self.theme.bg,
            );

            for (i, item) in items.iter().enumerate() {
                let item_rect = Rect::new(
                    list_rect.x,
                    list_rect.y + i as f32 * item_h,
                    list_rect.w,
                    item_h,
                );
                let item_hovered = item_rect.contains(
                    self.primary_pointer()
                        .map(|p| p.position)
                        .unwrap_or(Vec2::ZERO),
                );

                if item_hovered {
                    self.draw.rect(
                        item_rect.x,
                        item_rect.y,
                        item_rect.w,
                        item_rect.h,
                        self.theme.bg_hovered,
                    );
                }

                if i == *selected {
                    self.draw.rect(
                        item_rect.x,
                        item_rect.y,
                        3.0,
                        item_rect.h,
                        self.theme.accent,
                    );
                }

                let text_x = item_rect.x + 12.0;
                let text_y = item_rect.y + (item_rect.h - 7.0 * scale) * 0.5;
                self.draw.text(item, text_x, text_y, scale, self.theme.text);

                // Click on item
                if let Some(p) = self.primary_pointer()
                    && p.just_released
                    && item_rect.contains(p.position)
                {
                    *selected = i;
                    self.focus_widget = None;
                    interaction = Interaction::Changed;
                }
            }

            // Close on click outside
            if let Some(p) = self.primary_pointer()
                && p.just_pressed
                && !rect.contains(p.position)
                && !list_rect.contains(p.position)
            {
                self.focus_widget = None;
            }
        }

        interaction
    }

    /// Virtual joystick widget - appears on touch in left area, follows finger.
    pub fn virtual_joystick(
        &mut self,
        id: WidgetId,
        area: Rect,
        deadzone: f32,
    ) -> (Vec2, Interaction) {
        let primary = self.primary_pointer();
        let mut stick_pos = Vec2::ZERO;
        let mut interaction = Interaction::None;

        if let Some(pointer) = primary {
            if pointer.id == 0 || pointer.position.x < self.width() * 0.5 {
                // This pointer is for the joystick (left side or mouse)
                if pointer.just_pressed && area.contains(pointer.position) {
                    // Start joystick at touch position
                    self.begin_interaction(id, pointer.id);
                    stick_pos = Vec2::ZERO;
                    interaction = Interaction::Dragged;
                } else if self.is_active(id, pointer.id) {
                    if pointer.pressed {
                        // Update joystick position to follow finger
                        let center = area.center();
                        let delta = pointer.position - center;
                        let max_r = area.w.min(area.h) * 0.5 * 0.9;
                        let dist = delta.length();

                        if dist > max_r {
                            stick_pos = delta.normalize() * max_r;
                        } else {
                            stick_pos = delta;
                        }

                        // Normalize output
                        let output = if stick_pos.length() < max_r * deadzone {
                            Vec2::ZERO
                        } else {
                            stick_pos / max_r
                        };

                        interaction = Interaction::Dragged;
                        return (output, interaction);
                    } else {
                        self.end_interaction(id);
                        interaction = Interaction::Released;
                    }
                }
            }
        }

        // Draw joystick base
        let center = area.center();
        let radius = area.w.min(area.h) * 0.5 * 0.9;

        // Outer ring
        self.draw.rect_rounded(
            center.x - radius,
            center.y - radius,
            radius * 2.0,
            radius * 2.0,
            radius,
            [0.15, 0.18, 0.22, 0.6],
        );

        // Inner handle (only when active)
        if self.active_widget == Some(id) {
            let handle_r = radius * 0.45;
            let handle_x = center.x + stick_pos.x - handle_r;
            let handle_y = center.y + stick_pos.y - handle_r;
            self.draw.rect_rounded(
                handle_x,
                handle_y,
                handle_r * 2.0,
                handle_r * 2.0,
                handle_r,
                self.theme.accent,
            );

            // Handle highlight
            self.draw.rect_rounded(
                handle_x,
                handle_y,
                handle_r * 2.0,
                2.0,
                handle_r,
                [1.0, 0.7, 0.3, 1.0],
            );
        } else {
            // Center dot when inactive
            let dot = 6.0;
            self.draw.rect_rounded(
                center.x - dot * 0.5,
                center.y - dot * 0.5,
                dot,
                dot,
                dot * 0.5,
                [0.4, 0.45, 0.5, 0.4],
            );
        }

        (stick_pos / radius.max(1.0), interaction)
    }

    /// Touch camera control - right side of screen for camera rotation.
    pub fn touch_camera(&mut self, id: WidgetId, area: Rect) -> Vec2 {
        let primary_id = self.primary_pointer().map(|p| p.id);
        let mut delta = Vec2::ZERO;

        if let Some(pointer) = self.primary_pointer() {
            if pointer.id == 0 || pointer.position.x >= self.width() * 0.5 {
                if pointer.pressed && area.contains(pointer.position) {
                    if self.active_widget == Some(id) {
                        delta = pointer.drag_delta() * 0.005; // Sensitivity
                    }
                    self.begin_interaction(id, pointer.id);
                } else if self.is_active(id, pointer.id) && !pointer.pressed {
                    self.end_interaction(id);
                }
            }
        }

        // Visual feedback when active
        if let Some(pid) = primary_id {
            if self.is_active(id, pid) {
                // Subtle indicator
                self.draw.rect_rounded(
                    area.x + 10.0,
                    area.y + 10.0,
                    40.0,
                    40.0,
                    8.0,
                    [1.0, 0.55, 0.1, 0.15],
                );
            }
        }

        delta
    }

    /// Scrollable panel widget.
    pub fn scroll_panel(
        &mut self,
        id: WidgetId,
        rect: Rect,
        content_h: f32,
        scroll: &mut f32,
    ) -> (Rect, Interaction) {
        let (hovered, pressed, pointer_id) = self.test_rect(rect);
        let mut interaction = Interaction::None;

        let max_scroll = (content_h - rect.h).max(0.0);
        *scroll = scroll.clamp(0.0, max_scroll);

        // Background
        self.draw
            .rect_rounded(rect.x, rect.y, rect.w, rect.h, 8.0, self.theme.bg);

        // Scrollbar
        if max_scroll > 0.0 {
            let bar_w = 8.0;
            let bar_x = rect.x + rect.w - bar_w - 4.0;
            let bar_h = (rect.h / content_h * rect.h).max(30.0);
            let bar_y = rect.y + 4.0 + *scroll / max_scroll * (rect.h - bar_h - 8.0);

            // Track
            self.draw.rect_rounded(
                bar_x,
                rect.y + 4.0,
                bar_w,
                rect.h - 8.0,
                bar_w * 0.5,
                [0.1, 0.12, 0.15, 0.5],
            );
            // Thumb
            self.draw
                .rect_rounded(bar_x, bar_y, bar_w, bar_h, bar_w * 0.5, self.theme.accent);

            // Drag scrollbar
            let thumb_rect = Rect::new(bar_x, bar_y, bar_w, bar_h);
            if pressed && thumb_rect.contains(self.primary_pointer().unwrap().position) {
                self.begin_interaction(id, pointer_id.unwrap_or(0));
            }
            if self.is_active(id, pointer_id.unwrap_or(0)) {
                if let Some(p) = self.primary_pointer() {
                    if p.pressed {
                        let track_h = rect.h - 8.0;
                        let rel_y = (p.position.y - rect.y - 4.0) / track_h;
                        *scroll = (rel_y * max_scroll).clamp(0.0, max_scroll);
                        interaction = Interaction::Dragged;
                    } else {
                        self.end_interaction(id);
                    }
                }
            }
        }

        // Mouse wheel scroll
        if hovered {
            for pointer in &self.pointers {
                if pointer.id == 0 {
                    // Mouse scroll would need scroll_delta from input
                }
            }
        }

        // Return content rect (offset by scroll)
        let content_rect = Rect::new(rect.x, rect.y - *scroll, rect.w, content_h);
        (content_rect, interaction)
    }

    /// Panel/container widget.
    pub fn panel(&mut self, rect: Rect) {
        self.rect_shadow(rect.x, rect.y, rect.w, rect.h, 12.0, 4.0);
        self.draw
            .rect_rounded(rect.x, rect.y, rect.w, rect.h, 12.0, self.theme.bg);
        self.draw
            .rect_rounded(rect.x, rect.y, rect.w, rect.h, 12.0, self.theme.border);
    }

    /// Layout helpers
    pub fn layout_horizontal(&self, rect: Rect, count: usize, spacing: f32) -> Vec<Rect> {
        let total_spacing = spacing * (count - 1) as f32;
        let item_w = (rect.w - total_spacing) / count as f32;
        (0..count)
            .map(|i| {
                Rect::new(
                    rect.x + i as f32 * (item_w + spacing),
                    rect.y,
                    item_w,
                    rect.h,
                )
            })
            .collect()
    }

    pub fn layout_vertical(&self, rect: Rect, count: usize, spacing: f32) -> Vec<Rect> {
        let total_spacing = spacing * (count - 1) as f32;
        let item_h = (rect.h - total_spacing) / count as f32;
        (0..count)
            .map(|i| {
                Rect::new(
                    rect.x,
                    rect.y + i as f32 * (item_h + spacing),
                    rect.w,
                    item_h,
                )
            })
            .collect()
    }
}

/// Helper for building widget IDs from paths.
pub fn widget_id(path: &str) -> WidgetId {
    WidgetId::from_str(path)
}
