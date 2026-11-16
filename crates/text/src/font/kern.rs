//! Legacy `kern` table support (format 0 pair adjustments).
//!
//! Modern fonts often put kerning into `GPOS` instead. This parser handles the
//! classic table so common UI fonts get correct advances; missing kerning is
//! harmless and only affects spacing slightly.

use super::FontError;
use super::sfnt::{read_i16, read_u16, read_u32};

#[derive(Debug, Default)]
pub(crate) struct KernTable {
    pairs: Vec<(u32, i16)>,
}

impl KernTable {
    pub(crate) fn parse(bytes: &[u8], offset: usize, length: usize) -> Result<Self, FontError> {
        let table = bytes
            .get(offset..offset + length)
            .ok_or_else(|| FontError::invalid("'kern' table is out of range"))?;
        if table.len() < 4 {
            return Err(FontError::invalid("'kern' table is truncated"));
        }

        let version = read_u16(table, 0)?;
        let mut pairs = Vec::new();

        if version == 0 {
            let table_count = read_u16(table, 2)? as usize;
            let mut cursor = 4usize;
            for _ in 0..table_count {
                if cursor + 6 > table.len() {
                    break;
                }
                let subtable_length = read_u16(table, cursor + 2)? as usize;
                let coverage = read_u16(table, cursor + 4)?;
                let format = (coverage >> 8) as u8;
                let horizontal = coverage & 0x1 != 0;
                let minimum = coverage & 0x2 != 0;
                let cross_stream = coverage & 0x4 != 0;
                if format == 0 && horizontal && !minimum && !cross_stream {
                    parse_format0(
                        table,
                        cursor + 6,
                        subtable_length.saturating_sub(6),
                        &mut pairs,
                    )?;
                }
                if subtable_length == 0 {
                    break;
                }
                cursor = cursor.saturating_add(subtable_length);
            }
        } else if read_u32(table, 0)? == 0x0001_0000 {
            let table_count = read_u32(table, 4)? as usize;
            let mut cursor = 8usize;
            for _ in 0..table_count {
                if cursor + 8 > table.len() {
                    break;
                }
                let subtable_length = read_u32(table, cursor)? as usize;
                let coverage = read_u16(table, cursor + 4)?;
                let format = (coverage & 0xFF) as u8;
                if format == 0 {
                    parse_format0(
                        table,
                        cursor + 8,
                        subtable_length.saturating_sub(8),
                        &mut pairs,
                    )?;
                }
                if subtable_length == 0 {
                    break;
                }
                cursor = cursor.saturating_add(subtable_length);
            }
        }

        pairs.sort_unstable_by_key(|&(key, _)| key);
        pairs.dedup_by_key(|&mut (key, _)| key);
        Ok(Self { pairs })
    }

    pub(crate) fn lookup(&self, left: u16, right: u16) -> i16 {
        let key = ((left as u32) << 16) | right as u32;
        self.pairs
            .binary_search_by_key(&key, |&(candidate, _)| candidate)
            .map(|index| self.pairs[index].1)
            .unwrap_or(0)
    }
}

fn parse_format0(
    table: &[u8],
    offset: usize,
    available: usize,
    pairs: &mut Vec<(u32, i16)>,
) -> Result<(), FontError> {
    if available < 8 {
        return Ok(());
    }
    let pair_count = read_u16(table, offset)? as usize;
    let pairs_offset = offset + 8;
    let needed = pair_count * 6;
    if pairs_offset + needed > table.len() {
        return Err(FontError::invalid("'kern' format 0 pairs are truncated"));
    }
    for index in 0..pair_count {
        let record = pairs_offset + index * 6;
        let left = read_u16(table, record)?;
        let right = read_u16(table, record + 2)?;
        let value = read_i16(table, record + 4)?;
        if value != 0 {
            pairs.push((((left as u32) << 16) | right as u32, value));
        }
    }
    Ok(())
}
