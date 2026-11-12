//! Immediate-mode 2D drawing commands consumed by the engine UI pass.

const FONT_ATLAS_SIZE: u32 = 128;
const FONT_CELL_SIZE: u32 = 8;
const GLYPH_COLUMNS: u32 = 16;

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

    pub fn text(&mut self, text: &str, mut x: f32, y: f32, scale: f32, color: [f32; 4]) {
        let cell_uv = 1.0 / GLYPH_COLUMNS as f32;
        for ch in text.chars() {
            let index = ch as u32;
            if !(32..=126).contains(&index) {
                continue;
            }
            let cx = (index % GLYPH_COLUMNS) as f32;
            let cy = (index / GLYPH_COLUMNS) as f32;
            let u0 = cx * cell_uv;
            let v0 = cy * cell_uv;
            let u1 = u0 + cell_uv;
            let v1 = v0 + cell_uv;
            let glyph_width = 5.0 * scale;
            let glyph_height = 7.0 * scale;

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
            x += 6.0 * scale;
        }
    }

    pub fn text_width(text: &str, scale: f32) -> f32 {
        text.chars()
            .filter(|c| (32..=126).contains(&(*c as u32)))
            .count() as f32
            * 6.0
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

/// 128x128 RGBA font atlas. Glyphs live in 8x8 cells, characters 32..126 are
/// packed in a 16x8 grid. The glyph itself is a 5x7 bitmap.
pub fn font_atlas_rgba8() -> Vec<u8> {
    let mut pixels = vec![0u8; (FONT_ATLAS_SIZE * FONT_ATLAS_SIZE * 4) as usize];
    let size = FONT_ATLAS_SIZE;

    for code in 32u32..127 {
        let glyph = glyph_rows(code as u8 as char);
        let cell_x = (code % GLYPH_COLUMNS) * FONT_CELL_SIZE;
        let cell_y = (code / GLYPH_COLUMNS) * FONT_CELL_SIZE;

        for row in 0..7u32 {
            for col in 0..5u32 {
                if (glyph[row as usize] & (1 << (4 - col))) != 0 {
                    let px = cell_x + col;
                    let py = cell_y + row;
                    let idx = ((py * size + px) * 4) as usize;
                    pixels[idx..idx + 4].copy_from_slice(&[255, 255, 255, 255]);
                }
            }
        }
    }

    pixels
}

fn glyph_rows(ch: char) -> [u8; 7] {
    match ch {
        '0' => [
            0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
        ],
        '1' => [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        '2' => [
            0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
        ],
        '3' => [
            0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        '4' => [
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ],
        '5' => [
            0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110,
        ],
        '6' => [
            0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
        ],
        '7' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ],
        '8' => [
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ],
        '9' => [
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100,
        ],
        'A' => [
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'B' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
        ],
        'C' => [
            0b01111, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b01111,
        ],
        'D' => [
            0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
        ],
        'E' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ],
        'F' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'G' => [
            0b01111, 0b10000, 0b10000, 0b10011, 0b10001, 0b10001, 0b01111,
        ],
        'H' => [
            0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'I' => [
            0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        'J' => [
            0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100,
        ],
        'K' => [
            0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
        ],
        'L' => [
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
        ],
        'M' => [
            0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
        ],
        'N' => [
            0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
        ],
        'O' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'P' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'Q' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101,
        ],
        'R' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ],
        'S' => [
            0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        'T' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'U' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'V' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100,
        ],
        'W' => [
            0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001,
        ],
        'X' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001,
        ],
        'Y' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'Z' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111,
        ],
        ':' => [
            0b00000, 0b01100, 0b01100, 0b00000, 0b01100, 0b01100, 0b00000,
        ],
        '.' => [
            0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b01100,
        ],
        '-' => [
            0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000,
        ],
        '/' => [
            0b00001, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b10000,
        ],
        ' ' => [0; 7],
        _ => [
            0b11111, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11111,
        ],
    }
}
