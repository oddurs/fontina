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

//! Things more than one test binary needs, and the fixtures none of them can commit.

use std::path::Path;

/// Write a TrueType collection holding every font in `sources`.
///
/// `fixtures/` has no `.ttc`, because a free one small enough to commit is hard to come
/// by and a collection is not a thing you can carve out of a single font. Several code
/// paths turn on `face_count > 1` all the same — a face id carries a `#index`, a
/// `@font-face` rule takes a fragment, and one file's tags belong to every face in it —
/// and until now nothing exercised any of them.
///
/// A collection is a header naming an offset per font, followed by the fonts. Each
/// font's table directory records offsets from the start of the *file*, so a font moved
/// to offset `n` needs every offset in its directory raised by `n`; that is the whole of
/// the work, and it is done here rather than committed as a binary nobody can read.
/// Checksums are left as they were: nothing in fontina verifies them, and a collection
/// whose checksums were right would say no more about the code under test.
pub fn write_collection(sources: &[&Path], to: &Path) {
    let fonts: Vec<Vec<u8>> = sources
        .iter()
        .map(|p| std::fs::read(p).unwrap_or_else(|e| panic!("reading {}: {e}", p.display())))
        .collect();

    // `ttcf`, a version, a count, and one offset per font.
    let header = 12 + 4 * fonts.len();
    let mut offsets = Vec::with_capacity(fonts.len());
    let mut at = header;
    for font in &fonts {
        at = at.next_multiple_of(4);
        offsets.push(at);
        at += font.len();
    }

    let mut out = Vec::with_capacity(at);
    out.extend_from_slice(b"ttcf");
    out.extend_from_slice(&1u16.to_be_bytes()); // major version
    out.extend_from_slice(&0u16.to_be_bytes()); // minor version
    out.extend_from_slice(&(fonts.len() as u32).to_be_bytes());
    for offset in &offsets {
        out.extend_from_slice(&(*offset as u32).to_be_bytes());
    }

    for (font, offset) in fonts.iter().zip(&offsets) {
        out.resize(*offset, 0);
        let mut font = font.clone();
        let tables = u16::from_be_bytes([font[4], font[5]]) as usize;
        for i in 0..tables {
            // A table record is tag, checksum, offset, length; the offset is at 8.
            let field = 12 + i * 16 + 8;
            let was = u32::from_be_bytes(font[field..field + 4].try_into().unwrap());
            let now = was + *offset as u32;
            font[field..field + 4].copy_from_slice(&now.to_be_bytes());
        }
        out.extend_from_slice(&font);
    }
    std::fs::write(to, out).unwrap_or_else(|e| panic!("writing {}: {e}", to.display()));
}
