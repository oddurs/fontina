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

//! The whole import wrapper, from arbitrary bytes.
//!
//! `load_bytes` is `Container::detect` → `container::unwrap` → `parse::parse_sfnt`, so
//! this target spends most of its budget in the two decoders: the hand-written WOFF 1.0
//! sfnt reconstruction and the WOFF2 brotli decoder. That is the code an untrusted file
//! reaches first and the code with the least of fontations behind it.
//!
//! It must return. A panic is a bug, a hang is a bug (`-timeout`), and an allocation
//! driven by a length field in the input is a bug (`-rss_limit_mb`); `scripts/fuzz`
//! passes both limits, because the `catch_unwind` in `scan::parse_paths` only ever
//! catches the first of the three.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // The name is only ever echoed back into `FileInfo.path`, so it is a constant.
    let _ = fontina_core::load_bytes(data, "fuzz");
});
