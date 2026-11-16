//! SFNT container parsing: table directory, metrics, `name`, `cmap` and glyph
//! outlines. Only what the engine actually needs is decoded; every offset is
//! bounds-checked before use.

use std::ops::Range;

use super::cmap::Cmap;
use super::glyf::{self, Outline};
use super::kern::KernTable;
use super::{FontError, FontMetrics};

const TAG_TTCF: u32 = 0x7474_6366; // 'ttcf'
const TAG_TRUE: u32 = 0x7472_7565; // 'true'
const TAG_OTTO: u32 = 0x4f54_544f; // 'OTTO'
const TAG_TRUETYPE: u32 = 0x0001_0000;

/// Four-byte SFNT table tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Tag(pub [u8; 4]);

#[derive(Debug, Clone, Copy)]
struct Table {
    tag: Tag,
    offset: usize,
    length: usize,
}

/// Parsed TrueType font data.
#[derive(Debug)]
pub struct FontData {
    bytes: Vec<u8>,
    tables: Vec<Table>,

    /// Design units per em from `head`.
    pub units_per_em: u16,
    /// Number of glyphs from `maxp`.
    pub num_glyphs: u16,
    /// Horizontal ascender from `hhea` / `OS/2`.
    pub ascender: i16,
    /// Horizontal descender from `hhea` / `OS/2` (negative).
    pub descender: i16,
    /// Recommended extra spacing between lines.
    pub line_gap: i16,
    /// Family name from the `name` table.
    pub family_name: String,
    /// Style name from the `name` table.
    pub style_name: String,

    number_of_h_metrics: u16,
    index_to_loc_format: i16,
    hmtx_offset: usize,
    loca_offset: usize,
    glyf_offset: usize,
    cmap: Cmap,
    kern: Option<KernTable>,
}

impl FontData {
    /// Parses a complete TTF/TrueType file.
    pub fn parse(bytes: Vec<u8>) -> Result<Self, FontError> {
        if bytes.len() < 12 {
            return Err(FontError::invalid("file is shorter than an SFNT header"));
        }

        let mut base = 0usize;
        let mut version = read_u32(&bytes, 0)?;

        // TrueType Collections store offsets to individual faces.
        if version == TAG_TTCF {
            let num_fonts = read_u32(&bytes, 8)? as usize;
            if num_fonts == 0 {
                return Err(FontError::invalid("empty TrueType collection"));
            }
            base = read_u32(&bytes, 12)? as usize;
            if base + 12 > bytes.len() {
                return Err(FontError::invalid("collection face offset is out of range"));
            }
            version = read_u32(&bytes, base)?;
        }

        match version {
            TAG_TRUETYPE | TAG_TRUE => {}
            TAG_OTTO => {
                return Err(FontError::unsupported(
                    "OpenType/CFF outlines are not supported; use a TrueType (.ttf) font",
                ));
            }
            other => {
                return Err(FontError::invalid(format!(
                    "unknown SFNT version 0x{other:08x}"
                )));
            }
        }

        let num_tables = read_u16(&bytes, base + 4)? as usize;
        let directory_end = base + 12 + num_tables * 16;
        if directory_end > bytes.len() {
            return Err(FontError::invalid("truncated table directory"));
        }

        let mut tables = Vec::with_capacity(num_tables);
        for index in 0..num_tables {
            let record = base + 12 + index * 16;
            let tag = Tag([
                bytes[record],
                bytes[record + 1],
                bytes[record + 2],
                bytes[record + 3],
            ]);
            let offset = read_u32(&bytes, record + 8)? as usize;
            let length = read_u32(&bytes, record + 12)? as usize;
            let end = offset
                .checked_add(length)
                .ok_or_else(|| FontError::invalid(format!("table {:?} range overflows", tag.0)))?;
            if end > bytes.len() {
                return Err(FontError::invalid(format!(
                    "table {:?} extends past the end of the file",
                    tag.0
                )));
            }
            tables.push(Table {
                tag,
                offset,
                length,
            });
        }

        let find = |tag: &[u8; 4]| tables.iter().find(|table| table.tag.0 == *tag).copied();

        let head = find(b"head").ok_or_else(|| FontError::invalid("missing 'head' table"))?;
        if head.length < 54 {
            return Err(FontError::invalid("'head' table is truncated"));
        }
        let units_per_em = read_u16(&bytes, head.offset + 18)?;
        if units_per_em == 0 {
            return Err(FontError::invalid("unitsPerEm is zero"));
        }
        let index_to_loc_format = read_i16(&bytes, head.offset + 50)?;

        let maxp = find(b"maxp").ok_or_else(|| FontError::invalid("missing 'maxp' table"))?;
        if maxp.length < 6 {
            return Err(FontError::invalid("'maxp' table is truncated"));
        }
        let num_glyphs = read_u16(&bytes, maxp.offset + 4)?;

        let hhea = find(b"hhea").ok_or_else(|| FontError::invalid("missing 'hhea' table"))?;
        if hhea.length < 36 {
            return Err(FontError::invalid("'hhea' table is truncated"));
        }
        let mut ascender = read_i16(&bytes, hhea.offset + 4)?;
        let mut descender = read_i16(&bytes, hhea.offset + 6)?;
        let mut line_gap = read_i16(&bytes, hhea.offset + 8)?;
        let number_of_h_metrics = read_u16(&bytes, hhea.offset + 34)?;
        if number_of_h_metrics == 0 {
            return Err(FontError::invalid("numberOfHMetrics is zero"));
        }

        let hmtx = find(b"hmtx").ok_or_else(|| FontError::invalid("missing 'hmtx' table"))?;
        let minimum_hmtx = number_of_h_metrics as usize * 4;
        if hmtx.length < minimum_hmtx {
            return Err(FontError::invalid("'hmtx' table is truncated"));
        }

        // OS/2 typographic metrics are generally better for UI text than the
        // hhea values, but some old fonts fill them with zeroes.
        if let Some(os2) = find(b"OS/2")
            && os2.length >= 78
        {
            let typo_ascender = read_i16(&bytes, os2.offset + 68)?;
            let typo_descender = read_i16(&bytes, os2.offset + 70)?;
            let typo_line_gap = read_i16(&bytes, os2.offset + 72)?;
            if typo_ascender != 0 || typo_descender != 0 {
                ascender = typo_ascender;
                descender = typo_descender;
                line_gap = typo_line_gap;
            }
        }

        let cmap_table = find(b"cmap").ok_or_else(|| FontError::invalid("missing 'cmap' table"))?;
        let cmap = Cmap::parse(&bytes, cmap_table.offset, cmap_table.length)?;

        let kern = find(b"kern")
            .and_then(|table| KernTable::parse(&bytes, table.offset, table.length).ok());

        let (family_name, style_name) = parse_names(&bytes, find(b"name"));

        let loca = find(b"loca");
        let glyf = find(b"glyf");
        let expected_loca_entries = num_glyphs as usize + 1;
        if let Some(loca) = loca {
            let entry_size = if index_to_loc_format == 0 { 2 } else { 4 };
            if loca.length < expected_loca_entries * entry_size {
                return Err(FontError::invalid("'loca' table is truncated"));
            }
        }

        Ok(Self {
            bytes,
            tables,
            units_per_em,
            num_glyphs,
            ascender,
            descender,
            line_gap,
            family_name,
            style_name,
            number_of_h_metrics,
            index_to_loc_format,
            hmtx_offset: hmtx.offset,
            loca_offset: loca.map_or(0, |table| table.offset),
            glyf_offset: glyf.map_or(0, |table| table.offset),
            cmap,
            kern,
        })
    }

