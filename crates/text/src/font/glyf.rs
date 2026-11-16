//! `glyf`/`loca` outline extraction, including composite glyphs.

use super::FontError;
use super::sfnt::{FontData, read_i16, read_u8, read_u16};

/// One point of a quadratic TrueType contour.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OutlinePoint {
    pub x: f32,
    pub y: f32,
    pub on_curve: bool,
}

/// A single closed contour.
pub type OutlineContour = Vec<OutlinePoint>;

/// Axis-aligned bounds in font units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OutlineBounds {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}

/// Vector outline of a glyph in font units (Y points up).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Outline {
    pub contours: Vec<OutlineContour>,
    pub bounds: Option<OutlineBounds>,
}

impl Outline {
    pub fn is_empty(&self) -> bool {
        self.contours.iter().all(Vec::is_empty)
    }

    pub(crate) fn push_contour(&mut self, contour: OutlineContour) {
        if !contour.is_empty() {
            self.contours.push(contour);
        }
    }

    pub(crate) fn recompute_bounds(&mut self) {
        let mut bounds: Option<OutlineBounds> = None;
        for point in self.contours.iter().flatten() {
            bounds = Some(match bounds {
                None => OutlineBounds {
                    min_x: point.x,
                    min_y: point.y,
                    max_x: point.x,
                    max_y: point.y,
                },
                Some(mut bounds) => {
                    bounds.min_x = bounds.min_x.min(point.x);
                    bounds.min_y = bounds.min_y.min(point.y);
                    bounds.max_x = bounds.max_x.max(point.x);
                    bounds.max_y = bounds.max_y.max(point.y);
                    bounds
                }
            });
        }
        self.bounds = bounds;
    }

    pub(crate) fn transform(&mut self, a: f32, b: f32, c: f32, d: f32, dx: f32, dy: f32) {
        for point in self.contours.iter_mut().flatten() {
            let x = point.x;
            let y = point.y;
            point.x = a * x + c * y + dx;
            point.y = b * x + d * y + dy;
        }
        self.recompute_bounds();
    }
}

const ON_CURVE_POINT: u8 = 0x01;
const X_SHORT_VECTOR: u8 = 0x02;
const Y_SHORT_VECTOR: u8 = 0x04;
const REPEAT_FLAG: u8 = 0x08;
const X_IS_SAME_OR_POSITIVE: u8 = 0x10;
const Y_IS_SAME_OR_POSITIVE: u8 = 0x20;

const ARG_1_AND_2_ARE_WORDS: u16 = 0x0001;
const ARGS_ARE_XY_VALUES: u16 = 0x0002;
const WE_HAVE_A_SCALE: u16 = 0x0008;
const MORE_COMPONENTS: u16 = 0x0020;
const WE_HAVE_AN_X_AND_Y_SCALE: u16 = 0x0040;
const WE_HAVE_A_TWO_BY_TWO: u16 = 0x0080;
const SCALED_COMPONENT_OFFSET: u16 = 0x0800;

/// Parses one glyph. `depth` protects against malicious recursive composites.
pub(crate) fn parse_glyph(font: &FontData, data: &[u8], depth: u8) -> Result<Outline, FontError> {
    if depth > 8 {
        return Err(FontError::invalid("composite glyph nesting is too deep"));
    }
    if data.len() < 10 {
        // A zero-length or truncated glyph is treated as blank rather than a
        // hard failure: fonts in the wild contain empty glyph records.
        return Ok(Outline::default());
    }

    let number_of_contours = read_i16(data, 0)?;
    if number_of_contours == 0 {
        return Ok(Outline::default());
    }
    if number_of_contours > 0 {
        return parse_simple(data, number_of_contours as usize);
    }
    if number_of_contours != -1 {
        return Err(FontError::invalid(format!(
            "unsupported numberOfContours {number_of_contours}"
        )));
    }
    parse_composite(font, data, depth)
}

