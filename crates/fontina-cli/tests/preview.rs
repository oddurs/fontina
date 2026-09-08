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

//! `fontina preview`, when the font cannot show the text.
//!
//! The first thing anyone does with a font manager is preview a font with their own
//! words. When the font does not cover them, shaping substitutes `.notdef` and the font
//! draws its empty box, so the reader gets a row of rectangles that looks like a bug in
//! the program rather than an answer about the font. The answer is available — it is the
//! whole point of `covers` — so the preview says it.

use fontina_testkit::Cli;

/// A sandbox with the fixtures indexed and a terminal eighty columns wide, which is what
/// every test here draws against.
fn session(name: &str) -> Cli {
    let cli = fontina_testkit::cli!(name).with_env("COLUMNS", "80");
    cli.scan_fixtures();
    cli
}

#[test]
fn a_preview_says_when_the_font_has_no_glyph_for_the_text() {
    let s = session("notdef");
    let amiri = s.id_of("Amiri-Regular.ttf");

    // Amiri is an Arabic and Latin face with no CJK at all.
    let out = s.ok(&[
        "preview",
        &amiri,
        "--protocol",
        "blocks",
        "--text",
        "日本語",
    ]);
    let title = out.lines().next().unwrap_or_default();
    assert!(
        title.contains("3 of 3"),
        "the title says how much of the text the font cannot show: {title:?}"
    );
    assert!(
        title.contains("not in this font"),
        "and says what that means: {title:?}"
    );

    let out = s.ok(&["preview", &amiri, "--protocol", "blocks", "--text", "Hi"]);
    let title = out.lines().next().unwrap_or_default();
    assert!(
        !title.contains("not in this font"),
        "text the font covers gets no warning: {title:?}"
    );

    // Mixed text counts only the part that is missing.
    let out = s.ok(&["preview", &amiri, "--protocol", "blocks", "--text", "A日"]);
    let title = out.lines().next().unwrap_or_default();
    assert!(title.contains("1 of 2"), "{title:?}");
}

/// The preview still draws the boxes, so the reader sees the font's own answer.
#[test]
fn a_preview_of_uncovered_text_still_draws_something() {
    let s = session("boxes");
    let amiri = s.id_of("Amiri-Regular.ttf");
    let out = s.ok(&[
        "preview",
        &amiri,
        "--protocol",
        "blocks",
        "--text",
        "日本語",
    ]);
    assert!(
        out.contains('\u{2580}'),
        "the .notdef boxes are drawn, not suppressed"
    );
}