    /// Looks a table up by tag. Useful for diagnostics and future extensions.
    pub fn table_range(&self, tag: &[u8; 4]) -> Option<Range<usize>> {
        self.tables
            .iter()
            .find(|table| table.tag.0 == *tag)
            .map(|table| table.offset..table.offset + table.length)
    }

    /// Maps a Unicode scalar to a glyph index.
    pub fn glyph_index(&self, ch: char) -> Option<u16> {
        self.cmap.lookup(ch as u32)
    }

    /// Horizontal advance in font units.
    pub fn advance(&self, glyph: u16) -> f32 {
        let index = glyph as usize;
        let hmetrics = self.number_of_h_metrics as usize;
        let record = if index < hmetrics {
            self.hmtx_offset + index * 4
        } else {
            self.hmtx_offset + (hmetrics - 1) * 4
        };
        read_u16(&self.bytes, record).map_or(0.0, f32::from)
    }

    /// Left side bearing in font units.
    pub fn side_bearing(&self, glyph: u16) -> f32 {
        let index = glyph as usize;
        let hmetrics = self.number_of_h_metrics as usize;
        let record = if index < hmetrics {
            self.hmtx_offset + index * 4 + 2
        } else {
            self.hmtx_offset + hmetrics * 4 + (index - hmetrics) * 2
        };
        read_i16(&self.bytes, record).map_or(0.0, f32::from)
    }

    /// Kerning adjustment between two glyphs in font units.
    pub fn kern(&self, left: u16, right: u16) -> f32 {
        self.kern
            .as_ref()
            .map_or(0.0, |table| table.lookup(left, right) as f32)
    }

