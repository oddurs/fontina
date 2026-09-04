// SPDX-License-Identifier: GPL-3.0-or-later
//
// fontina — a font manager.
// Copyright (C) 2026 Oddur Sigurdsson
//
// This program is free software: you can redistribute it and/or modify it under the
// terms of the GNU General Public License as published by the Free Software Foundation,
// either version 3 of the License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful, but WITHOUT ANY
// WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A
// PARTICULAR PURPOSE. See the GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with this
// program. If not, see <https://www.gnu.org/licenses/>.

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

/// Narrow an sfnt header field to the u16 it is stored in, clamping instead of
/// wrapping for the table counts where the OpenType formulas overflow.
fn saturate_u16(v: u32) -> u16 {
    u16::try_from(v).unwrap_or(u16::MAX)
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
    // sfnt header. A directory with no tables is not a font, and rangeShift is
    // defined as numTables * 16 - searchRange, so zero is a parse error, not a
    // subtraction to be attempted.
    if num_tables == 0 {
        return Err(Error::Woff("WOFF header declares no tables".into()));
    }
    let mut out = Vec::new();
    let n = num_tables as u16;
    // entrySelector = floor(log2(numTables)), searchRange = 2^entrySelector * 16,
    // rangeShift = numTables * 16 - searchRange (OpenType, table directory). The last
    // two are computed in u32 because both leave u16 once numTables reaches 4096, and
    // are stored saturated: they are binary-search hints no consumer needs exact.
    let entry_selector = (u16::BITS - 1 - n.leading_zeros()) as u16;
    let search_range = (1u32 << entry_selector) * 16;
    let range_shift = u32::from(n) * 16 - search_range;
    out.extend_from_slice(&flavor.to_be_bytes());
    out.extend_from_slice(&n.to_be_bytes());
    out.extend_from_slice(&saturate_u16(search_range).to_be_bytes());
    out.extend_from_slice(&entry_selector.to_be_bytes());
    out.extend_from_slice(&saturate_u16(range_shift).to_be_bytes());

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    /// A WOFF 1.0 file whose directory holds `num_tables` empty entries. The header
    /// arithmetic only gets interesting at table counts whose directory runs to
    /// hundreds of kilobytes, so the bytes are built here rather than checked in.
    fn woff1(num_tables: u16) -> Vec<u8> {
        let mut b = Vec::with_capacity(44 + usize::from(num_tables) * 20);
        b.extend_from_slice(b"wOFF");
        b.extend_from_slice(&0x0001_0000u32.to_be_bytes()); // flavor
        b.extend_from_slice(&0u32.to_be_bytes()); // length
        b.extend_from_slice(&num_tables.to_be_bytes()); // numTables
        b.extend_from_slice(&0u16.to_be_bytes()); // reserved
        b.extend_from_slice(&0u32.to_be_bytes()); // totalSfntSize
        b.extend_from_slice(&0u32.to_be_bytes()); // major/minorVersion
        b.extend_from_slice(&[0u8; 20]); // meta and priv blocks, all absent
        assert_eq!(b.len(), 44);
        for tag in 0..u32::from(num_tables) {
            b.extend_from_slice(&tag.to_be_bytes());
            b.extend_from_slice(&[0u8; 16]); // offset, compLength, origLength, checksum
        }
        b
    }

    /// Decode on another thread so a regression that spins forever fails the test
    /// instead of hanging it.
    fn decode_bounded(bytes: Vec<u8>) -> Result<Vec<u8>> {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(decode_woff1(&bytes));
        });
        rx.recv_timeout(Duration::from_secs(30))
            .expect("decode_woff1 must terminate")
    }

    #[test]
    fn woff1_header_matches_opentype_for_a_normal_table_count() {
        let out = decode_bounded(woff1(9)).unwrap();
        assert_eq!(out[4..6], 9u16.to_be_bytes()); // numTables
        assert_eq!(out[6..8], 128u16.to_be_bytes()); // searchRange = 2^3 * 16
        assert_eq!(out[8..10], 3u16.to_be_bytes()); // entrySelector = floor(log2(9))
        assert_eq!(out[10..12], 16u16.to_be_bytes()); // rangeShift = 9 * 16 - 128
    }

    #[test]
    fn woff1_header_survives_a_table_count_that_overflows_the_formulas() {
        // 1 << (entry_selector + 1) leaves u16 here: it used to panic in debug and spin
        // forever in release, wedging the scan worker for good.
        let out = decode_bounded(woff1(32_768)).unwrap();
        assert_eq!(out[4..6], 32_768u16.to_be_bytes());
        assert_eq!(out[6..8], u16::MAX.to_be_bytes()); // 2^15 * 16, saturated
        assert_eq!(out[8..10], 15u16.to_be_bytes());
        assert_eq!(out[10..12], 0u16.to_be_bytes()); // 32768 * 16 - 2^15 * 16
    }

    #[test]
    fn woff1_with_no_tables_is_a_parse_error() {
        // rangeShift would be 0 * 16 - 16, which is not a number.
        let err = decode_bounded(woff1(0)).unwrap_err();
        assert!(matches!(err, Error::Woff(_)), "{err:?}");
    }
}
