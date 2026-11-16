//! High-level text system: font database, glyph cache, atlas and layout.

use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::path::Path;

use crate::atlas::{AtlasUpdate, GlyphAtlas, TextureRect};
use crate::font::embedded;
use crate::font::{Font, FontData, FontError, FontId};
use crate::layout::{PositionedGlyph, TextAlign, TextLayout, TextLine, TextMetrics, TextStyle};
use crate::raster::rasterize;

const SIZE_STEPS_PER_PIXEL: f32 = 4.0;
const TAB_SPACES: f32 = 4.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct GlyphKey {
    font: FontId,
    glyph: u16,
    size_q: u32,
}

#[derive(Debug, Clone)]
struct CachedGlyph {
    slot: Option<TextureRect>,
    left: i32,
    top: i32,
    advance: f32,
}

/// Text engine shared by all UI drawing.
///
/// One instance owns the font database and the glyph atlas. The UI draw list
/// embeds a `TextSystem`, so games get TTF text with no extra setup.
#[derive(Debug)]
pub struct TextSystem {
    fonts: Vec<Font>,
    default_font: FontId,
    bold_font: Option<FontId>,
    fallbacks: Vec<FontId>,
    atlas: GlyphAtlas,
    cache: HashMap<GlyphKey, CachedGlyph>,
    missing_glyphs: HashSet<char>,
}

impl Default for TextSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl TextSystem {
    /// Creates a text system with the embedded Open Sans Regular/Bold fonts.
    pub fn new() -> Self {
        let mut system = Self::empty();
        match Font::from_bytes(FontId::DEFAULT, embedded::OPEN_SANS_REGULAR.to_vec()) {
            Ok(font) => {
                system.default_font = font.id;
                system.fonts.push(font);
            }
            Err(error) => {
                eprintln!("[text] failed to load embedded regular font: {error}");
            }
        }
        if let Ok(font) = Font::from_bytes(FontId::BOLD, embedded::OPEN_SANS_BOLD.to_vec()) {
            system.bold_font = Some(font.id);
            system.fonts.push(font);
        }
        system
    }

    /// Creates an empty system. Add at least one font before drawing text.
    pub fn empty() -> Self {
        Self {
            fonts: Vec::new(),
            default_font: FontId::DEFAULT,
            bold_font: None,
            fallbacks: Vec::new(),
            atlas: GlyphAtlas::new(),
            cache: HashMap::new(),
            missing_glyphs: HashSet::new(),
        }
    }

    /// Adds a font from memory and returns its identifier.
    pub fn add_font_bytes(&mut self, bytes: Vec<u8>) -> Result<FontId, FontError> {
        let id = FontId(
            self.fonts
                .iter()
                .map(|font| font.id.0 + 1)
                .max()
                .unwrap_or(0),
        );
        let font = Font::from_bytes(id, bytes)?;
        if self.fonts.is_empty() {
            self.default_font = id;
        }
        self.fonts.push(font);
        Ok(id)
    }

    /// Adds a font from disk and returns its identifier.
    pub fn add_font_file(&mut self, path: impl AsRef<Path>) -> Result<FontId, FontError> {
        let bytes = std::fs::read(path)?;
        self.add_font_bytes(bytes)
    }

    /// Sets the fallback font used when the default font lacks a codepoint.
    pub fn set_default_font(&mut self, id: FontId) -> bool {
        if self.font(id).is_some() {
            self.default_font = id;
            true
        } else {
            false
        }
    }

    /// Appends a font to the fallback chain.
    pub fn add_fallback(&mut self, id: FontId) {
        if self.font(id).is_some() && !self.fallbacks.contains(&id) {
            self.fallbacks.push(id);
        }
    }

    pub fn default_font(&self) -> FontId {
        self.default_font
    }

    pub fn bold_font(&self) -> Option<FontId> {
        self.bold_font
    }

    pub fn font(&self, id: FontId) -> Option<&Font> {
        self.fonts.iter().find(|font| font.id == id)
    }

    pub fn fonts(&self) -> &[Font] {
        &self.fonts
    }

    pub fn font_data(&self, id: FontId) -> Option<&FontData> {
        self.font(id).map(|font| &font.data)
    }

    /// Current atlas.
    pub fn atlas(&self) -> &GlyphAtlas {
        &self.atlas
    }

