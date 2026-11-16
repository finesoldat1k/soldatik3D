//! Unicode `cmap` decoding. Format 4 (BMP) and format 12 (full Unicode) are
//! preferred; formats 0 and 6 are kept as fallbacks for old fonts.

use super::FontError;
use super::sfnt::{read_u16, read_u32};

#[derive(Debug)]
pub(crate) struct Cmap {
    format: CmapFormat,
}

#[derive(Debug)]
enum CmapFormat {
    SegmentMapping4 {
        subtable: Vec<u8>,
        end_codes: Vec<u16>,
        start_codes: Vec<u16>,
        id_deltas: Vec<i16>,
        id_range_offsets: Vec<u16>,
        range_offset_positions: Vec<usize>,
    },
    SegmentedCoverage12(Vec<(u32, u32, u32)>),
    Trimmed6 {
        first_code: u16,
        glyphs: Vec<u16>,
    },
    Byte0([u8; 256]),
}

impl Cmap {
    pub(crate) fn parse(bytes: &[u8], offset: usize, length: usize) -> Result<Self, FontError> {
        if length < 4 {
            return Err(FontError::invalid("'cmap' table is truncated"));
        }
        let table = bytes
            .get(offset..offset + length)
            .ok_or_else(|| FontError::invalid("'cmap' table is out of range"))?;
        let count = read_u16(table, 2)? as usize;
        if 4 + count * 8 > table.len() {
            return Err(FontError::invalid("'cmap' encoding records are truncated"));
        }

        let mut best: Option<(u32, usize, u16)> = None;
        for index in 0..count {
            let record = 4 + index * 8;
            let platform = read_u16(table, record)?;
            let encoding = read_u16(table, record + 2)?;
            let subtable_offset = read_u32(table, record + 4)? as usize;
            let Some(format) = read_u16(table, subtable_offset).ok() else {
                continue;
            };
            let score = match (platform, encoding, format) {
                (3, 10, 12) => 100,
                (0, _, 12) => 90,
                (3, 1, 4) => 80,
                (0, _, 4) => 70,
                (3, 0, 4) => 50, // symbol fonts map into the private use area
                (3, 1, 6) => 40,
                (1, 0, 6) => 30,
                (1, 0, 0) => 20,
                _ => 0,
            };
            if score > 0 && best.is_none_or(|(best_score, _, _)| score > best_score) {
                best = Some((score, subtable_offset, format));
            }
        }

        let Some((_, subtable_offset, format)) = best else {
            return Err(FontError::invalid(
                "no supported Unicode 'cmap' subtable (need format 4 or 12)",
            ));
        };

        let format = match format {
            4 => Self::parse_format4(table, subtable_offset)?,
            12 => Self::parse_format12(table, subtable_offset)?,
            6 => Self::parse_format6(table, subtable_offset)?,
            0 => Self::parse_format0(table, subtable_offset)?,
            other => {
                return Err(FontError::unsupported(format!(
                    "'cmap' format {other} is not supported"
                )));
            }
        };

        Ok(Self { format })
    }

    pub(crate) fn lookup(&self, code: u32) -> Option<u16> {
        match &self.format {
            CmapFormat::SegmentMapping4 {
                subtable,
                end_codes,
                start_codes,
                id_deltas,
                id_range_offsets,
                range_offset_positions,
            } => {
                if code > 0xFFFF {
                    return None;
                }
                let code = code as u16;
                let mut low = 0usize;
                let mut high = end_codes.len();
                while low < high {
                    let mid = (low + high) / 2;
                    if end_codes[mid] < code {
                        low = mid + 1;
                    } else {
                        high = mid;
                    }
                }
                let segment = *end_codes.get(low)?;
                if code < start_codes[low] || code > segment {
                    return None;
                }
                if id_range_offsets[low] == 0 {
                    let glyph = (code as i32 + id_deltas[low] as i32) as u16;
                    return (glyph != 0).then_some(glyph);
                }
                let address = range_offset_positions[low]
                    + id_range_offsets[low] as usize
                    + (code - start_codes[low]) as usize * 2;
                let glyph = read_u16(subtable, address).ok()?;
                if glyph == 0 {
                    None
                } else {
                    Some((glyph as i32 + id_deltas[low] as i32) as u16)
                }
            }
            CmapFormat::SegmentedCoverage12(groups) => {
                let mut low = 0usize;
                let mut high = groups.len();
                while low < high {
                    let mid = (low + high) / 2;
                    let (start, end, _) = groups[mid];
                    if code < start {
                        high = mid;
                    } else if code > end {
                        low = mid + 1;
                    } else {
                        let (start, _, glyph) = groups[mid];
                        let id = glyph + (code - start);
                        return (id != 0).then_some(id.min(u16::MAX as u32) as u16);
                    }
                }
                None
            }
            CmapFormat::Trimmed6 { first_code, glyphs } => {
                let index = code.checked_sub(*first_code as u32)? as usize;
                glyphs.get(index).copied().filter(|&glyph| glyph != 0)
            }
            CmapFormat::Byte0(glyphs) => {
                let index = code as usize;
                glyphs
                    .get(index)
                    .copied()
                    .filter(|&glyph| glyph != 0)
                    .map(u16::from)
            }
        }
    }

