//! Growable R8 glyph atlas with shelf packing and dirty-region tracking.

/// A rectangle in atlas texels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextureRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// Metadata for uploading the atlas to the GPU.
#[derive(Debug, Clone)]
pub struct AtlasUpdate {
    pub width: u32,
    pub height: u32,
    /// The whole texture must be recreated/reuploaded (first use or resize).
    pub full: bool,
    /// Changed regions when `full` is false.
    pub dirty: Vec<TextureRect>,
}

#[derive(Debug, Clone, Copy)]
struct Shelf {
    y: u32,
    height: u32,
    x: u32,
}

/// Single-page R8 coverage atlas.
///
/// The atlas starts small and grows by doubling when needed. Glyph slots keep
/// texel coordinates, so growing the texture does not invalidate cached
/// glyphs; the next frame simply re-normalizes UVs with the new size.
#[derive(Debug)]
pub struct GlyphAtlas {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
    shelves: Vec<Shelf>,
    used_height: u32,
    dirty: Vec<TextureRect>,
    full_update: bool,
    warned_full: bool,
}

impl Default for GlyphAtlas {
    fn default() -> Self {
        Self::new()
    }
}

impl GlyphAtlas {
    const INITIAL_SIZE: u32 = 512;
    const MAX_WIDTH: u32 = 2048;
    const MAX_HEIGHT: u32 = 4096;
    const PADDING: u32 = 2;

    pub fn new() -> Self {
        let size = Self::INITIAL_SIZE;
        let mut pixels = vec![0u8; (size * size) as usize];
        // Texel (0, 0) is opaque white. UI shaders use UV (0, 0) for solid
        // rectangles and skip sampling, but keeping a white texel makes the
        // atlas safe for custom shaders too.
        pixels[0] = 255;
        Self {
            width: size,
            height: size,
            pixels,
            shelves: Vec::new(),
            used_height: 1,
            dirty: vec![TextureRect {
                x: 0,
                y: 0,
                w: 1,
                h: 1,
            }],
            full_update: true,
            warned_full: false,
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    pub fn update(&self) -> Option<AtlasUpdate> {
        if !self.full_update && self.dirty.is_empty() {
            return None;
        }
        Some(AtlasUpdate {
            width: self.width,
            height: self.height,
            full: self.full_update,
            dirty: self.dirty.clone(),
        })
    }

    /// Called by the renderer bridge after a successful GPU upload.
    pub fn mark_uploaded(&mut self) {
        self.dirty.clear();
        self.full_update = false;
    }

    /// Allocates a slot and copies coverage into the atlas.
    pub fn insert(&mut self, width: u32, height: u32, coverage: &[u8]) -> Option<TextureRect> {
        if width == 0 || height == 0 || coverage.len() < (width * height) as usize {
            return None;
        }
        let Some(rect) = self.allocate(width, height) else {
            if !self.warned_full {
                self.warned_full = true;
                eprintln!(
                    "[text] glyph atlas is full ({}x{}); further glyphs are skipped",
                    self.width, self.height
                );
            }
            return None;
        };

        for row in 0..height {
            let source = &coverage[(row * width) as usize..((row + 1) * width) as usize];
            let target = ((rect.y + row) * self.width + rect.x) as usize;
            self.pixels[target..target + width as usize].copy_from_slice(source);
        }
        self.dirty.push(rect);
        Some(rect)
    }

    fn allocate(&mut self, width: u32, height: u32) -> Option<TextureRect> {
        let padded_width = width + Self::PADDING;
        let padded_height = height + Self::PADDING;
        if padded_width > self.width {
            let new_width = self
                .width
                .max(padded_width)
                .next_power_of_two()
                .min(Self::MAX_WIDTH);
            if new_width < padded_width {
                return None;
            }
            self.resize(new_width, self.height);
        }

        for shelf in &mut self.shelves {
            if shelf.height >= padded_height && shelf.x + padded_width <= self.width {
                let rect = TextureRect {
                    x: shelf.x,
                    y: shelf.y,
                    w: width,
                    h: height,
                };
                shelf.x += padded_width;
                return Some(rect);
            }
        }

        if self.used_height + padded_height > self.height {
            let mut new_height = self.height;
            while new_height < self.used_height + padded_height && new_height < Self::MAX_HEIGHT {
                new_height *= 2;
            }
            if new_height < self.used_height + padded_height {
                return None;
            }
            self.resize(self.width, new_height);
        }

        let y = self.used_height;
        self.shelves.push(Shelf {
            y,
            height: padded_height,
            x: padded_width,
        });
        self.used_height += padded_height;
        Some(TextureRect {
            x: Self::PADDING,
            y,
            w: width,
            h: height,
        })
    }

    fn resize(&mut self, new_width: u32, new_height: u32) {
        if new_width == self.width && new_height == self.height {
            return;
        }
        let mut pixels = vec![0u8; (new_width * new_height) as usize];
        let copy_width = self.width.min(new_width);
        let copy_height = self.height.min(new_height);
        for row in 0..copy_height {
            let source = (row * self.width) as usize;
            let target = (row * new_width) as usize;
            pixels[target..target + copy_width as usize]
                .copy_from_slice(&self.pixels[source..source + copy_width as usize]);
        }
        self.pixels = pixels;
        self.width = new_width;
        self.height = new_height;
        self.full_update = true;
        self.dirty.clear();
    }
}