    /// Pending texture update, if glyphs were added since the last upload.
    pub fn atlas_update(&self) -> Option<AtlasUpdate> {
        self.atlas.update()
    }

    /// Called after the renderer uploaded the atlas.
    pub fn mark_atlas_uploaded(&mut self) {
        self.atlas.mark_uploaded();
    }

    /// Drops rasterized glyphs. The atlas keeps its texture; new sizes reuse
    /// free shelves.
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Measures a string without rasterizing glyphs.
    pub fn measure(&self, text: &str, style: &TextStyle) -> TextMetrics {
        let font_id = style.font.unwrap_or(self.default_font);
        let Some(font) = self.font(font_id) else {
            return TextMetrics::default();
        };
        let size = quantize_size(style.size);
        let metrics = font.metrics(size);
        let chars: Vec<char> = text.chars().collect();
        if chars.is_empty() {
            return TextMetrics::default();
        }
        let ranges = self.break_lines(&chars, style, size);
        let max_width = ranges
            .iter()
            .map(|range| self.range_width(&chars, range.clone(), style, size))
            .fold(0.0f32, f32::max);
        let line_height = line_height(style, size, &metrics);
        let height = if ranges.is_empty() {
            0.0
        } else {
            (metrics.ascent - metrics.descent) + line_height * ranges.len().saturating_sub(1) as f32
        };
        TextMetrics {
            width: max_width,
            height,
            line_count: ranges.len(),
            ascent: metrics.ascent,
            descent: metrics.descent,
        }
    }

    /// Lays out a string and inserts any new glyphs into the atlas.
    pub fn layout(&mut self, text: &str, style: &TextStyle) -> TextLayout {
        let font_id = style.font.unwrap_or(self.default_font);
        let Some(font) = self.font(font_id) else {
            return TextLayout::default();
        };
        let size = quantize_size(style.size);
        let metrics = font.metrics(size);
        let chars: Vec<char> = text.chars().collect();
        if chars.is_empty() {
            return TextLayout::default();
        }

        let ranges = self.break_lines(&chars, style, size);
        let widths: Vec<f32> = ranges
            .iter()
            .map(|range| self.range_width(&chars, range.clone(), style, size))
            .collect();
        let max_width = widths.iter().copied().fold(0.0f32, f32::max);
        let block_width = style
            .wrap_width
            .map(|width| width.max(max_width))
            .unwrap_or(max_width);
        let line_height = line_height(style, size, &metrics);
        let height = if ranges.is_empty() {
            0.0
        } else {
            (metrics.ascent - metrics.descent) + line_height * ranges.len().saturating_sub(1) as f32
        };

        let mut lines = Vec::with_capacity(ranges.len());
        let mut glyph_count = 0usize;
        // Glyph UVs are normalized against the atlas size. Rasterizing a long
        // string can grow the atlas halfway through this loop, so remember the
        // slots and compute UVs only after every glyph has been inserted.
        let mut pending_uvs: Vec<(usize, usize, TextureRect)> = Vec::new();

        for (line_index, range) in ranges.iter().enumerate() {
            let baseline = metrics.ascent + line_index as f32 * line_height;
            let align_offset = match style.align {
                TextAlign::Left => 0.0,
                TextAlign::Center => (block_width - widths[line_index]) * 0.5,
                TextAlign::Right => block_width - widths[line_index],
            };
            let mut pen_x = align_offset;
            let mut previous: Option<(FontId, u16)> = None;
            let mut glyphs = Vec::new();

            for &ch in &chars[range.clone()] {
                if ch == '\n' {
                    continue;
                }
                if ch == '\t' {
                    pen_x += self.space_advance(style, size) * TAB_SPACES;
                    previous = None;
                    continue;
                }
                if ch.is_whitespace() {
                    pen_x += self.space_advance(style, size);
                    previous = None;
                    continue;
                }

                let Some((resolved_font, glyph)) = self.resolve(ch, style.font) else {
                    self.warn_missing(ch);
                    continue;
                };

                if let Some((previous_font, previous_glyph)) = previous
                    && previous_font == resolved_font
                    && let Some(font) = self.font(resolved_font)
                {
                    let scale = size / font.data.units_per_em.max(1) as f32;
                    pen_x += font.kern(previous_glyph, glyph) * scale;
                }

                if let Some(cached) = self.glyph_slot(resolved_font, glyph, size) {
                    if let Some(slot) = cached.slot {
                        pending_uvs.push((lines.len(), glyphs.len(), slot));
                        glyphs.push(PositionedGlyph {
                            font: resolved_font,
                            glyph,
                            x: pen_x + cached.left as f32,
                            y: baseline - cached.top as f32,
                            w: slot.w as f32,
                            h: slot.h as f32,
                            uv: [0.0; 4],
                        });
                        glyph_count += 1;
                    }
                    pen_x += cached.advance;
                }
                previous = Some((resolved_font, glyph));
            }

            lines.push(TextLine {
                width: widths[line_index],
                baseline,
                glyphs,
            });
        }

        for (line_index, glyph_index, slot) in pending_uvs {
            lines[line_index].glyphs[glyph_index].uv = self.uv(slot);
        }

        TextLayout {
            width: block_width,
            height,
            lines,
            glyph_count,
        }
    }