fn parse_simple(data: &[u8], contour_count: usize) -> Result<Outline, FontError> {
    let end_points_offset = 10;
    let instructions_offset = end_points_offset + contour_count * 2;
    if instructions_offset + 2 > data.len() {
        return Err(FontError::invalid("simple glyph end points are truncated"));
    }

    let mut end_points = Vec::with_capacity(contour_count);
    for index in 0..contour_count {
        end_points.push(read_u16(data, end_points_offset + index * 2)? as usize);
    }
    if end_points.is_empty() {
        return Ok(Outline::default());
    }
    if end_points.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(FontError::invalid(
            "simple glyph end points are not increasing",
        ));
    }

    let instruction_length = read_u16(data, instructions_offset)? as usize;
    let flags_offset = instructions_offset + 2 + instruction_length;
    if flags_offset > data.len() {
        return Err(FontError::invalid(
            "simple glyph instructions are truncated",
        ));
    }

    let point_count = end_points.last().copied().unwrap_or(0) + 1;
    let mut flags = Vec::with_capacity(point_count);
    let mut cursor = flags_offset;
    while flags.len() < point_count {
        let flag = read_u8(data, cursor)?;
        cursor += 1;
        flags.push(flag);
        if flag & REPEAT_FLAG != 0 {
            let repeat = read_u8(data, cursor)? as usize;
            cursor += 1;
            for _ in 0..repeat {
                if flags.len() >= point_count {
                    break;
                }
                flags.push(flag);
            }
        }
    }

    let mut x_coords = Vec::with_capacity(point_count);
    let mut x = 0i32;
    for flag in &flags {
        let delta = if flag & X_SHORT_VECTOR != 0 {
            let value = read_u8(data, cursor)? as i32;
            cursor += 1;
            if flag & X_IS_SAME_OR_POSITIVE != 0 {
                value
            } else {
                -value
            }
        } else if flag & X_IS_SAME_OR_POSITIVE != 0 {
            0
        } else {
            let value = read_i16(data, cursor)? as i32;
            cursor += 2;
            value
        };
        x += delta;
        x_coords.push(x as f32);
    }

    let mut y_coords = Vec::with_capacity(point_count);
    let mut y = 0i32;
    for flag in &flags {
        let delta = if flag & Y_SHORT_VECTOR != 0 {
            let value = read_u8(data, cursor)? as i32;
            cursor += 1;
            if flag & Y_IS_SAME_OR_POSITIVE != 0 {
                value
            } else {
                -value
            }
        } else if flag & Y_IS_SAME_OR_POSITIVE != 0 {
            0
        } else {
            let value = read_i16(data, cursor)? as i32;
            cursor += 2;
            value
        };
        y += delta;
        y_coords.push(y as f32);
    }

    let mut outline = Outline::default();
    let mut start = 0usize;
    for end in end_points {
        let end = end.min(point_count.saturating_sub(1));
        if end < start {
            continue;
        }
        let contour: OutlineContour = (start..=end)
            .map(|index| OutlinePoint {
                x: x_coords[index],
                y: y_coords[index],
                on_curve: flags[index] & ON_CURVE_POINT != 0,
            })
            .collect();
        outline.push_contour(contour);
        start = end + 1;
    }
    outline.recompute_bounds();
    Ok(outline)
}

fn parse_composite(font: &FontData, data: &[u8], depth: u8) -> Result<Outline, FontError> {
    let mut outline = Outline::default();
    let mut cursor = 10usize;

    loop {
        if cursor + 4 > data.len() {
            return Err(FontError::invalid("composite glyph header is truncated"));
        }
        let flags = read_u16(data, cursor)?;
        let component_glyph = read_u16(data, cursor + 2)?;
        cursor += 4;

        let (arg1, arg2) = if flags & ARG_1_AND_2_ARE_WORDS != 0 {
            let first = read_i16(data, cursor)? as f32;
            let second = read_i16(data, cursor + 2)? as f32;
            cursor += 4;
            (first, second)
        } else {
            let first = read_u8(data, cursor)? as i8 as f32;
            let second = read_u8(data, cursor + 1)? as i8 as f32;
            cursor += 2;
            (first, second)
        };

        let (mut a, mut b, mut c, mut d) = (1.0f32, 0.0f32, 0.0f32, 1.0f32);
        if flags & WE_HAVE_A_SCALE != 0 {
            let scale = read_f2dot14(data, cursor)?;
            cursor += 2;
            a = scale;
            d = scale;
        } else if flags & WE_HAVE_AN_X_AND_Y_SCALE != 0 {
            a = read_f2dot14(data, cursor)?;
            d = read_f2dot14(data, cursor + 2)?;
            cursor += 4;
        } else if flags & WE_HAVE_A_TWO_BY_TWO != 0 {
            a = read_f2dot14(data, cursor)?;
            b = read_f2dot14(data, cursor + 2)?;
            c = read_f2dot14(data, cursor + 4)?;
            d = read_f2dot14(data, cursor + 6)?;
            cursor += 8;
        }

        let Some(component_data) = font.glyph_bytes(component_glyph) else {
            if flags & MORE_COMPONENTS == 0 {
                break;
            }
            continue;
        };
        let mut component = parse_glyph(font, component_data, depth + 1)?;

        let mut dx = 0.0f32;
        let mut dy = 0.0f32;
        if flags & ARGS_ARE_XY_VALUES != 0 {
            dx = arg1;
            dy = arg2;
            if flags & SCALED_COMPONENT_OFFSET != 0 {
                let transformed_x = a * dx + c * dy;
                let transformed_y = b * dx + d * dy;
                dx = transformed_x;
                dy = transformed_y;
            }
        } else {
            // Anchor-point matching. The first argument addresses the
            // component, the second addresses the already assembled outline.
            let component_index = arg1.max(0.0) as usize;
            let parent_index = arg2.max(0.0) as usize;
            let component_point = component
                .contours
                .iter()
                .flatten()
                .nth(component_index)
                .copied();
            let parent_point = outline.contours.iter().flatten().nth(parent_index).copied();
            if let (Some(component_point), Some(parent_point)) = (component_point, parent_point) {
                let transformed_x = a * component_point.x + c * component_point.y;
                let transformed_y = b * component_point.x + d * component_point.y;
                dx = parent_point.x - transformed_x;
                dy = parent_point.y - transformed_y;
            }
        }

        component.transform(a, b, c, d, dx, dy);
        for contour in component.contours {
            outline.push_contour(contour);
        }

        if flags & MORE_COMPONENTS == 0 {
            break;
        }
    }

    outline.recompute_bounds();
    Ok(outline)
}

fn read_f2dot14(data: &[u8], offset: usize) -> Result<f32, FontError> {
    read_i16(data, offset).map(|value| value as f32 / 16384.0)
}