    /// Extracts a glyph outline in font units.
    pub fn outline(&self, glyph: u16) -> Result<Outline, FontError> {
        if glyph >= self.num_glyphs {
            return Err(FontError::invalid(format!(
                "glyph {glyph} is outside 0..{}",
                self.num_glyphs
            )));
        }
        if self.loca_offset == 0 || self.glyf_offset == 0 {
            return Ok(Outline::default());
        }
        let start = self.loca_entry(glyph)? as usize;
        let end = self.loca_entry(glyph.saturating_add(1))? as usize;
        if start >= end {
            return Ok(Outline::default());
        }
        let start = self.glyf_offset + start;
        let end = self.glyf_offset + end;
        if end > self.bytes.len() {
            return Err(FontError::invalid("glyph data is out of range"));
        }
        glyf::parse_glyph(self, &self.bytes[start..end], 0)
    }

    /// Scaled vertical metrics.
    pub fn metrics(&self, size_px: f32) -> FontMetrics {
        let scale = size_px / self.units_per_em.max(1) as f32;
        let ascent = self.ascender as f32 * scale;
        let descent = self.descender as f32 * scale;
        let line_gap = self.line_gap as f32 * scale;
        FontMetrics {
            size_px,
            ascent,
            descent,
            line_gap,
            line_height: (ascent - descent + line_gap).max(size_px),
            units_per_em: self.units_per_em,
        }
    }

    pub(crate) fn glyph_bytes(&self, glyph: u16) -> Option<&[u8]> {
        if self.loca_offset == 0 || self.glyf_offset == 0 || glyph >= self.num_glyphs {
            return None;
        }
        let start = self.loca_entry(glyph).ok()? as usize;
        let end = self.loca_entry(glyph + 1).ok()? as usize;
        if start >= end {
            return None;
        }
        self.bytes
            .get(self.glyf_offset + start..self.glyf_offset + end)
    }

    fn loca_entry(&self, glyph: u16) -> Result<u32, FontError> {
        let offset = if self.index_to_loc_format == 0 {
            self.loca_offset + glyph as usize * 2
        } else {
            self.loca_offset + glyph as usize * 4
        };
        if self.index_to_loc_format == 0 {
            read_u16(&self.bytes, offset)
                .map(u32::from)
                .map(|value| value * 2)
        } else {
            read_u32(&self.bytes, offset)
        }
    }
}

fn parse_names(bytes: &[u8], table: Option<Table>) -> (String, String) {
    let Some(table) = table else {
        return (String::new(), String::new());
    };
    let mut family = String::new();
    let mut style = String::new();
    let Ok(format) = read_u16(bytes, table.offset) else {
        return (family, style);
    };
    let Ok(count) = read_u16(bytes, table.offset + 2) else {
        return (family, style);
    };
    let string_offset = table.offset + 4 + if format == 1 { 2 } else { 0 };
    let Ok(string_base) = read_u16(bytes, string_offset) else {
        return (family, style);
    };
    let string_base = table.offset + string_base as usize;

    for index in 0..count as usize {
        let record = table.offset + 6 + if format == 1 { 2 } else { 0 } + index * 12;
        let Ok(platform) = read_u16(bytes, record) else {
            break;
        };
        let Ok(encoding) = read_u16(bytes, record + 2) else {
            break;
        };
        let Ok(name_id) = read_u16(bytes, record + 6) else {
            break;
        };
        let Ok(length) = read_u16(bytes, record + 8) else {
            break;
        };
        let Ok(offset) = read_u16(bytes, record + 10) else {
            break;
        };
        let start = string_base + offset as usize;
        let end = start + length as usize;
        if end > bytes.len() {
            continue;
        }
        let raw = &bytes[start..end];
        let text = if platform == 3 || platform == 0 {
            decode_utf16_be(raw)
        } else if platform == 1 && encoding == 0 {
            raw.iter().map(|&byte| byte as char).collect()
        } else {
            continue;
        };
        match name_id {
            1 if family.is_empty() => family = text,
            2 if style.is_empty() => style = text,
            _ => {}
        }
        if !family.is_empty() && !style.is_empty() {
            break;
        }
    }

    (family, style)
}

fn decode_utf16_be(bytes: &[u8]) -> String {
    let units = bytes
        .chunks_exact(2)
        .map(|pair| u16::from_be_bytes([pair[0], pair[1]]));
    char::decode_utf16(units)
        .map(|unit| unit.unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect()
}

pub(crate) fn read_u8(bytes: &[u8], offset: usize) -> Result<u8, FontError> {
    bytes
        .get(offset)
        .copied()
        .ok_or_else(|| FontError::invalid("unexpected end of font data"))
}

pub(crate) fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, FontError> {
    let slice = bytes
        .get(offset..offset + 2)
        .ok_or_else(|| FontError::invalid("unexpected end of font data"))?;
    Ok(u16::from_be_bytes([slice[0], slice[1]]))
}

pub(crate) fn read_i16(bytes: &[u8], offset: usize) -> Result<i16, FontError> {
    read_u16(bytes, offset).map(|value| value as i16)
}

pub(crate) fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, FontError> {
    let slice = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| FontError::invalid("unexpected end of font data"))?;
    Ok(u32::from_be_bytes([slice[0], slice[1], slice[2], slice[3]]))
}