    fn resolve(&self, ch: char, preferred: Option<FontId>) -> Option<(FontId, u16)> {
        let preferred = preferred.unwrap_or(self.default_font);
        if let Some(font) = self.font(preferred)
            && let Some(glyph) = font.glyph_index(ch)
        {
            return Some((font.id, glyph));
        }
        for &fallback in &self.fallbacks {
            if fallback == preferred {
                continue;
            }
            if let Some(font) = self.font(fallback)
                && let Some(glyph) = font.glyph_index(ch)
            {
                return Some((font.id, glyph));
            }
        }
        if preferred != self.default_font
            && let Some(font) = self.font(self.default_font)
            && let Some(glyph) = font.glyph_index(ch)
        {
            return Some((font.id, glyph));
        }
        None
    }

    fn space_advance(&self, style: &TextStyle, size: f32) -> f32 {
        let font_id = style.font.unwrap_or(self.default_font);
        if let Some(font) = self.font(font_id) {
            let glyph = font.glyph_index(' ').unwrap_or(0);
            let scale = size / font.data.units_per_em.max(1) as f32;
            let advance = font.advance(glyph) * scale;
            if advance > 0.0 {
                return advance;
            }
        }
        size * 0.25
    }

    fn range_width(
        &self,
        chars: &[char],
        range: Range<usize>,
        style: &TextStyle,
        size: f32,
    ) -> f32 {
        let mut width = 0.0;
        let mut previous: Option<(FontId, u16)> = None;
        for &ch in &chars[range] {
            if ch == '\t' {
                width += self.space_advance(style, size) * TAB_SPACES;
                previous = None;
                continue;
            }
            if ch.is_whitespace() {
                width += self.space_advance(style, size);
                previous = None;
                continue;
            }
            let Some((font_id, glyph)) = self.resolve(ch, style.font) else {
                continue;
            };
            if let Some((previous_font, previous_glyph)) = previous
                && previous_font == font_id
                && let Some(font) = self.font(font_id)
            {
                let scale = size / font.data.units_per_em.max(1) as f32;
                width += font.kern(previous_glyph, glyph) * scale;
            }
            if let Some(font) = self.font(font_id) {
                let scale = size / font.data.units_per_em.max(1) as f32;
                width += font.advance(glyph) * scale;
            }
            previous = Some((font_id, glyph));
        }
        width
    }

    fn char_width(&self, ch: char, style: &TextStyle, size: f32) -> f32 {
        if ch == '\t' {
            return self.space_advance(style, size) * TAB_SPACES;
        }
        if ch.is_whitespace() {
            return self.space_advance(style, size);
        }
        let Some((font_id, glyph)) = self.resolve(ch, style.font) else {
            return 0.0;
        };
        let Some(font) = self.font(font_id) else {
            return 0.0;
        };
        font.advance(glyph) * (size / font.data.units_per_em.max(1) as f32)
    }

