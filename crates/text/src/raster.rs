//! Anti-aliased scanline rasterizer for quadratic TrueType outlines.
//!
//! Curves are flattened adaptively, then coverage is accumulated with 8
//! vertical sub-samples per pixel and exact horizontal span coverage. This
//! gives clean small UI text without a font-rasterization dependency.

use crate::font::{Outline, OutlinePoint};

/// A rasterized glyph bitmap in R8 coverage form.
#[derive(Debug, Clone, Default)]
pub struct RasterImage {
    pub width: u32,
    pub height: u32,
    /// X offset from the pen position to the bitmap's left edge.
    pub left: i32,
    /// Y offset from the baseline up to the bitmap's top edge.
    pub top: i32,
    /// `width * height` coverage values in row-major order.
    pub coverage: Vec<u8>,
}

impl RasterImage {
    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0 || self.coverage.is_empty()
    }
}

const VERTICAL_SAMPLES: u32 = 8;
const MAX_BITMAP_DIMENSION: u32 = 4096;
const MAX_BITMAP_PIXELS: u64 = 16 * 1024 * 1024;

#[derive(Clone, Copy)]
struct Point {
    x: f32,
    y: f32,
}

#[derive(Clone, Copy)]
struct Edge {
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
}

/// Rasterizes an outline at `scale` pixels per font unit.
///
/// `scale` is normally `font_size_px / units_per_em`.
pub fn rasterize(outline: &Outline, scale: f32) -> RasterImage {
    if outline.contours.is_empty() || !scale.is_finite() || scale <= 0.0 {
        return RasterImage::default();
    }

    let mut edges = Vec::new();
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;

    for contour in &outline.contours {
        flatten_contour(contour, scale, &mut edges);
    }
    for edge in &edges {
        min_x = min_x.min(edge.x0).min(edge.x1);
        min_y = min_y.min(edge.y0).min(edge.y1);
        max_x = max_x.max(edge.x0).max(edge.x1);
        max_y = max_y.max(edge.y0).max(edge.y1);
    }

    if !min_x.is_finite() || !min_y.is_finite() || !max_x.is_finite() || !max_y.is_finite() {
        return RasterImage::default();
    }

    let left = min_x.floor() as i32;
    let right = max_x.ceil() as i32;
    // Bitmap rows grow downward, font coordinates grow upward.
    let top = max_y.ceil() as i32;
    let bottom = min_y.floor() as i32;
    let width = (right - left).max(0) as u32;
    let height = (top - bottom).max(0) as u32;

    if width == 0 || height == 0 {
        return RasterImage::default();
    }
    if width > MAX_BITMAP_DIMENSION
        || height > MAX_BITMAP_DIMENSION
        || u64::from(width) * u64::from(height) > MAX_BITMAP_PIXELS
    {
        return RasterImage::default();
    }

    let mut coverage = vec![0u8; (width * height) as usize];
    let mut row_coverage = vec![0.0f32; width as usize];
    let mut crossings: Vec<(f32, i32)> = Vec::with_capacity(16);
    let weight = 1.0 / VERTICAL_SAMPLES as f32;

    for row in 0..height {
        row_coverage.fill(0.0);
        for sample in 0..VERTICAL_SAMPLES {
            let sample_y = top as f32 - (row as f32 + (sample as f32 + 0.5) * weight);
            crossings.clear();

            for edge in &edges {
                let y0_below = edge.y0 <= sample_y;
                let y1_below = edge.y1 <= sample_y;
                if y0_below == y1_below {
                    continue;
                }
                let dy = edge.y1 - edge.y0;
                if dy.abs() < f32::EPSILON {
                    continue;
                }
                let t = (sample_y - edge.y0) / dy;
                let x = edge.x0 + t * (edge.x1 - edge.x0);
                crossings.push((x, if edge.y1 > edge.y0 { 1 } else { -1 }));
            }

            if crossings.is_empty() {
                continue;
            }
            crossings.sort_unstable_by(|a, b| a.0.total_cmp(&b.0));

            let mut winding = 0i32;
            let mut span_start = 0.0f32;
            for &(x, direction) in &crossings {
                let was_inside = winding != 0;
                winding += direction;
                let is_inside = winding != 0;
                if !was_inside && is_inside {
                    span_start = x;
                } else if was_inside && !is_inside {
                    add_span(&mut row_coverage, span_start, x, left as f32, weight);
                }
            }
        }

        let row_offset = row as usize * width as usize;
        for (index, value) in row_coverage.iter().enumerate() {
            coverage[row_offset + index] = (value.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    }

    RasterImage {
        width,
        height,
        left,
        top,
        coverage,
    }
}

fn add_span(row: &mut [f32], start: f32, end: f32, left: f32, weight: f32) {
    let mut x0 = start - left;
    let mut x1 = end - left;
    if x1 < x0 {
        std::mem::swap(&mut x0, &mut x1);
    }
    let width = row.len() as f32;
    x0 = x0.clamp(0.0, width);
    x1 = x1.clamp(0.0, width);
    if x1 <= x0 {
        return;
    }

    let first = x0.floor() as usize;
    let last = (x1.ceil() as usize).min(row.len());
    for (index, cell) in row.iter_mut().enumerate().take(last).skip(first) {
        let cell_start = index as f32;
        let cell_end = cell_start + 1.0;
        let overlap = x1.min(cell_end) - x0.max(cell_start);
        if overlap > 0.0 {
            *cell += overlap * weight;
        }
    }
}

fn flatten_contour(contour: &[OutlinePoint], scale: f32, edges: &mut Vec<Edge>) {
    if contour.is_empty() {
        return;
    }

    let points: Vec<(Point, bool)> = contour
        .iter()
        .map(|point| {
            (
                Point {
                    x: point.x * scale,
                    y: point.y * scale,
                },
                point.on_curve,
            )
        })
        .collect();

    // Rotate the contour so it begins at an on-curve point whenever possible.
    let start_index = points.iter().position(|(_, on_curve)| *on_curve);
    let (start, ordered): (Point, Vec<(Point, bool)>) = if let Some(start_index) = start_index {
        let mut ordered = Vec::with_capacity(points.len());
        for offset in 0..points.len() {
            ordered.push(points[(start_index + offset) % points.len()]);
        }
        (points[start_index].0, ordered)
    } else {
        // All points are off-curve: the implied start is the midpoint between
        // the last and first control points.
        let first = points[0].0;
        let last = points[points.len() - 1].0;
        (
            Point {
                x: (first.x + last.x) * 0.5,
                y: (first.y + last.y) * 0.5,
            },
            points,
        )
    };

    let mut current = start;
    let mut control: Option<Point> = None;

    for &(point, on_curve) in &ordered {
        if on_curve {
            if let Some(control) = control.take() {
                flatten_quad(current, control, point, edges);
            } else {
                edges.push(Edge {
                    x0: current.x,
                    y0: current.y,
                    x1: point.x,
                    y1: point.y,
                });
            }
            current = point;
        } else if let Some(previous_control) = control.replace(point) {
            let implied = Point {
                x: (previous_control.x + point.x) * 0.5,
                y: (previous_control.y + point.y) * 0.5,
            };
            flatten_quad(current, previous_control, implied, edges);
            current = implied;
        }
    }

    if let Some(control) = control {
        flatten_quad(current, control, start, edges);
    } else if (current.x - start.x).abs() > f32::EPSILON
        || (current.y - start.y).abs() > f32::EPSILON
    {
        edges.push(Edge {
            x0: current.x,
            y0: current.y,
            x1: start.x,
            y1: start.y,
        });
    }
}

fn flatten_quad(from: Point, control: Point, to: Point, edges: &mut Vec<Edge>) {
    // The second difference is a cheap proxy for curvature. More segments are
    // used for large glyphs, fewer for small UI text.
    let dx = from.x - 2.0 * control.x + to.x;
    let dy = from.y - 2.0 * control.y + to.y;
    let curvature = (dx * dx + dy * dy).sqrt();
    let segments = ((curvature * 0.5).sqrt().ceil() as usize).clamp(1, 32);

    let mut previous = from;
    for step in 1..=segments {
        let t = step as f32 / segments as f32;
        let inv = 1.0 - t;
        let point = Point {
            x: inv * inv * from.x + 2.0 * inv * t * control.x + t * t * to.x,
            y: inv * inv * from.y + 2.0 * inv * t * control.y + t * t * to.y,
        };
        edges.push(Edge {
            x0: previous.x,
            y0: previous.y,
            x1: point.x,
            y1: point.y,
        });
        previous = point;
    }
}
