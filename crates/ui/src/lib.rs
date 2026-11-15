//! Immediate-mode 2D drawing commands consumed by the engine UI pass.
//!
//! Coordinates are physical pixels with the origin in the top-left corner.
//! The draw list is backend-neutral: it produces vertices, and each renderer
//! uploads them with its own pipeline. Font atlas data lives in the `assets`
//! crate so text and glyph packing stay independent from draw commands.
//!
//! The `widget_system` module provides a comprehensive touch-friendly UI system:
//! - Hybrid immediate/retained mode: immediate API builds persistent widget tree
//! - Flexbox-style layout engine
//! - Component-based architecture (ECS-style)
//! - Hit-test + bubbling event system
//! - Built-in widgets: Button, Label, Container, Dropdown, VirtualJoystick, TouchCamera, ScrollPanel
//!
//! The `widgets` module (legacy) provides immediate-mode components:
//! buttons, toggles, sliders, progress bars, dropdowns, virtual joystick,
//! touch camera control, scroll panels, and layout helpers.

use assets::{
    FONT_ATLAS_SIZE, FONT_CELL_SIZE, FONT_GLYPH_COLUMNS, FONT_GLYPH_HEIGHT, FONT_GLYPH_WIDTH,
};

pub mod widget_system;
pub mod widgets;

// Re-export widget_system as the primary API
pub use widget_system::{
    WidgetId,
    Rect,
    LayoutProps,
    FlexDirection,
    AlignItems,
    JustifyContent,
    FlexBasis,
    ComputedLayout,
    PointerState,
    InteractionEvent,
    EventPropagation,
    Component,
    ComponentStore,
    WidgetState,
    Theme,
    WidgetNode,
    Widget,
    UpdateContext,
    LayoutConstraints,
    InputEvent,
    DrawList,
    LayoutEngine,
    EventDispatcher,
    Ui,
    UiFrame,
    // Widgets
    Button, ButtonStyle, ButtonComponent,
    Container, ContainerStyle,
    Label, TextAlignment,
    Dropdown, DropdownStyle, DropdownComponent,
    VirtualJoystick, VirtualJoystickStyle, VirtualJoystickComponent,
    TouchCamera, TouchCameraStyle, TouchCameraComponent,
    ScrollPanel, ScrollPanelStyle, ScrollPanelComponent,
    // Builders
    ButtonBuilder, ContainerBuilder, LabelBuilder, DropdownBuilder,
    VirtualJoystickBuilder, TouchCameraBuilder, ScrollPanelBuilder,
};

// Re-export legacy widgets for backward compatibility
pub use widgets::{Interaction, PointerState as LegacyPointerState, Rect as LegacyRect, Theme as LegacyTheme, UiContext, WidgetId as LegacyWidgetId, widget_id};
pub use widgets::{TouchInputProvider, TouchStateProvider};

/// Horizontal distance between two glyph origins in the built-in font.
const GLYPH_ADVANCE: f32 = FONT_GLYPH_WIDTH as f32 + 1.0;

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UiVertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
    pub uv: [f32; 2],
}

impl UiVertex {
    pub const STRIDE: u32 = std::mem::size_of::<Self>() as u32;
}

/// A screen-space draw list. Coordinates are physical pixels with the origin
/// in the top-left corner.
pub struct UiDrawList {
    width: f32,
    height: f32,
    vertices: Vec<UiVertex>,
}