    fn break_lines(&self, chars: &[char], style: &TextStyle, size: f32) -> Vec<Range<usize>> {
        let Some(wrap) = style.wrap_width else {
            let mut lines = Vec::new();
            let mut start = 0usize;
            for (index, &ch) in chars.iter().enumerate() {
                if ch == '\n' {
                    lines.push(start..index);
                    start = index + 1;
                }
            }
            lines.push(start..chars.len());
            return lines;
        };

        let wrap = wrap.max(1.0);
        let mut lines = Vec::new();
        let mut start = 0usize;
        for (index, &ch) in chars.iter().enumerate() {
            if ch == '\n' {
                self.break_paragraph(chars, start, index, wrap, style, size, &mut lines);
                start = index + 1;
            }
        }
        self.break_paragraph(chars, start, chars.len(), wrap, style, size, &mut lines);
        if lines.is_empty() {
            lines.push(0..0);
        }
        lines
    }

    #[allow(clippy::too_many_arguments)]
    fn break_paragraph(
        &self,
        chars: &[char],
        start: usize,
        end: usize,
        wrap: f32,
        style: &TextStyle,
        size: f32,
        lines: &mut Vec<Range<usize>>,
    ) {
        if start == end {
            lines.push(start..end);
            return;
        }

        let mut line_start = start;
        let mut line_width = 0.0f32;
        let mut index = start;

        while index < end {
            while index < end && chars[index].is_whitespace() && chars[index] != '\n' {
                index += 1;
            }
            if line_width == 0.0 {
                line_start = index;
            }
            if index >= end {
                break;
            }

            let word_start = index;
            while index < end && !chars[index].is_whitespace() {
                index += 1;
            }
            let word_end = index;
            let word_width = self.range_width(chars, word_start..word_end, style, size);

            let space_start = index;
            while index < end && chars[index].is_whitespace() && chars[index] != '\n' {
                index += 1;
            }
            let space_width = self.range_width(chars, space_start..index, style, size);

            if line_width > 0.0 && line_width + space_width + word_width > wrap {
                lines.push(line_start..word_start);
                line_start = word_start;
                line_width = 0.0;
            }

            if line_width == 0.0 && word_width > wrap {
                // A single word longer than the wrap width: split by character.
                let mut chunk_start = word_start;
                let mut chunk_width = 0.0f32;
                for (offset, &ch) in chars.iter().enumerate().take(word_end).skip(word_start) {
                    let width = self.char_width(ch, style, size);
                    if chunk_width > 0.0 && chunk_width + width > wrap {
                        lines.push(chunk_start..offset);
                        chunk_start = offset;
                        chunk_width = 0.0;
                    }
                    chunk_width += width;
                }
                line_start = chunk_start;
                line_width = chunk_width + space_width;
                continue;
            }

            if line_width > 0.0 {
                line_width += space_width;
            }
            line_width += word_width;
        }

        if line_start < end {
            let mut trimmed_end = end;
            while trimmed_end > line_start && chars[trimmed_end - 1].is_whitespace() {
                trimmed_end -= 1;
            }
            lines.push(line_start..trimmed_end);
        } else if lines.last().is_none_or(|line| line.end < end) {
            lines.push(start..start);
        }
    }

    fn glyph_slot(&mut self, font_id: FontId, glyph: u16, size: f32) -> Option<CachedGlyph> {
        let size_q = (size * SIZE_STEPS_PER_PIXEL).round() as u32;
        let key = GlyphKey {
            font: font_id,
            glyph,
            size_q,
        };
        if let Some(cached) = self.cache.get(&key) {
            return Some(cached.clone());
        }

        let font = self.font(font_id)?;
        let units = font.data.units_per_em.max(1) as f32;
        let scale = size / units;
        let advance = font.advance(glyph) * scale;
        let outline = font.outline(glyph).unwrap_or_default();
        let raster = rasterize(&outline, scale);
        let slot = if raster.is_empty() {
            None
        } else {
            self.atlas
                .insert(raster.width, raster.height, &raster.coverage)
        };
        let cached = CachedGlyph {
            slot,
            left: raster.left,
            top: raster.top,
            advance,
        };
        self.cache.insert(key, cached.clone());
        Some(cached)
    }

    fn uv(&self, slot: TextureRect) -> [f32; 4] {
        let width = self.atlas.width() as f32;
        let height = self.atlas.height() as f32;
        [
            slot.x as f32 / width,
            slot.y as f32 / height,
            (slot.x + slot.w) as f32 / width,
            (slot.y + slot.h) as f32 / height,
        ]
    }

