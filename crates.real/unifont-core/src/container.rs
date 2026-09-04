//! Unwrap WOFF / WOFF2 containers to raw sfnt bytes. TTF/OTF/TTC pass through.

use crate::error::{Error, Result};
use crate::model::Container;
use std::borrow::Cow;
use std::io::{Cursor, Read};

/// Return the sfnt bytes for a font file, decoding WOFF and WOFF2 when needed.
pub fn unwrap(container: Container, bytes: &[u8]) -> Result<Cow<'_, [u8]>> {
    match container {
        Container::Ttf | Container::Otf | Container::Ttc => Ok(Cow::Borrowed(bytes)),
        Container::Woff => decode_woff1(bytes).map(Cow::Owned),
        Container::Woff2 => {
            let mut cursor = Cursor::new(bytes);
            woff2_patched::convert_woff2_to_ttf(&mut cursor)
                .map(Cow::Owned)
                .map_err(|e| Error::Woff(format!("woff2: {e:?}")))
        }
    }
}

fn be_u16(b: &[u8], at: usize) -> Result<u16> {
    b.get(at..at + 2)
        .map(|s| u16::from_be_bytes([s[0], s[1]]))
        .ok_or_else(|| Error::Woff("truncated WOFF header".into()))
}

fn be_u32(b: &[u8], at: usize) -> Result<u32> {
    b.get(at..at + 4)
        .map(|s| u32::from_be_bytes([s[0], s[1], s[2], s[3]]))
        .ok_or_else(|| Error::Woff("truncated WOFF header".into()))
}

/// Rebuild an sfnt from a WOFF 1.0 file (W3C WOFF File Format 1.0, section 3).
fn decode_woff1(bytes: &[u8]) -> Result<Vec<u8>> {
    if bytes.len() < 44 {
        return Err(Error::Woff("file too short".into()));
    }
    let flavor = be_u32(bytes, 4)?;
    let num_tables = be_u16(bytes, 12)? as usize;
    let dir_start = 44;
    let mut entries = Vec::with_capacity(num_tables);
    for i in 0..num_tables {
        let e = dir_start + i * 20;
        let tag = be_u32(bytes, e)?;
        let offset = be_u32(bytes, e + 4)? as usize;
        let comp_len = be_u32(bytes, e + 8)? as usize;
        let orig_len = be_u32(bytes, e + 12)? as usize;
        let checksum = be_u32(bytes, e + 16)?;
        entries.push((tag, offset, comp_len, orig_len, checksum));
    }
    // sfnt header
    let mut out = Vec::new();
    let n = num_tables as u16;
    let mut entry_selector = 0u16;
    while (1u16 << (entry_selector + 1)) <= n {
        entry_selector += 1;
    }
    let search_range = (1u16 << entry_selector) * 16;
    let range_shift = n * 16 - search_range;
    out.extend_from_slice(&flavor.to_be_bytes());
    out.extend_from_slice(&n.to_be_bytes());
    out.extend_from_slice(&search_range.to_be_bytes());
    out.extend_from_slice(&entry_selector.to_be_bytes());
    out.extend_from_slice(&range_shift.to_be_bytes());

    let mut table_offset = 12 + num_tables * 16;
    let mut tables: Vec<Vec<u8>> = Vec::with_capacity(num_tables);
    for &(tag, offset, comp_len, orig_len, checksum) in &entries {
        let comp = bytes
            .get(offset..offset + comp_len)
            .ok_or_else(|| Error::Woff("table data out of bounds".into()))?;
        let data = if comp_len == orig_len {
            comp.to_vec()
        } else {
            let mut d = flate2::read::ZlibDecoder::new(comp);
            let mut buf = Vec::with_capacity(orig_len);
            d.read_to_end(&mut buf)
                .map_err(|e| Error::Woff(format!("zlib: {e}")))?;
            buf
        };
        out.extend_from_slice(&tag.to_be_bytes());
        out.extend_from_slice(&checksum.to_be_bytes());
        out.extend_from_slice(&(table_offset as u32).to_be_bytes());
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let padded = (data.len() + 3) & !3;
        table_offset += padded;
        tables.push(data);
    }
    for data in tables {
        let padded = (data.len() + 3) & !3;
        out.extend_from_slice(&data);
        out.resize(out.len() + (padded - data.len()), 0);
    }
    Ok(out)
}
