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

//! Metadata extraction, from bytes that are already sfnt.
//!
//! The `parse` target has to get past container detection and, for a compressed
//! container, produce something a decoder accepts before a single table is read. This
//! one hands the mutator's bytes straight to `parse::parse_sfnt`, so the budget is spent
//! in the table readers, the name-record decoding, the license classifier and the
//! codepoint walk instead of on failing a magic-number check.
//!
//! No API had to be widened for this: `fontina_core::parse::parse_sfnt` is the
//! documented entry point for "sfnt bytes in, `FaceMetadata` out" and was already
//! public, as is `FileInfo`, which is part of the published schema.

#![no_main]

use fontina_core::model::{Container, FileInfo};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // `parse_sfnt` only clones this into each face; nothing it holds steers the parse.
    // The hash is left empty rather than computed, so the budget goes to the parser.
    let file = FileInfo {
        path: "fuzz".to_owned(),
        size: data.len() as u64,
        mtime: 0,
        blake3: String::new(),
        container: Container::Ttf,
        face_count: 0,
    };
    let _ = fontina_core::parse::parse_sfnt(data, &file);
});