impl UiDrawList {
    pub fn new(viewport_width: u32, viewport_height: u32) -> Self {
        Self {
            width: viewport_width.max(1) as f32,
            height: viewport_height.max(1) as f32,
            vertices: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
    }

    /// Updates the viewport used to map screen coordinates to NDC. Clearing
    /// the draw list is left to the caller.
    pub fn resize(&mut self, viewport_width: u32, viewport_height: u32) {
        self.width = viewport_width.max(1) as f32;
        self.height = viewport_height.max(1) as f32;
    }

    pub fn width(&self) -> f32 {
        self.width
    }

    pub fn height(&self) -> f32 {
        self.height
    }

    pub fn vertices(&self) -> &[UiVertex] {
        &self.vertices
    }

    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }

    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
        let x0 = self.to_ndc_x(x);
        let y0 = self.to_ndc_y(y);
        let x1 = self.to_ndc_x(x + w);
        let y1 = self.to_ndc_y(y + h);
        self.vertices.extend_from_slice(&[
            UiVertex {
                position: [x0, y0],
                color,
                uv: [0.0, 0.0],
            },
            UiVertex {
                position: [x1, y0],
                color,
                uv: [0.0, 0.0],
            },
            UiVertex {
                position: [x1, y1],
                color,
                uv: [0.0, 0.0],
            },
            UiVertex {
                position: [x0, y0],
                color,
                uv: [0.0, 0.0],
            },
            UiVertex {
                position: [x1, y1],
                color,
                uv: [0.0, 0.0],
            },
            UiVertex {
                position: [x0, y1],
                color,
                uv: [0.0, 0.0],
            },
        ]);
    }

    pub fn rect_rounded(&mut self, x: f32, y: f32, w: f32, h: f32, _radius: f32, color: [f32; 4]) {
        // For now, draw as regular rect. Proper rounded corners need more vertices.
        self.rect(x, y, w, h, color);
    }

    pub fn text(&mut self, text: &str, mut x: f32, y: f32, scale: f32, color: [f32; 4]) {
        let atlas_size = FONT_ATLAS_SIZE as f32;
        for ch in text.chars() {
            let index = ch as u32;
            if !(32..=126).contains(&index) {
                continue;
            }
            let cell_x = (index % FONT_GLYPH_COLUMNS) * FONT_CELL_SIZE;
            let cell_y = (index / FONT_GLYPH_COLUMNS) * FONT_CELL_SIZE;
            // Only the 5x7 glyph area of each cell holds ink; sampling the
            // whole 8x8 cell would add transparent padding and make text look
            // spaced out and fuzzy.
            let u0 = cell_x as f32 / atlas_size;
            let v0 = cell_y as f32 / atlas_size;
            let u1 = (cell_x + FONT_GLYPH_WIDTH) as f32 / atlas_size;
            let v1 = (cell_y + FONT_GLYPH_HEIGHT) as f32 / atlas_size;
            let glyph_width = FONT_GLYPH_WIDTH as f32 * scale;
            let glyph_height = FONT_GLYPH_HEIGHT as f32 * scale;

            let x0 = self.to_ndc_x(x);
            let y0 = self.to_ndc_y(y);
            let x1 = self.to_ndc_x(x + glyph_width);
            let y1 = self.to_ndc_y(y + glyph_height);

            self.vertices.extend_from_slice(&[
                UiVertex {
                    position: [x0, y0],
                    color,
                    uv: [u0, v0],
                },
                UiVertex {
                    position: [x1, y0],
                    color,
                    uv: [u1, v0],
                },
                UiVertex {
                    position: [x1, y1],
                    color,
                    uv: [u1, v1],
                },
                UiVertex {
                    position: [x0, y0],
                    color,
                    uv: [u0, v0],
                },
                UiVertex {
                    position: [x1, y1],
                    color,
                    uv: [u1, v1],
                },
                UiVertex {
                    position: [x0, y1],
                    color,
                    uv: [u0, v1],
                },
            ]);
            x += GLYPH_ADVANCE * scale;
        }
    }

    pub fn text_width(text: &str, scale: f32) -> f32 {
        text.chars()
            .filter(|c| (32..=126).contains(&(*c as u32)))
            .count() as f32
            * GLYPH_ADVANCE
            * scale
    }

    pub fn crosshair(&mut self) {
        let cx = self.width * 0.5;
        let cy = self.height * 0.5;
        self.rect(cx - 2.0, cy - 2.0, 4.0, 4.0, [1.0, 1.0, 1.0, 0.7]);
    }

    fn to_ndc_x(&self, x: f32) -> f32 {
        (x / self.width) * 2.0 - 1.0
    }

    fn to_ndc_y(&self, y: f32) -> f32 {
        (y / self.height) * 2.0 - 1.0
    }
}

// Implement DrawList trait for UiDrawList to work with widget_system
impl widget_system::DrawList for UiDrawList {
    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
        self.rect(x, y, w, h, color);
    }

    fn rect_rounded(&mut self, x: f32, y: f32, w: f32, h: f32, radius: f32, color: [f32; 4]) {
        self.rect_rounded(x, y, w, h, radius, color);
    }

    fn text(&mut self, text: &str, x: f32, y: f32, scale: f32, color: [f32; 4]) {
        self.text(text, x, y, scale, color);
    }

    fn text_width(&self, text: &str, scale: f32) -> f32 {
        Self::text_width(text, scale)
    }
}
