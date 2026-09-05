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

//! Half-block previews for the details pane, rendered through `fontina_core::render` and
//! cached by face, text, size and pane width.

use super::theme::{self, Theme};
use fontina_core::FaceMetadata;
use fontina_core::render::{Bitmap, RenderOptions, render_face};
use ratatui::style::Style;
use ratatui::text::{Line, Span};

/// A face, the exact options its lines were rendered from, and the pixel height they
/// were laid out for. Keying on the options themselves means a new `RenderOptions` field
/// cannot be left out of the key and silently stop a preview repainting.
type Key = (i64, RenderOptions, u32);

/// Renderings kept between frames.
///
/// The details pane needs one at a time, but a waterfall needs one per size and a
/// comparison one per face, and re-rendering all of them on every keystroke would make
/// scrolling crawl. Most recent first, and bounded: a cache that grows without limit
/// while someone walks a library is a leak with a friendly name.
#[derive(Default)]
pub struct Cache {
    entries: Vec<(Key, Vec<Line<'static>>)>,
    theme: Theme,
    /// Renderings that actually reached the rasteriser.
    ///
    /// The point of a cache is the work it does not do, and "did not do it" is not
    /// something a return value can say. A test can ask.
    #[cfg(test)]
    rasterised: usize,
}

/// Renderings kept at once.
///
/// Enough for a full waterfall and a wide comparison together, which is the largest
/// thing the browser asks for: nine sizes down the ladder, and one per face of a family
/// on the screen at the same time. Below that a scroll would evict what it is about to
/// ask for again.
///
/// The other end is memory. A cached rendering is a `Vec<Line>` and each line holds a
/// `Vec<Span>` sized to the pane's width, at around forty bytes a span; the widest
/// thing the browser draws is a comparison across a two-hundred-column terminal, about
/// fifteen text rows deep, so a worst-case entry is on the order of 120 KB and
/// thirty-two of them are under 4 MB. PLAN.md §7 budgets 40 MB of idle RSS for the
/// browser at five thousand faces, so the cache at its very worst is a tenth of it, and
/// in the shape a details pane actually asks for — forty columns by six rows — closer
/// to a hundredth. Raising this number is a memory decision, and that is the arithmetic
/// to redo before raising it.
const CAPACITY: usize = 32;

/// Sample text for a face: the shared default for Latin, so the pane and the HTML
/// specimen agree, and the opening clause of its own script's paragraph otherwise.
pub fn sample_for(face: &FaceMetadata) -> String {
    fontina_core::typography::preview_text(face).to_string()
}

impl Cache {
    /// An empty cache that draws in `theme`.
    pub fn new(theme: Theme) -> Self {
        Cache {
            theme,
            ..Default::default()
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Lines for a preview of `face` under `opts`, fitting `px_rows` pixel rows (two per
    /// text row).
    pub fn lines(
        &mut self,
        face: &FaceMetadata,
        opts: &RenderOptions,
        px_rows: u32,
    ) -> Vec<Line<'static>> {
        let key = (crate::face_key(face), opts.clone(), px_rows);
        if let Some(i) = self.entries.iter().position(|(k, _)| *k == key) {
            // Touch it, so a waterfall being scrolled keeps its own renderings and
            // evicts whatever the reader has stopped looking at.
            let entry = self.entries.remove(i);
            let lines = entry.1.clone();
            self.entries.insert(0, entry);
            return lines;
        }
        #[cfg(test)]
        {
            self.rasterised += 1;
        }
        let lines = match render_face(face, opts) {
            Ok(bitmap) => {
                let mut lines = to_lines(
                    &bitmap,
                    px_rows.saturating_sub(2 * u32::from(bitmap.missing > 0)),
                    &self.theme,
                );
                // A font draws its `.notdef` box for every character it does not cover,
                // so a preview of text this face cannot show is a row of empty
                // rectangles and nothing else says why. The command line says it in the
                // title; here there is no title, so it goes above the drawing.
                if bitmap.missing > 0 {
                    lines.insert(
                        0,
                        Line::from(Span::styled(
                            format!(
                                "{} of {} glyph(s) not in this font",
                                bitmap.missing, bitmap.glyphs
                            ),
                            self.theme.warn(),
                        )),
                    );
                }
                lines
            }
            Err(e) => vec![Line::from(Span::styled(
                format!("preview unavailable: {e}"),
                self.theme.bad(),
            ))],
        };
        self.entries.insert(0, (key, lines.clone()));
        self.entries.truncate(CAPACITY);
        lines
    }

    /// How many renderings are held. Only the tests ask, but what they are asking is
    /// whether a waterfall keeps a rendering per row rather than thrashing one slot.
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// How many of the requests so far reached the rasteriser.
    #[cfg(test)]
    pub fn rasterised(&self) -> usize {
        self.rasterised
    }
}

/// Coverage to `▀` cells. Ink is drawn with the terminal's default foreground; the
/// half-block trick needs an explicit pair of colours only where there is ink.
fn to_lines(bm: &Bitmap, px_rows: u32, theme: &Theme) -> Vec<Line<'static>> {
    let w = bm.width as usize;
    let full = bm.height as usize;
    let want = (px_rows as usize).min(full);
    // Clip to the ink, not to the top of the rendering. The top rows are the font's
    // empty ascent: Source Serif at 28 px is 41 pixels tall with nothing above row 9, so
    // a details pane squeezed by feature controls showed eight rows of blank and looked
    // like a broken preview. Centre what is inked in the rows there are.
    let inked = |y: usize| bm.coverage[y * w..(y + 1) * w].iter().any(|&a| a != 0);
    let first = (0..full).find(|&y| inked(y));
    let start = match first {
        Some(first) => {
            let last = (first..full).rev().find(|&y| inked(y)).unwrap_or(first);
            let ink_h = last - first + 1;
            if ink_h <= want {
                first.saturating_sub((want - ink_h) / 2).min(full - want)
            } else {
                first
            }
        }
        None => 0,
    };
    let h = start + want;
    let mut out = Vec::with_capacity(want.div_ceil(2));
    // Ink colour: a neutral light grey blended over black reads on dark and light
    // themes alike once the block glyph carries both halves.
    let ink = |a: u8| theme.ink(a);
    for row in 0..want.div_ceil(2) {
        let y0 = start + row * 2;
        let y1 = y0 + 1;
        let mut spans: Vec<Span<'static>> = Vec::with_capacity(w);
        let mut blank = 0usize;
        for x in 0..w {
            let top = bm.coverage[y0 * w + x];
            let bottom = if y1 < h { bm.coverage[y1 * w + x] } else { 0 };
            if top == 0 && bottom == 0 {
                blank += 1;
                continue;
            }
            if blank > 0 {
                spans.push(Span::raw(" ".repeat(blank)));
                blank = 0;
            }
            // Two pixels in one cell. Normally the block glyph carries them as two
            // colours; with no colour to carry them the glyph itself says which half
            // is inked, and the preview survives rather than disappearing.
            spans.push(match (ink(top), ink(bottom)) {
                (Some(fg), Some(bg)) => Span::styled("▀", Style::default().fg(fg).bg(bg)),
                _ => Span::raw(theme::density(top, bottom).to_string()),
            });
        }
        out.push(Line::from(spans));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;
    use std::path::PathBuf;

    // ----- bitmaps as pictures -----

    /// The point of the fallback: a preview with no colour is still a preview.
    ///
    /// A half-block rendering normally puts two pixels in one cell as two colours, so
    /// under `NO_COLOR` it would come back as a rectangle of identical `▀` — type
    /// reduced to a solid block. The block glyphs carry the same two pixels as shape
    /// instead, and this asserts the shape is really there rather than one repeated
    /// character.
    #[test]
    fn a_preview_survives_a_terminal_with_no_colour() {
        let bm = art(&["..##..", "..##..", "######", "..##.."]);
        let plain = to_lines(&bm, 8, &Theme::new(theme::Depth::None));

        let drawn: String = plain
            .iter()
            .flat_map(|l| l.spans.iter())
            .map(|s| s.content.as_ref())
            .collect();

        assert!(
            !drawn.trim().is_empty(),
            "no colour must not mean no preview"
        );
        for span in plain.iter().flat_map(|l| l.spans.iter()) {
            assert!(span.style.fg.is_none(), "a colourless preview set a colour");
            assert!(
                span.style.bg.is_none(),
                "a colourless preview set a background"
            );
        }
        let shapes: std::collections::BTreeSet<char> =
            drawn.chars().filter(|c| !c.is_whitespace()).collect();
        assert!(
            shapes.len() > 1,
            "the glyphs have to distinguish the halves, got {shapes:?}"
        );
    }

    /// And the colour paths still colour, at each depth that has colour to give.
    #[test]
    fn a_preview_is_coloured_wherever_colour_exists() {
        let bm = art(&["####", "####"]);
        for depth in [
            theme::Depth::True,
            theme::Depth::Ansi256,
            theme::Depth::Ansi16,
        ] {
            let lines = to_lines(&bm, 4, &Theme::new(depth));
            let coloured = lines
                .iter()
                .flat_map(|l| l.spans.iter())
                .any(|s| s.style.fg.is_some());
            assert!(coloured, "{depth:?} drew no colour");
        }
    }

    /// A bitmap from ASCII art, the way `fontina-core`'s encoder tests write one: `.` is
    /// bare and `#` is full ink, so the shape under test can be read by eye.
    fn art(rows: &[&str]) -> Bitmap {
        let width = rows.first().map_or(0, |r| r.chars().count()) as u32;
        let height = rows.len() as u32;
        let mut coverage = Vec::with_capacity((width * height) as usize);
        for row in rows {
            assert_eq!(row.chars().count() as u32, width, "ragged art");
            for c in row.chars() {
                coverage.push(match c {
                    '.' => 0,
                    '-' => 85,
                    '+' => 170,
                    '#' => 255,
                    other => panic!("{other:?} is not a coverage level"),
                });
            }
        }
        Bitmap {
            width,
            height,
            coverage,
            baseline: height as f32,
            glyphs: 1,
            missing: 0,
        }
    }

    /// A bitmap of a given shape, inked in a pattern that makes every pixel distinct
    /// enough to notice being moved.
    fn shaped(width: u32, height: u32, ink: bool) -> Bitmap {
        let coverage = (0..width as usize * height as usize)
            .map(|i| if ink { ((i * 37) % 256) as u8 } else { 0 })
            .collect();
        Bitmap {
            width,
            height,
            coverage,
            baseline: height as f32,
            glyphs: 1,
            missing: 0,
        }
    }

    fn text_of(lines: &[Line<'static>]) -> String {
        lines
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The invariants a conversion has to keep, whatever it was handed. `px_rows` is a
    /// budget in pixel rows and two of them go in a cell, so the budget in text rows is
    /// half of it; a pane can never be given more rows than it asked for, because the
    /// caller draws these into a `Paragraph` that would silently scroll if it were.
    #[track_caller]
    fn check(bm: &Bitmap, px_rows: u32, what: &str) -> Vec<Line<'static>> {
        let out = to_lines(bm, px_rows, &Theme::new(theme::Depth::True));
        assert!(
            out.len() <= (px_rows as usize).div_ceil(2),
            "{what}: {} rows for a budget of {px_rows} pixel rows",
            out.len(),
        );
        assert!(
            out.len() <= (bm.height as usize).div_ceil(2),
            "{what}: {} rows out of a {}-row bitmap",
            out.len(),
            bm.height,
        );
        assert_eq!(
            out.len(),
            (px_rows.min(bm.height) as usize).div_ceil(2),
            "{what}: two pixel rows to a text row, up to the budget",
        );
        for (i, line) in out.iter().enumerate() {
            assert!(
                line.width() <= bm.width as usize,
                "{what}: row {i} is {} cells wide, the bitmap is {}",
                line.width(),
                bm.width,
            );
        }
        out
    }

    // ----- every shape a bitmap can be -----

    #[test]
    fn a_bitmap_of_any_shape_converts_without_breaking() {
        for &(w, h) in &[
            (0u32, 0u32),
            (1, 1),
            (1, 2),
            (1, 9), // one column
            (9, 1), // one row
            (40, 1),
            (1, 40),
            (0, 9), // no columns, some rows
            (9, 0), // no rows, some columns
            (2, 2),
            (3, 3),
            (80, 24),
            (200, 90), // wider and taller than any pane here
        ] {
            for ink in [true, false] {
                let bm = shaped(w, h, ink);
                for px_rows in [0u32, 1, 2, 3, 4, 5, 40, 1000] {
                    check(&bm, px_rows, &format!("{w}x{h} ink={ink} rows={px_rows}"));
                }
            }
        }
    }

    #[test]
    fn a_pane_with_no_rows_is_given_no_lines() {
        assert!(to_lines(&shaped(80, 40, true), 0, &Theme::new(theme::Depth::True)).is_empty());
        // And a bitmap with no rows produces none however much room there is.
        assert!(to_lines(&shaped(80, 0, true), 40, &Theme::new(theme::Depth::True)).is_empty());
    }

    #[test]
    fn one_pixel_is_one_cell() {
        let out = check(&art(&["#"]), 2, "a single lit pixel");
        assert_eq!(text_of(&out), "▀");
        // Unlit, it is a space that the trailing-blank elision drops altogether.
        let out = check(&art(&["."]), 2, "a single dark pixel");
        assert_eq!(text_of(&out), "");
        assert_eq!(out.len(), 1, "the row still exists");
    }

    #[test]
    fn an_odd_number_of_pixel_rows_leaves_the_bottom_half_dark() {
        // Three rows of ink into a three-pixel-row budget: two cells, and the second
        // one's lower half is outside the budget, so it is drawn dark.
        let bm = art(&["##", "##", "##"]);
        let out = check(&bm, 3, "three rows");
        assert_eq!(out.len(), 2);
        for span in &out[1].spans {
            assert_eq!(span.content.as_ref(), "▀");
            assert_eq!(
                span.style.bg,
                Some(Color::Rgb(30, 30, 30)),
                "the half outside the budget is background",
            );
        }
    }

    /// A cell is emitted only where one of its two pixel rows has ink; everything else
    /// is a run of spaces, and a run at the end of a row is dropped rather than padded.
    #[test]
    fn blank_columns_are_spaces_and_trailing_ones_are_nothing() {
        let bm = art(&["..#..", "....."]);
        let out = check(&bm, 2, "one lit pixel among five");
        assert_eq!(text_of(&out), "  ▀");
        assert_eq!(out[0].spans.len(), 2, "a run of blanks, then the cell");
        assert_eq!(out[0].spans[0].content.as_ref(), "  ");
    }

    #[test]
    fn the_two_halves_of_a_cell_are_its_two_pixel_rows() {
        let bm = art(&["#.", ".#"]);
        let out = check(&bm, 2, "a diagonal");
        let spans = &out[0].spans;
        assert_eq!(spans.len(), 2);
        // Top lit, bottom dark; then the other way round.
        assert_eq!(spans[0].style.fg, Some(Color::Rgb(230, 230, 230)));
        assert_eq!(spans[0].style.bg, Some(Color::Rgb(30, 30, 30)));
        assert_eq!(spans[1].style.fg, Some(Color::Rgb(30, 30, 30)));
        assert_eq!(spans[1].style.bg, Some(Color::Rgb(230, 230, 230)));
    }

    /// The whole conversion for a bitmap small enough to read: which cells are drawn,
    /// and the grey each half of each cell carries. Written back out as two characters a
    /// cell — the upper half then the lower — so the box in the art is still a box here.
    #[test]
    fn a_small_bitmap_converts_to_a_readable_picture() {
        #[rustfmt::skip] // it is a picture; keep it one row to a line
        let bm = art(&[
            "........",
            ".######.",
            ".#....#.",
            ".#-++-#.",
            ".#....#.",
            ".######.",
            "...--...",
        ]);
        let grey = |c: Option<Color>| match c {
            Some(Color::Rgb(v, _, _)) => match v {
                0..=30 => '.',
                31..=100 => '-',
                101..=180 => '+',
                _ => '#',
            },
            other => panic!("a cell without a colour: {other:?}"),
        };
        let mut out = String::new();
        for line in check(&bm, 14, "the box") {
            for span in &line.spans {
                if span.content.as_ref() == "▀" {
                    out.push(grey(span.style.fg));
                    out.push(grey(span.style.bg));
                } else {
                    // A run of blanks: two characters a cell here too.
                    out.push_str(&"  ".repeat(span.content.chars().count()));
                }
            }
            out.push('\n');
        }
        insta::assert_snapshot!(out);
    }

    // ----- clipping to the pane, and to the ink -----

    /// The reason `to_lines` looks for the ink at all: the top rows of a rendering are
    /// the font's empty ascent, and a pane too short to hold the whole rendering used to
    /// spend its rows on them. This is the property #66 fixed, stated as a property
    /// rather than as one face at one size.
    #[test]
    fn a_short_pane_is_spent_on_the_ink_and_not_on_the_empty_ascent() {
        // Twenty rows, ink only on rows 12..16.
        let mut rows = vec!["........"; 20];
        for row in rows.iter_mut().take(16).skip(12) {
            *row = "..####..";
        }
        let bm = art(&rows);
        for px_rows in [4u32, 6, 8, 10] {
            let out = check(&bm, px_rows, &format!("{px_rows} pixel rows"));
            assert!(
                text_of(&out).contains('▀'),
                "{px_rows} rows showed none of the ink",
            );
        }
        // Ink at the very top is found too, without running off the bitmap.
        let mut rows = vec!["........"; 20];
        rows[0] = "..####..";
        rows[1] = "..####..";
        assert!(text_of(&check(&art(&rows), 4, "ink at the top")).contains('▀'));
        // And at the very bottom.
        let mut rows = vec!["........"; 20];
        rows[18] = "..####..";
        rows[19] = "..####..";
        assert!(text_of(&check(&art(&rows), 4, "ink at the bottom")).contains('▀'));
    }

    #[test]
    fn ink_taller_than_the_pane_starts_at_the_top_of_the_ink() {
        // Ink on every row but the first two: taller than the four rows on offer, so the
        // window starts where the ink does rather than centring something that cannot be
        // centred.
        let mut rows = vec!["####"; 12];
        rows[0] = "....";
        rows[1] = "....";
        let out = check(&art(&rows), 4, "ink taller than the pane");
        assert_eq!(out.len(), 2);
        assert_eq!(text_of(&out), "▀▀▀▀\n▀▀▀▀");
    }

    #[test]
    fn a_bitmap_with_no_ink_at_all_is_a_pane_of_blank_rows() {
        let out = check(&art(&["...."; 10]), 6, "no ink");
        assert_eq!(out.len(), 3);
        assert_eq!(text_of(&out), "\n\n", "three rows, nothing on them");
    }

    /// `to_lines` is given a budget in rows and none in columns, so a bitmap wider than
    /// the pane comes back wider than the pane. That is by design — the caller clips
    /// horizontally by passing `RenderOptions::max_width`, and ratatui truncates what is
    /// left — but it means the width of the output is the width of the bitmap, never the
    /// width of the pane, and a caller that forgets `max_width` gets a line that runs off
    /// the side.
    #[test]
    fn columns_are_not_clipped_and_that_is_the_callers_job() {
        let bm = shaped(200, 4, true);
        let out = check(&bm, 4, "wider than any pane");
        assert_eq!(out.len(), 2);
        assert!(
            out.iter().all(|l| l.width() == 200),
            "the bitmap's width, not the pane's",
        );
    }

    // ----- the cache -----

    fn face(name: &str) -> FaceMetadata {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures")
            .join(name);
        fontina_core::load_file(&path).unwrap().1.remove(0)
    }

    fn small(text: &str) -> RenderOptions {
        RenderOptions {
            text: text.into(),
            size: 12.0,
            padding: 1,
            max_width: Some(40),
            ..Default::default()
        }
    }

    #[test]
    fn a_rendering_is_kept_and_reused_and_the_cache_stays_bounded() {
        let mut cache = Cache::default();
        let f = face("SourceSerif4-Regular.otf");
        let first = cache.lines(&f, &small("Aa"), 20);
        assert_eq!(cache.len(), 1);
        let again = cache.lines(&f, &small("Aa"), 20);
        assert_eq!(text_of(&first), text_of(&again));
        assert_eq!(cache.len(), 1, "the same key is the same entry");
        // Every field of the key is part of it.
        cache.lines(&f, &small("Ab"), 20);
        cache.lines(
            &f,
            &RenderOptions {
                size: 14.0,
                ..small("Aa")
            },
            20,
        );
        cache.lines(&f, &small("Aa"), 22);
        assert_eq!(cache.len(), 4);
        // And it never grows past its capacity, however far someone scrolls.
        for i in 0..200 {
            cache.lines(&f, &small(&format!("line {i}")), 20);
            assert!(cache.len() <= CAPACITY);
        }
        assert_eq!(cache.len(), CAPACITY);
        cache.clear();
        assert_eq!(cache.len(), 0);
    }

    /// What a cache is for is the work it does not do, and only the count can say so:
    /// two identical requests are one rasterisation, and the second is the same lines.
    #[test]
    fn the_same_request_twice_rasterises_once() {
        let mut cache = Cache::default();
        let f = face("SourceSerif4-Regular.otf");
        let first = cache.lines(&f, &small("Aa"), 20);
        assert_eq!(cache.rasterised(), 1);
        for _ in 0..50 {
            assert_eq!(text_of(&cache.lines(&f, &small("Aa"), 20)), text_of(&first));
        }
        assert_eq!(cache.rasterised(), 1, "a hit reached the rasteriser");
    }

    /// The shape the browser actually makes: moving an axis is a different rendering
    /// and has to be one, while scrolling back over a face already seen is not.
    #[test]
    fn a_slider_move_is_a_miss_and_a_scroll_back_is_a_hit() {
        let mut cache = Cache::default();
        let f = face("BricolageGrotesque[opsz,wdth,wght].ttf");
        let at = |wght: f32| RenderOptions {
            variations: vec![("wght".into(), wght)],
            ..small("Aa")
        };

        // Dragging the weight axis across ten stops: ten renderings, because each one
        // is a different set of outlines.
        for w in 0..10u8 {
            cache.lines(&f, &at(200.0 + f32::from(w) * 50.0), 20);
        }
        assert_eq!(cache.rasterised(), 10, "an axis move has to be a miss");

        // Scrolling back over all ten: none, because they are all still held.
        for w in 0..10u8 {
            cache.lines(&f, &at(200.0 + f32::from(w) * 50.0), 20);
        }
        assert_eq!(cache.rasterised(), 10, "and a scroll back has to be a hit");

        // Least recently used, not first in: touching the oldest entry keeps it, and
        // what falls out is whatever the reader has stopped looking at.
        let oldest = at(200.0);
        for i in 0..CAPACITY {
            cache.lines(&f, &oldest, 20);
            cache.lines(&f, &small(&format!("line {i}")), 20);
        }
        let before = cache.rasterised();
        cache.lines(&f, &oldest, 20);
        assert_eq!(
            cache.rasterised(),
            before,
            "the entry touched on every round was evicted anyway"
        );
    }

    /// Text the font cannot show is said, not drawn as a row of empty boxes.
    ///
    /// A font draws `.notdef` for every character it does not cover, and most fonts draw
    /// `.notdef` as a rectangle. A reader who types Japanese into the browser's sample
    /// text and gets three rectangles has been told something about the font, and has no
    /// way to know that is what they were told.
    #[test]
    fn a_preview_says_when_the_font_has_no_glyph_for_the_text() {
        let mut cache = Cache::default();
        let f = face("SourceSerif4-Regular.otf");

        let missing = cache.lines(&f, &small("日本語"), 20);
        let said = text_of(&missing);
        assert!(
            said.contains("3 of 3 glyph(s) not in this font"),
            "the pane says what the font cannot show:\n{said}"
        );
        assert!(
            said.lines().count() > 1,
            "and still draws the font's own answer below it"
        );

        let covered = cache.lines(&f, &small("Aa"), 20);
        assert!(
            !text_of(&covered).contains("not in this font"),
            "text the font covers gets no note"
        );
    }

    #[test]
    fn a_face_that_cannot_be_rendered_says_so_instead_of_drawing_nothing() {
        let mut cache = Cache::default();
        let mut f = face("SourceSerif4-Regular.otf");
        f.file.path = "/nonexistent/gone.otf".into();
        let lines = cache.lines(&f, &small("Aa"), 20);
        assert_eq!(lines.len(), 1);
        assert!(text_of(&lines).starts_with("preview unavailable: "));
        assert_eq!(lines[0].spans[0].style, Theme::default().bad());
        // The failure is cached too, so a broken face does not retry the read on every
        // frame the pane is drawn.
        assert_eq!(cache.len(), 1);
        cache.lines(&f, &small("Aa"), 20);
        assert_eq!(cache.len(), 1);
    }

    /// The whole path, over every fixture: a real rendering, into a real pane, at the
    /// sizes and widths a terminal actually has.
    #[test]
    fn every_fixture_fills_a_pane_of_any_size() {
        let mut cache = Cache::default();
        for name in [
            "Amiri-Regular.ttf",
            "BricolageGrotesque[opsz,wdth,wght].ttf",
            "Nabla[EDPT,EHLT].ttf",
            "SourceSerif4-Regular.otf",
            "inter-latin-400-normal.woff",
            "inter-latin-400-normal.woff2",
        ] {
            let f = face(name);
            let text = sample_for(&f);
            assert!(!text.is_empty(), "{name} has no sample text");
            for cols in [4u32, 20, 80, 200] {
                for rows in [2u32, 3, 8, 30] {
                    let opts = RenderOptions {
                        text: text.clone(),
                        size: 14.0,
                        padding: 1,
                        max_width: Some(cols),
                        ..Default::default()
                    };
                    let lines = cache.lines(&f, &opts, rows * 2);
                    assert!(
                        lines.len() <= rows as usize,
                        "{name}: {} lines into {rows} rows",
                        lines.len(),
                    );
                    assert!(
                        lines.iter().all(|l| l.width() <= cols as usize),
                        "{name}: a line wider than the {cols}-column pane",
                    );
                    assert!(
                        text_of(&lines).contains('▀'),
                        "{name} at {cols}x{rows}: an empty preview",
                    );
                }
            }
        }
    }
}
