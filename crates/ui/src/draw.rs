//! Screen-space draw list and text rendering.
//!
//! The list stores backend-neutral triangles in physical pixels (top-left
//! origin). Text is produced by the `text` crate and packed into the same
//! vertex buffer as solid rectangles; the UI shaders use `uv == (0, 0)` to
//! identify solid geometry.

use glam::Vec2;
use text::{AtlasUpdate, TextAlign, TextStyle, TextSystem};

use crate::geometry::Rect;

/// One UI vertex consumed by every rendering backend.
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

/// Undirected rectangle in screen pixels.
#[derive(Debug)]
pub struct UiDrawList {
    width: f32,
    height: f32,
    vertices: Vec<UiVertex>,
    clip: Rect,
    text: TextSystem,
}

impl UiDrawList {
    pub fn new(viewport_width: u32, viewport_height: u32) -> Self {
        let width = viewport_width.max(1) as f32;
        let height = viewport_height.max(1) as f32;
        Self {
            width,
            height,
            vertices: Vec::new(),
            clip: Rect::new(0.0, 0.0, width, height),
            text: TextSystem::new(),
        }
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
        self.clip = Rect::new(0.0, 0.0, self.width, self.height);
    }

    /// Updates the viewport used to map screen coordinates to NDC.
    pub fn resize(&mut self, viewport_width: u32, viewport_height: u32) {
        self.width = viewport_width.max(1) as f32;
        self.height = viewport_height.max(1) as f32;
        self.clip = Rect::new(0.0, 0.0, self.width, self.height);
    }

    pub fn width(&self) -> f32 {
        self.width
    }

    pub fn height(&self) -> f32 {
        self.height
    }

    pub fn viewport(&self) -> Rect {
        Rect::new(0.0, 0.0, self.width, self.height)
    }

    pub fn vertices(&self) -> &[UiVertex] {
        &self.vertices
    }

