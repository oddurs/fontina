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

//! The WOFF 2.0 decoder is not ours, and it panics on an input we cannot stop it being
//! given. This holds the containment that keeps that panic out of `load_bytes`.
//!
//! The input lives in `fuzz/regressions/open/`, which the replay test deliberately skips
//! and `scripts/fuzz seed` deliberately does not copy into a corpus, so this is the only
//! test that exercises it. See `fuzz/regressions/README.md` for why it is filed as open
//! rather than fixed.

use flate2::read::GzDecoder;
use std::io::Read;
use std::path::Path;

fn open_input(name: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fuzz/regressions/open")
        .join(name);
    let raw = std::fs::read(&path)
        .unwrap_or_else(|e| panic!("{} must exist and be readable: {e}", path.display()));
    if !name.ends_with(".gz") {
        return raw;
    }
    let mut out = Vec::new();
    GzDecoder::new(&raw[..])
        .read_to_end(&mut out)
        .expect("valid gzip");
    out
}

/// `bboxStreamSize` smaller than the bitmap derived from `numGlyphs` underflows a
/// subtraction inside `woff2-patched`. With `overflow-checks` on — which is exactly the
/// configuration this test runs in — that is a panic, and it used to escape `load_bytes`.
/// It must now come back as an error like any other malformed file.
#[test]
fn a_woff2_that_panics_the_decoder_comes_back_as_an_error() {
    let bytes = open_input("woff2-bbox-stream-underflow.woff2.gz");
    let result = fontina_core::load_bytes(&bytes, "woff2-bbox-stream-underflow");
    let err = result.expect_err("this input cannot produce a face");
    let message = err.to_string();
    assert!(
        message.contains("woff2"),
        "the error should name the container it came from, got {message:?}"
    );
}

/// A second panic in the same decoder, in different arithmetic.
///
/// `ttf_header::TableDirectory::new` subtracts one field read off the wire from another,
/// and a file that declares the first smaller underflows it. Found by
/// `scripts/fuzz parse` in a ten-minute run, minimised to forty-nine bytes — a WOFF 2.0
/// header and nothing else, so it costs nothing to keep.
///
/// It is here rather than in the fixed table for the same reason as the one above: the
/// arithmetic is not ours (ADR 0005), 0.4.0 is the newest release, and what fontina can
/// hold is the blast radius. Two inputs in two different functions is also the argument
/// for containing the decoder rather than waiting for the last bug in it.
#[test]
fn a_second_woff2_panic_is_contained_the_same_way() {
    let bytes = open_input("woff2-table-directory-underflow.woff2");
    let err = fontina_core::load_bytes(&bytes, "woff2-table-directory-underflow")
        .expect_err("this input cannot produce a face");
    let message = err.to_string();
    assert!(
        message.contains("woff2"),
        "the error should name the container it came from, got {message:?}"
    );
}

/// The containment must not swallow the ordinary case: a WOFF 2.0 file that decodes
/// still decodes, and one that is merely truncated still says so rather than reporting a
/// panic that did not happen.
#[test]
fn containment_leaves_well_formed_and_ordinarily_broken_files_alone() {
    let good = std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/inter-latin-400-normal.woff2"),
    )
    .expect("the woff2 fixture");
    assert!(
        fontina_core::load_bytes(&good, "inter.woff2").is_ok(),
        "a valid WOFF 2.0 file must still parse"
    );

    let truncated = &good[..good.len() / 2];
    let message = fontina_core::load_bytes(truncated, "truncated.woff2")
        .expect_err("half a file is not a font")
        .to_string();
    assert!(
        !message.contains("panicked"),
        "an ordinary decode failure must not be reported as a panic, got {message:?}"
    );
}