    fn parse_format4(table: &[u8], offset: usize) -> Result<CmapFormat, FontError> {
        let length = read_u16(table, offset + 2)? as usize;
        let subtable = table
            .get(offset..offset + length)
            .ok_or_else(|| FontError::invalid("'cmap' format 4 is out of range"))?;
        if subtable.len() < 16 {
            return Err(FontError::invalid("'cmap' format 4 is truncated"));
        }
        let seg_count_x2 = read_u16(subtable, 6)? as usize;
        if !seg_count_x2.is_multiple_of(2) || seg_count_x2 == 0 {
            return Err(FontError::invalid("'cmap' format 4 has invalid segCountX2"));
        }
        let seg_count = seg_count_x2 / 2;
        let end_codes_offset = 14;
        let start_codes_offset = end_codes_offset + seg_count * 2 + 2;
        let id_deltas_offset = start_codes_offset + seg_count * 2;
        let id_range_offsets_offset = id_deltas_offset + seg_count * 2;
        let glyph_id_array_offset = id_range_offsets_offset + seg_count * 2;
        if glyph_id_array_offset > subtable.len() {
            return Err(FontError::invalid("'cmap' format 4 arrays are truncated"));
        }

        let mut end_codes = Vec::with_capacity(seg_count);
        let mut start_codes = Vec::with_capacity(seg_count);
        let mut id_deltas = Vec::with_capacity(seg_count);
        let mut id_range_offsets = Vec::with_capacity(seg_count);
        let mut range_offset_positions = Vec::with_capacity(seg_count);

        for index in 0..seg_count {
            end_codes.push(read_u16(subtable, end_codes_offset + index * 2)?);
            start_codes.push(read_u16(subtable, start_codes_offset + index * 2)?);
            id_deltas.push(read_u16(subtable, id_deltas_offset + index * 2)? as i16);
            id_range_offsets.push(read_u16(subtable, id_range_offsets_offset + index * 2)?);
            range_offset_positions.push(id_range_offsets_offset + index * 2);
        }

        Ok(CmapFormat::SegmentMapping4 {
            subtable: subtable.to_vec(),
            end_codes,
            start_codes,
            id_deltas,
            id_range_offsets,
            range_offset_positions,
        })
    }

    fn parse_format12(table: &[u8], offset: usize) -> Result<CmapFormat, FontError> {
        let length = read_u32(table, offset + 4)? as usize;
        let subtable = table
            .get(offset..offset + length)
            .ok_or_else(|| FontError::invalid("'cmap' format 12 is out of range"))?;
        if subtable.len() < 16 {
            return Err(FontError::invalid("'cmap' format 12 is truncated"));
        }
        let num_groups = read_u32(subtable, 12)? as usize;
        if 16 + num_groups * 12 > subtable.len() {
            return Err(FontError::invalid("'cmap' format 12 groups are truncated"));
        }
        let mut groups = Vec::with_capacity(num_groups);
        for index in 0..num_groups {
            let record = 16 + index * 12;
            let start = read_u32(subtable, record)?;
            let end = read_u32(subtable, record + 4)?;
            let glyph = read_u32(subtable, record + 8)?;
            if start <= end {
                groups.push((start, end, glyph));
            }
        }
        groups.sort_unstable_by_key(|&(start, _, _)| start);
        Ok(CmapFormat::SegmentedCoverage12(groups))
    }

    fn parse_format6(table: &[u8], offset: usize) -> Result<CmapFormat, FontError> {
        let first_code = read_u16(table, offset + 6)?;
        let count = read_u16(table, offset + 8)? as usize;
        if offset + 10 + count * 2 > table.len() {
            return Err(FontError::invalid("'cmap' format 6 is truncated"));
        }
        let mut glyphs = Vec::with_capacity(count);
        for index in 0..count {
            glyphs.push(read_u16(table, offset + 10 + index * 2)?);
        }
        Ok(CmapFormat::Trimmed6 { first_code, glyphs })
    }

    fn parse_format0(table: &[u8], offset: usize) -> Result<CmapFormat, FontError> {
        let bytes = table
            .get(offset + 6..offset + 6 + 256)
            .ok_or_else(|| FontError::invalid("'cmap' format 0 is truncated"))?;
        let mut glyphs = [0u8; 256];
        glyphs.copy_from_slice(bytes);
        Ok(CmapFormat::Byte0(glyphs))
    }
}