    pub fn vertices_mut(&mut self) -> &mut Vec<UiVertex> {
        &mut self.vertices
    }

    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }

    pub fn text_system(&self) -> &TextSystem {
        &self.text
    }

    pub fn text_system_mut(&mut self) -> &mut TextSystem {
        &mut self.text
    }

    pub fn atlas_update(&self) -> Option<AtlasUpdate> {
        self.text.atlas_update()
    }

    pub fn atlas_pixels(&self) -> &[u8] {
        self.text.atlas().pixels()
    }

    pub fn atlas_size(&self) -> (u32, u32) {
        let atlas = self.text.atlas();
        (atlas.width(), atlas.height())
    }

    pub fn mark_atlas_uploaded(&mut self) {
        self.text.mark_atlas_uploaded();
    }

    /// Current clip rectangle in screen pixels.
    pub fn clip(&self) -> Rect {
        self.clip
    }

    /// Replaces the clip rectangle. Callers are expected to save and restore
    /// the previous value (the UI tree does this while walking scroll panels).
    pub fn set_clip(&mut self, clip: Rect) {
        self.clip = clip.intersect(self.viewport()).unwrap_or(Rect::ZERO);
    }

    pub fn reset_clip(&mut self) {
        self.clip = self.viewport();
    }

    /// Removes and returns the vertices appended after `start`.
    pub fn detach_from(&mut self, start: usize) -> Vec<UiVertex> {
        if start >= self.vertices.len() {
            return Vec::new();
        }
        self.vertices.split_off(start)
    }

    pub fn append_vertices(&mut self, vertices: &[UiVertex]) {
        self.vertices.extend_from_slice(vertices);
    }

    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
        if w <= 0.0 || h <= 0.0 || color[3] <= 0.0 {
            return;
        }
        self.quad(x, y, x + w, y + h, [0.0; 4], color, false);
    }

    pub fn rect_rounded(&mut self, x: f32, y: f32, w: f32, h: f32, _radius: f32, color: [f32; 4]) {
        // Rounded corners need a dedicated shader or more geometry. Keeping
        // the signature avoids churn in game code while the layout is stable.
        self.rect(x, y, w, h, color);
    }

    pub fn rect_outline(&mut self, rect: Rect, width: f32, color: [f32; 4]) {
        let width = width.max(1.0);
        self.rect(rect.x, rect.y, rect.w, width, color);
        self.rect(rect.x, rect.bottom() - width, rect.w, width, color);
        self.rect(rect.x, rect.y, width, rect.h, color);
        self.rect(rect.right() - width, rect.y, width, rect.h, color);
    }

    /// Draws a horizontal line with the given thickness.
    pub fn line_h(&mut self, x: f32, y: f32, width: f32, thickness: f32, color: [f32; 4]) {
        self.rect(x, y - thickness * 0.5, width, thickness, color);
    }

    /// Draws text with the default layout and returns the measured size.
    pub fn text(&mut self, text: &str, x: f32, y: f32, size: f32, color: [f32; 4]) -> Vec2 {
        let style = TextStyle::new(size).with_color(color);
        let layout = self.layout_text(text, &style);
        self.draw_layout(&layout, Vec2::new(x, y), color);
        Vec2::new(layout.width, layout.height)
    }

    /// Draws text with an explicit style and returns the measured size.
    pub fn text_styled(&mut self, text: &str, style: &TextStyle, origin: Vec2) -> Vec2 {
        let layout = self.layout_text(text, style);
        self.draw_layout(&layout, origin, style.color);
        Vec2::new(layout.width, layout.height)
    }

    /// Lays out and draws text, returning the full layout for callers that need
    /// line or glyph information.
    pub fn text_layout(&mut self, text: &str, style: &TextStyle, origin: Vec2) -> text::TextLayout {
        let layout = self.layout_text(text, style);
        self.draw_layout(&layout, origin, style.color);
        layout
    }

    /// Lays out text and keeps all previously emitted glyph UVs valid when the
    /// atlas grows during layout.
    fn layout_text(&mut self, text: &str, style: &TextStyle) -> text::TextLayout {
        let before = self.atlas_size();
        let layout = self.text.layout(text, style);
        let after = self.atlas_size();
        if before != after {
            self.rescale_uvs(before, after);
        }
        layout
    }

    /// Atlas growth preserves texel coordinates, so old normalized UVs only
    /// need to be rescaled to the new dimensions.
    fn rescale_uvs(&mut self, from: (u32, u32), to: (u32, u32)) {
        let scale_x = from.0 as f32 / to.0.max(1) as f32;
        let scale_y = from.1 as f32 / to.1.max(1) as f32;
        for vertex in &mut self.vertices {
            vertex.uv[0] *= scale_x;
            vertex.uv[1] *= scale_y;
        }
    }

    pub fn measure(&self, text: &str, size: f32) -> Vec2 {
        let metrics = self.text.measure(text, &TextStyle::new(size));
        Vec2::new(metrics.width, metrics.height)
    }

    pub fn measure_styled(&self, text: &str, style: &TextStyle) -> Vec2 {
        let metrics = self.text.measure(text, style);
        Vec2::new(metrics.width, metrics.height)
    }

    pub fn text_width(&self, text: &str, size: f32) -> f32 {
        self.measure(text, size).x
    }

    pub fn text_height(&self, text: &str, size: f32) -> f32 {
        self.measure(text, size).y
    }

    pub fn crosshair(&mut self) {
        let cx = self.width * 0.5;
        let cy = self.height * 0.5;
        self.rect(cx - 2.0, cy - 2.0, 4.0, 4.0, [1.0, 1.0, 1.0, 0.7]);
    }

    /// Draws a filled circle. Used by touch controls and status indicators.
    pub fn circle(&mut self, center: Vec2, radius: f32, color: [f32; 4]) {
        if radius <= 0.0 || color[3] <= 0.0 {
            return;
        }
        let segments = 28;
        let cx = self.to_ndc_x(center.x);
        let cy = self.to_ndc_y(center.y);
        for index in 0..segments {
            let a0 = index as f32 / segments as f32 * std::f32::consts::TAU;
            let a1 = (index + 1) as f32 / segments as f32 * std::f32::consts::TAU;
            let p1 = [
                self.to_ndc_x(center.x + radius * a0.cos()),
                self.to_ndc_y(center.y + radius * a0.sin()),
            ];
            let p2 = [
                self.to_ndc_x(center.x + radius * a1.cos()),
                self.to_ndc_y(center.y + radius * a1.sin()),
            ];
            self.vertices.extend_from_slice(&[
                UiVertex {
                    position: [cx, cy],
                    color,
                    uv: [0.0, 0.0],
                },
                UiVertex {
                    position: p1,
                    color,
                    uv: [0.0, 0.0],
                },
                UiVertex {
                    position: p2,
                    color,
                    uv: [0.0, 0.0],
                },
            ]);
        }
    }

    /// Draws a ring (circle outline) with the given thickness.
    pub fn circle_outline(&mut self, center: Vec2, radius: f32, thickness: f32, color: [f32; 4]) {
        if radius <= 0.0 || color[3] <= 0.0 {
            return;
        }
        let inner = (radius - thickness.max(1.0)).max(0.0);
        let segments = 28;
        for index in 0..segments {
            let a0 = index as f32 / segments as f32 * std::f32::consts::TAU;
            let a1 = (index + 1) as f32 / segments as f32 * std::f32::consts::TAU;
            let outer0 = [
                self.to_ndc_x(center.x + radius * a0.cos()),
                self.to_ndc_y(center.y + radius * a0.sin()),
            ];
            let outer1 = [
                self.to_ndc_x(center.x + radius * a1.cos()),
                self.to_ndc_y(center.y + radius * a1.sin()),
            ];
            let inner0 = [
                self.to_ndc_x(center.x + inner * a0.cos()),
                self.to_ndc_y(center.y + inner * a0.sin()),
            ];
            let inner1 = [
                self.to_ndc_x(center.x + inner * a1.cos()),
                self.to_ndc_y(center.y + inner * a1.sin()),
            ];
            self.vertices.extend_from_slice(&[
                UiVertex {
                    position: outer0,
                    color,
                    uv: [0.0, 0.0],
                },
                UiVertex {
                    position: inner0,
                    color,
                    uv: [0.0, 0.0],
                },
                UiVertex {
                    position: inner1,
                    color,
                    uv: [0.0, 0.0],
                },
                UiVertex {
                    position: outer0,
                    color,
                    uv: [0.0, 0.0],
                },
                UiVertex {
                    position: inner1,
                    color,
                    uv: [0.0, 0.0],
                },
                UiVertex {
                    position: outer1,
                    color,
                    uv: [0.0, 0.0],
                },
            ]);
        }
    }

    fn draw_layout(&mut self, layout: &text::TextLayout, origin: Vec2, color: [f32; 4]) {
        for line in &layout.lines {
            for glyph in &line.glyphs {
                self.glyph(
                    origin.x + glyph.x,
                    origin.y + glyph.y,
                    glyph.w,
                    glyph.h,
                    glyph.uv,
                    color,
                );
            }
        }
    }

    fn glyph(&mut self, x: f32, y: f32, w: f32, h: f32, uv: [f32; 4], color: [f32; 4]) {
        if w <= 0.0 || h <= 0.0 || color[3] <= 0.0 {
            return;
        }
        self.quad(x, y, x + w, y + h, uv, color, true);
    }

    /// Appends a clipped quad. For textured quads the UV rectangle is clamped
    /// proportionally; for solid quads the colour is used as-is.
    #[allow(clippy::too_many_arguments)]
    fn quad(
        &mut self,
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
        uv: [f32; 4],
        color: [f32; 4],
        textured: bool,
    ) {
        let clip = self.clip;
        let cx0 = x0.max(clip.x);
        let cy0 = y0.max(clip.y);
        let cx1 = x1.min(clip.right());
        let cy1 = y1.min(clip.bottom());
        if cx1 <= cx0 || cy1 <= cy0 {
            return;
        }

        let (u0, v0, u1, v1) = if textured {
            let width = (x1 - x0).max(f32::EPSILON);
            let height = (y1 - y0).max(f32::EPSILON);
            let left = (cx0 - x0) / width;
            let top = (cy0 - y0) / height;
            let right = (cx1 - x0) / width;
            let bottom = (cy1 - y0) / height;
            (
                uv[0] + (uv[2] - uv[0]) * left,
                uv[1] + (uv[3] - uv[1]) * top,
                uv[0] + (uv[2] - uv[0]) * right,
                uv[1] + (uv[3] - uv[1]) * bottom,
            )
        } else {
            (0.0, 0.0, 0.0, 0.0)
        };

        let nx0 = self.to_ndc_x(cx0);
        let ny0 = self.to_ndc_y(cy0);
        let nx1 = self.to_ndc_x(cx1);
        let ny1 = self.to_ndc_y(cy1);

        self.vertices.extend_from_slice(&[
            UiVertex {
                position: [nx0, ny0],
                color,
                uv: [u0, v0],
            },
            UiVertex {
                position: [nx1, ny0],
                color,
                uv: [u1, v0],
            },
            UiVertex {
                position: [nx1, ny1],
                color,
                uv: [u1, v1],
            },
            UiVertex {
                position: [nx0, ny0],
                color,
                uv: [u0, v0],
            },
            UiVertex {
                position: [nx1, ny1],
                color,
                uv: [u1, v1],
            },
            UiVertex {
                position: [nx0, ny1],
                color,
                uv: [u0, v1],
            },
        ]);
    }

    fn to_ndc_x(&self, x: f32) -> f32 {
        (x / self.width) * 2.0 - 1.0
    }

    fn to_ndc_y(&self, y: f32) -> f32 {
        (y / self.height) * 2.0 - 1.0
    }
}

/// Convenience for callers that want to build a text style with alignment.
pub fn text_style(size: f32, color: [f32; 4], align: TextAlign) -> TextStyle {
    TextStyle::new(size).with_color(color).align(align)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn earlier_glyph_uvs_follow_atlas_growth() {
        let mut draw = UiDrawList::new(1280, 720);
        let style = TextStyle::new(180.0);
        draw.text_styled("A", &style, Vec2::ZERO);
        let old_size = draw.atlas_size();
        let old_uv = draw.vertices()[0].uv;

        let text = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        draw.text_styled(text, &style, Vec2::ZERO);
        let new_size = draw.atlas_size();
        assert_ne!(old_size, new_size, "test text should grow the atlas");

        let expected_u = old_uv[0] * old_size.0 as f32 / new_size.0 as f32;
        let expected_v = old_uv[1] * old_size.1 as f32 / new_size.1 as f32;
        let rescaled = draw.vertices()[0].uv;
        assert!((rescaled[0] - expected_u).abs() < 1.0e-6);
        assert!((rescaled[1] - expected_v).abs() < 1.0e-6);
    }
}