    fn warn_missing(&mut self, ch: char) {
        if self.missing_glyphs.insert(ch) {
            eprintln!(
                "[text] no glyph for U+{:04X} ({ch:?}) in the default font or fallbacks",
                ch as u32
            );
        }
    }
}

fn quantize_size(size: f32) -> f32 {
    (size.clamp(1.0, 512.0) * SIZE_STEPS_PER_PIXEL).round() / SIZE_STEPS_PER_PIXEL
}

fn line_height(style: &TextStyle, size: f32, metrics: &crate::font::FontMetrics) -> f32 {
    if style.line_height > 0.0 {
        (size * style.line_height).max(size)
    } else {
        metrics.line_height.max(size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_embedded_font_and_cyrillic() {
        let system = TextSystem::new();
        let font = system.font(FontId::DEFAULT).expect("embedded font");
        assert!(font.data.units_per_em > 0);
        assert!(font.glyph_index('A').is_some());
        assert!(font.glyph_index('Я').is_some());
        assert!(font.glyph_index('ё').is_some());

        let glyph = font.glyph_index('A').unwrap();
        let outline = font.outline(glyph).unwrap();
        assert!(!outline.contours.is_empty());
        assert!(outline.bounds.is_some());
    }

    #[test]
    fn rasterizes_ascii_and_cyrillic() {
        let mut system = TextSystem::new();
        let layout = system.layout("Hello, мир! Ёж", &TextStyle::new(24.0));
        assert!(layout.glyph_count >= 10, "{layout:?}");
        assert!(layout.width > 0.0);
        assert!(layout.height > 0.0);
        assert!(system.atlas_update().is_some());
        let atlas = system.atlas();
        assert_eq!(
            atlas.width(),
            atlas.height().max(atlas.width()).min(atlas.width() * 2)
        );
    }

    #[test]
    fn measurement_matches_layout() {
        let mut system = TextSystem::new();
        let style = TextStyle::new(18.0);
        let metrics = system.measure("Soldatik3D", &style);
        let layout = system.layout("Soldatik3D", &style);
        assert!((metrics.width - layout.width).abs() < 0.01);
        assert!((metrics.height - layout.height).abs() < 0.01);
        assert_eq!(metrics.line_count, layout.lines.len());
    }

    #[test]
    fn wraps_long_lines() {
        let system = TextSystem::new();
        let style = TextStyle::new(16.0).wrap(80.0);
        let metrics = system.measure("one two three four five six", &style);
        assert!(metrics.line_count > 1);
        assert!(metrics.width <= 80.5);
    }

    #[test]
    fn glyph_uvs_use_the_final_atlas_size_after_growth() {
        let mut system = TextSystem::new();
        let style = TextStyle::new(180.0);
        let text = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        let first = system.layout(text, &style);
        let grown = {
            let atlas = system.atlas();
            (atlas.width(), atlas.height())
        };
        assert!(
            grown.0 > 512 || grown.1 > 512,
            "test text should force atlas growth, got {grown:?}"
        );

        for line in &first.lines {
            for glyph in &line.glyphs {
                assert!(
                    glyph.uv[0] >= 0.0
                        && glyph.uv[1] >= 0.0
                        && glyph.uv[2] <= 1.0
                        && glyph.uv[3] <= 1.0,
                    "invalid UV {:?}",
                    glyph.uv
                );
            }
        }

        // With every glyph cached, a second layout cannot grow the atlas. Its
        // UVs must match the first layout's UVs if normalization used the
        // final size instead of an intermediate one.
        let again = system.layout(text, &style);
        assert_eq!(first.lines.len(), again.lines.len());
        for (first_line, again_line) in first.lines.iter().zip(&again.lines) {
            assert_eq!(first_line.glyphs.len(), again_line.glyphs.len());
            for (first_glyph, again_glyph) in first_line.glyphs.iter().zip(&again_line.glyphs) {
                assert_eq!(first_glyph.uv, again_glyph.uv);
            }
        }
    }

    #[test]
    fn aligns_center() {
        let mut system = TextSystem::new();
        let style = TextStyle::new(20.0).align(TextAlign::Center);
        let layout = system.layout("abc", &style);
        let line = &layout.lines[0];
        assert!(line.glyphs[0].x >= 0.0);
        assert!(line.width <= layout.width);
    }
}
