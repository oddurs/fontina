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

//! Which characters a face actually covers, by Unicode block.
//!
//! The selection and the search live here, away from drawing, so that "find U+0041" and
//! "find Arabic" can be tested without a terminal. What the grid looks like is
//! `ui::mod`'s business.

use fontina_core::FaceMetadata;
use fontina_core::unicode::{BlockCoverage, glyph_map};

/// A face's coverage, one block at a time.
#[derive(Default)]
pub struct Glyphs {
    blocks: Vec<BlockCoverage>,
    /// Index into `blocks`.
    block: usize,
    /// First row of the character grid on show, in rows of whatever width the pane has.
    scroll: usize,
    /// The codepoint a search landed on, so the grid can point at it.
    found: Option<u32>,
}

impl Glyphs {
    pub fn for_face(face: &FaceMetadata) -> Self {
        Glyphs {
            blocks: glyph_map(&face.coverage.ranges),
            block: 0,
            scroll: 0,
            found: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    pub fn blocks(&self) -> &[BlockCoverage] {
        &self.blocks
    }

    pub fn selected(&self) -> Option<&BlockCoverage> {
        self.blocks.get(self.block)
    }

    pub fn selected_index(&self) -> usize {
        self.block
    }

    pub fn scroll_row(&self) -> usize {
        self.scroll
    }

    pub fn found(&self) -> Option<u32> {
        self.found
    }

    /// The selected block's name and extent, for a status line.
    pub fn selected_map_name(&self) -> String {
        match self.selected() {
            Some(b) => format!("{} (U+{:04X}–U+{:04X})", b.block, b.start, b.end),
            None => String::new(),
        }
    }

    /// Total codepoints covered, across every block.
    pub fn covered(&self) -> usize {
        self.blocks.iter().map(|b| b.codepoints.len()).sum()
    }

    /// Move to another block, stopping at the ends. Changing block starts at its top.
    pub fn select(&mut self, delta: i32) {
        if self.blocks.is_empty() {
            return;
        }
        let last = self.blocks.len() as i32 - 1;
        // Saturating, because the addition happens before the clamp: from any block but
        // the first, a delta near either end of i32 overflowed, which panicked in debug
        // and in release wrapped the sum so the selection went to the opposite end from
        // the one asked for.
        let next = (self.block as i32).saturating_add(delta).clamp(0, last) as usize;
        if next != self.block {
            self.block = next;
            self.scroll = 0;
            self.found = None;
        }
    }

    /// Scroll the character grid, never past the last row that has characters on it.
    pub fn scroll_by(&mut self, rows: i32, cols: usize) {
        let Some(block) = self.blocks.get(self.block) else {
            return;
        };
        let cols = cols.max(1);
        let last_row = block.codepoints.len().div_ceil(cols).saturating_sub(1);
        // Saturating for the same reason: Home and End pass i32::MAX / 2 and i32::MIN / 2,
        // and the halving that keeps that from overflowing should not be load-bearing.
        self.scroll = (self.scroll as i32)
            .saturating_add(rows)
            .clamp(0, last_row as i32) as usize;
    }

    /// Pull the scroll position back inside a block that is now laid out `cols` wide.
    ///
    /// A pane that grew since the last keypress would otherwise start past the end and
    /// render nothing. A search result is followed as well: `find` scrolled to a row
    /// counted in the columns of the frame before, so a resize used to carry the found
    /// codepoint off the grid, cursor and all, with no key that brought it back.
    pub fn clamp_scroll(&mut self, cols: usize) {
        if let Some(cp) = self.found
            && let Some(block) = self.blocks.get(self.block)
            && let Some(at) = block.codepoints.iter().position(|&c| c == cp)
        {
            self.scroll = at / cols.max(1);
            return;
        }
        self.scroll_by(0, cols);
    }

    /// Jump to a codepoint or a block.
    ///
    /// A query that parses as a codepoint — `U+0041`, `0x41`, `41`, or the character `A`
    /// itself — selects the block holding it and scrolls to it. Anything else is matched
    /// against block names. Returns whether anything was found, so the caller can say so.
    pub fn find(&mut self, query: &str, cols: usize) -> bool {
        let query = query.trim();
        if query.is_empty() {
            return false;
        }
        // A codepoint the face does not cover falls through to the name search rather
        // than failing: "0641" is a codepoint, but it could also be a block someone is
        // half-remembering.
        if let Some(cp) = parse_codepoint(query)
            && self.go_to(cp, cols)
        {
            return true;
        }
        let needle = query.to_lowercase();
        // One letter would match a block name by substring almost at random — "e" lands
        // on the first block with an "e" in it — and report success, so the reader gets
        // no signal that the jump misfired.
        if needle.chars().count() < 2 {
            return false;
        }
        if let Some(i) = self
            .blocks
            .iter()
            .position(|b| b.block.to_lowercase().contains(&needle))
        {
            self.block = i;
            self.scroll = 0;
            self.found = None;
            return true;
        }
        false
    }

    /// Select the block covering `cp` and scroll to the row it sits on.
    fn go_to(&mut self, cp: u32, cols: usize) -> bool {
        let cols = cols.max(1);
        let Some((i, at)) = self.blocks.iter().enumerate().find_map(|(i, b)| {
            b.codepoints
                .iter()
                .position(|&c| c == cp)
                .map(|pos| (i, pos))
        }) else {
            return false;
        };
        self.block = i;
        self.scroll = at / cols;
        self.found = Some(cp);
        true
    }
}

/// `U+0041`, `0x41`, `0041`, or a single character standing for itself.
fn parse_codepoint(s: &str) -> Option<u32> {
    let mut chars = s.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) {
        // A lone character is the character. Reading "a" as U+000A would be defensible
        // arithmetic and useless behaviour: someone who wants a codepoint by number can
        // write `U+000A`, `0x0A` or `0a`, and someone who types one letter into a box
        // that finds glyphs means that letter.
        return Some(c as u32);
    }
    let hex = s
        .strip_prefix("U+")
        .or_else(|| s.strip_prefix("u+"))
        .or_else(|| s.strip_prefix("0x"))
        .or_else(|| s.strip_prefix("0X"))
        .unwrap_or(s);
    u32::from_str_radix(hex, 16).ok().filter(|c| *c <= 0x10FFFF)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// A fixture, parsed once for the whole test binary and cloned from there.
    ///
    /// Parsing one of these in a debug build is close to a second, and the tests below
    /// ask for the same handful of faces a hundred times over. Cloning a parsed face is
    /// a memcpy; parsing it again is not, and the total was four minutes.
    fn face(name: &str) -> FaceMetadata {
        static PARSED: std::sync::OnceLock<
            std::sync::Mutex<std::collections::HashMap<String, FaceMetadata>>,
        > = std::sync::OnceLock::new();
        let cache = PARSED.get_or_init(Default::default);
        let mut cache = cache.lock().unwrap_or_else(|e| e.into_inner());
        cache
            .entry(name.to_string())
            .or_insert_with(|| {
                let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../../fixtures")
                    .join(name);
                fontina_core::load_file(&path).unwrap().1.remove(0)
            })
            .clone()
    }

    #[test]
    fn a_codepoint_is_read_in_every_spelling() {
        assert_eq!(parse_codepoint("U+0041"), Some(0x41));
        assert_eq!(parse_codepoint("u+0041"), Some(0x41));
        assert_eq!(parse_codepoint("0x41"), Some(0x41));
        assert_eq!(parse_codepoint("41"), Some(0x41));
        assert_eq!(parse_codepoint("0641"), Some(0x641));
        // Beyond the last plane is not a codepoint.
        assert_eq!(parse_codepoint("110000"), None);
        // A single character stands for itself, hex digit or not: someone typing one
        // letter into a box that finds glyphs means that letter.
        assert_eq!(parse_codepoint("Ω"), Some('Ω' as u32));
        assert_eq!(parse_codepoint("ب"), Some('ب' as u32));
        assert_eq!(parse_codepoint("a"), Some('a' as u32));
        assert_eq!(parse_codepoint("f"), Some('f' as u32));
        // Two or more hex digits are a number again.
        assert_eq!(parse_codepoint("0a"), Some(0x0A));
        // Words are not codepoints.
        assert_eq!(parse_codepoint("Arabic"), None);
    }

    #[test]
    fn a_face_reports_the_blocks_it_covers() {
        let g = Glyphs::for_face(&face("Amiri-Regular.ttf"));
        assert!(!g.is_empty());
        assert!(
            g.blocks().iter().any(|b| b.block.contains("Arabic")),
            "Amiri covers Arabic"
        );
        assert_eq!(
            g.covered(),
            g.blocks().iter().map(|b| b.codepoints.len()).sum::<usize>()
        );
    }

    #[test]
    fn searching_for_a_codepoint_selects_the_block_holding_it() {
        let f = face("Amiri-Regular.ttf");
        let mut g = Glyphs::for_face(&f);
        assert!(g.find("U+0041", 16), "Amiri covers Latin A");
        assert_eq!(g.found(), Some(0x41));
        let block = g.selected().unwrap();
        assert!(
            block.codepoints.contains(&0x41),
            "the selected block holds the codepoint"
        );
    }

    #[test]
    fn searching_for_a_block_selects_it_by_name() {
        let mut g = Glyphs::for_face(&face("Amiri-Regular.ttf"));
        assert!(g.find("arabic", 16));
        assert!(
            g.selected()
                .unwrap()
                .block
                .to_lowercase()
                .contains("arabic")
        );
        assert_eq!(
            g.found(),
            None,
            "a block match points at no single character"
        );
    }

    #[test]
    fn a_search_that_finds_nothing_changes_nothing() {
        let mut g = Glyphs::for_face(&face("Amiri-Regular.ttf"));
        g.select(1);
        let (before, scroll) = (g.selected_index(), g.scroll_row());
        assert!(!g.find("Tibetan", 16));
        assert!(!g.find("U+10FFFD", 16));
        assert!(!g.find("", 16));
        assert_eq!(g.selected_index(), before);
        assert_eq!(g.scroll_row(), scroll);
    }

    #[test]
    fn selection_and_scrolling_stay_in_range() {
        let mut g = Glyphs::for_face(&face("Amiri-Regular.ttf"));
        g.select(-100);
        assert_eq!(g.selected_index(), 0);
        g.select(10_000);
        assert_eq!(g.selected_index(), g.blocks().len() - 1);

        g.scroll_by(-50, 16);
        assert_eq!(g.scroll_row(), 0);
        g.scroll_by(100_000, 16);
        let covered = g.selected().unwrap().codepoints.len();
        assert_eq!(g.scroll_row(), covered.div_ceil(16) - 1);

        // A narrower pane means more rows, and the cap follows it.
        g.scroll_by(100_000, 4);
        assert_eq!(g.scroll_row(), covered.div_ceil(4) - 1);
    }

    #[test]
    fn changing_block_starts_at_the_top() {
        let mut g = Glyphs::for_face(&face("Amiri-Regular.ttf"));
        g.scroll_by(5, 16);
        assert!(g.scroll_row() > 0);
        g.select(1);
        assert_eq!(g.scroll_row(), 0, "a new block starts at its first row");
    }

    /// An extreme delta stops at an end, rather than overflowing on the way to it.
    ///
    /// The addition happened before the clamp, so from any block but the first this
    /// panicked in debug and, in release, wrapped so far that the selection landed at the
    /// opposite end from the one asked for. Home and End reach `scroll_by` with
    /// `i32::MAX / 2` for the same reason; the halving is no longer what saves it.
    #[test]
    fn an_extreme_delta_stops_at_an_end() {
        let mut g = Glyphs::for_face(&face("Amiri-Regular.ttf"));
        assert!(
            g.blocks().len() > 2,
            "the fixture has blocks to move between"
        );
        g.select(1);
        let from = g.selected_index();
        assert!(from > 0, "not sitting on the first block");

        g.select(i32::MAX);
        assert_eq!(
            g.selected_index(),
            g.blocks().len() - 1,
            "forwards, to the last"
        );
        g.select(i32::MIN);
        assert_eq!(g.selected_index(), 0, "backwards, to the first");

        g.scroll_by(i32::MAX, 16);
        g.scroll_by(i32::MIN, 16);
        assert_eq!(g.scroll_row(), 0);
    }

    /// A resize keeps the codepoint that was searched for on the grid.
    ///
    /// `find` scrolls to a row counted in the columns of the frame it was called from.
    /// A resize re-lays the block at a different width, so that row holds something else
    /// entirely: widening moved the grid past the found codepoint and narrowing left it
    /// dozens of rows above, with `found()` still naming it and no key that returned to
    /// it.
    #[test]
    fn a_resize_keeps_the_found_codepoint_on_the_grid() {
        for (before, after) in [(16, 64), (64, 16), (7, 61), (61, 7), (32, 32)] {
            let mut g = Glyphs::for_face(&face("Amiri-Regular.ttf"));
            let cp = g
                .blocks()
                .iter()
                .flat_map(|b| b.codepoints.iter().copied())
                .nth(300)
                .expect("the fixture covers enough characters to scroll");
            assert!(
                g.find(&format!("U+{cp:04X}"), before),
                "U+{cp:04X} is there"
            );
            g.clamp_scroll(after);

            let block = g.selected().expect("a block is selected");
            let at = block
                .codepoints
                .iter()
                .position(|&c| c == cp)
                .expect("the found codepoint is in the selected block");
            assert_eq!(
                g.scroll_row(),
                at / after,
                "U+{cp:04X} left the grid when {before} columns became {after}"
            );
        }
    }

    // ----- terminal sizes -----
    //
    // The grid is laid out for whatever pane the last frame had, and the reader resizes
    // the terminal between keypresses. So the arithmetic has to hold for a pane with no
    // room for a single cell, for one cell, and for more cells than any block has
    // characters — and it has to hold when the width changes underneath a scroll that
    // was counted in the old one.

    /// Grid widths in cells: none, one, an awkward few, a usual pane, a wall of them,
    /// and the arithmetic itself.
    const WIDTHS: [usize; 8] = [0, 1, 2, 3, 16, 64, 4096, usize::MAX];

    /// The deltas `ui::mod` sends for j, k, PageDown, PageUp, Home and End.
    const SCROLL_KEYS: [i32; 6] = [1, -1, 10, -10, i32::MIN / 2, i32::MAX / 2];

    /// A face that covers exactly these ranges, and is a real font in every other
    /// respect. The map reads nothing but the coverage, so this is how three codepoints
    /// — or sixty thousand — get put in front of it.
    fn face_covering(ranges: Vec<[u32; 2]>) -> FaceMetadata {
        let mut f = face("Amiri-Regular.ttf");
        f.coverage.ranges = ranges;
        f
    }

    /// The invariant behind every scroll: the first row on show holds characters. A
    /// scroll past the end draws an empty grid, which reads as a face covering nothing.
    fn first_row_holds_characters(g: &Glyphs, cols: usize) -> bool {
        let Some(block) = g.selected() else {
            return g.scroll_row() == 0;
        };
        g.scroll_row().saturating_mul(cols.max(1)) < block.codepoints.len()
    }

    #[test]
    fn no_pane_width_scrolls_the_grid_out_of_its_block() {
        for cols in WIDTHS {
            for ranges in [
                vec![[0x41, 0x43]],
                vec![[0x4E00, 0x9FFF]],
                vec![[0x20, 0x7E], [0x600, 0x6FF], [0x1EE00, 0x1EEFF]],
            ] {
                let mut g = Glyphs::for_face(&face_covering(ranges));
                for _ in 0..3 {
                    for key in SCROLL_KEYS {
                        g.scroll_by(key, cols);
                        assert!(
                            first_row_holds_characters(&g, cols),
                            "scrolled to row {} at {cols} column(s)",
                            g.scroll_row()
                        );
                    }
                    g.select(1);
                    assert!(first_row_holds_characters(&g, cols));
                }
            }
        }
    }

    /// `cols.max(1)`, twice over: a pane with no room for a cell is laid out as one
    /// cell rather than dividing by zero, in the scrolling and in the search alike.
    #[test]
    fn a_pane_with_no_room_for_a_cell_is_laid_out_as_one() {
        let f = face("Amiri-Regular.ttf");
        let (mut none, mut one) = (Glyphs::for_face(&f), Glyphs::for_face(&f));
        for key in SCROLL_KEYS {
            none.scroll_by(key, 0);
            one.scroll_by(key, 1);
            assert_eq!(none.scroll_row(), one.scroll_row());
        }
        none.clamp_scroll(0);
        one.clamp_scroll(1);
        assert_eq!(none.scroll_row(), one.scroll_row());
        assert!(none.find("U+0641", 0) && one.find("U+0641", 1));
        assert_eq!(none.scroll_row(), one.scroll_row());
        assert_eq!(none.selected_index(), one.selected_index());
    }

    /// The end of a block is the last row that has characters on it, and no other, at
    /// every width. One row narrower and the last characters are unreachable; one wider
    /// and the grid draws blank.
    #[test]
    fn the_end_key_lands_on_the_last_row_that_has_characters() {
        for cols in [1usize, 2, 3, 16, 64, 4096] {
            let mut g = Glyphs::for_face(&face("Amiri-Regular.ttf"));
            for _ in 0..g.blocks().len() {
                g.scroll_by(i32::MAX / 2, cols);
                let n = g.selected().unwrap().codepoints.len();
                assert_eq!(g.scroll_row(), n.div_ceil(cols) - 1);
                assert!(g.scroll_row() * cols < n, "the last row has characters");
                assert!(
                    (g.scroll_row() + 1) * cols >= n,
                    "and it is the last row that does"
                );
                g.select(1);
            }
        }
    }

    /// Paging down until it stops and then up the same number of times lands back at
    /// the top: the step is the same in both directions, and the clamp at each end
    /// swallows the overshoot rather than losing a row to it.
    #[test]
    fn paging_to_the_end_and_back_lands_where_it_started() {
        for cols in [1usize, 2, 3, 16, 64] {
            for ranges in [vec![[0x41, 0x43]], vec![[0x4E00, 0x9FFF]]] {
                let mut g = Glyphs::for_face(&face_covering(ranges));
                let mut pages = 0;
                loop {
                    let before = g.scroll_row();
                    g.scroll_by(10, cols);
                    if g.scroll_row() == before {
                        break;
                    }
                    pages += 1;
                    assert!(pages < 100_000, "paging never reached the end");
                }
                let n = g.selected().unwrap().codepoints.len();
                assert_eq!(g.scroll_row(), n.div_ceil(cols) - 1);
                for _ in 0..pages {
                    g.scroll_by(-10, cols);
                }
                assert_eq!(
                    g.scroll_row(),
                    0,
                    "paged down {pages} time(s) at {cols} columns and back up as many"
                );
            }
        }
        // Home and End are the same round trip in one keypress each.
        let mut g = Glyphs::for_face(&face("Amiri-Regular.ttf"));
        g.scroll_by(i32::MAX / 2, 16);
        let end = g.scroll_row();
        assert!(end > 0);
        g.scroll_by(i32::MIN / 2, 16);
        assert_eq!(g.scroll_row(), 0);
        g.scroll_by(i32::MAX / 2, 16);
        assert_eq!(g.scroll_row(), end);
    }

    /// `a_resize_pulls_the_scroll_back_into_range` in `ui::mod` is one case of this;
    /// every width to every other width is the rule it was an example of. The scroll
    /// has to survive the trip, and the end of the block has to stay reachable after it.
    #[test]
    fn a_resize_from_any_width_to_any_other_keeps_the_grid_on_screen() {
        let f = face("Amiri-Regular.ttf");
        for from in WIDTHS {
            for to in WIDTHS {
                let mut g = Glyphs::for_face(&f);
                // The widest block this face has, so there is a long way to fall.
                g.select(g.blocks().len() as i32);
                g.scroll_by(i32::MAX / 2, from);
                g.clamp_scroll(to);
                assert!(
                    first_row_holds_characters(&g, to),
                    "{from} columns to {to} left the grid at row {}",
                    g.scroll_row()
                );
                g.scroll_by(i32::MAX / 2, to);
                let n = g.selected().unwrap().codepoints.len();
                assert_eq!(g.scroll_row(), n.div_ceil(to.max(1)) - 1);
            }
        }
    }

    /// Every key the map answers, in a fixed order nobody chose, with the pane resized
    /// under it. A fixed sequence rather than a random one: a test that fails on some
    /// runs and not others tells nobody anything.
    #[test]
    fn no_sequence_of_keys_leaves_the_map_outside_itself() {
        let mut g = Glyphs::for_face(&face("Amiri-Regular.ttf"));
        let mut cols = 16;
        let mut state = 0x2545_F491u32;
        // Eight hundred, not five thousand: every one of the five actions is reached
        // hundreds of times, and `find` walks the whole coverage on each visit, which in
        // a debug build was two minutes of the test suite for the last four thousand
        // repetitions of a sequence that had already proved its point.
        for _ in 0..800 {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            match (state >> 3) % 5 {
                0 => g.select(1),
                1 => g.select(-1),
                2 => g.scroll_by(
                    SCROLL_KEYS[(state >> 16) as usize % SCROLL_KEYS.len()],
                    cols,
                ),
                3 => {
                    // A resize, and the frame that follows it.
                    cols = WIDTHS[(state >> 8) as usize % WIDTHS.len()];
                    g.clamp_scroll(cols);
                }
                _ => {
                    g.find("U+0641", cols);
                }
            }
            assert!(g.selected_index() < g.blocks().len());
            assert!(first_row_holds_characters(&g, cols));
        }
    }

    // ----- data at the extremes -----

    #[test]
    fn a_face_covering_three_codepoints_is_one_block_and_one_row() {
        let mut g = Glyphs::for_face(&face_covering(vec![[0x41, 0x43]]));
        assert!(!g.is_empty());
        assert_eq!(g.blocks().len(), 1);
        assert_eq!(g.covered(), 3);
        assert_eq!(g.selected_map_name(), "Basic Latin (U+0000–U+007F)");
        for cols in WIDTHS {
            let mut g = Glyphs::for_face(&face_covering(vec![[0x41, 0x43]]));
            g.scroll_by(i32::MAX / 2, cols);
            assert_eq!(g.scroll_row(), 3usize.div_ceil(cols.max(1)) - 1);
            g.select(1);
            assert_eq!(g.selected_index(), 0, "there is nowhere else to go");
            assert_eq!(g.scroll_row(), 3usize.div_ceil(cols.max(1)) - 1);
        }
        assert!(g.find("U+0042", 16));
        assert!(
            !g.find("U+0044", 16),
            "one past the coverage is not covered"
        );
    }

    /// Four orders of magnitude up from the three above: the whole of CJK Unified
    /// Ideographs, in one block.
    #[test]
    fn a_face_covering_tens_of_thousands_of_codepoints_reaches_the_last_one() {
        let mut g = Glyphs::for_face(&face_covering(vec![[0x4E00, 0x9FFF]]));
        let covered = g.covered();
        assert_eq!(covered, (0x9FFF - 0x4E00 + 1) as usize);
        assert_eq!(g.blocks().len(), 1);
        for cols in [1usize, 16, 4096] {
            g.scroll_by(i32::MIN / 2, cols);
            assert_eq!(g.scroll_row(), 0);
            g.scroll_by(i32::MAX / 2, cols);
            assert_eq!(g.scroll_row(), covered.div_ceil(cols) - 1);
        }
        assert!(g.find("U+9FFF", 16));
        assert_eq!(g.found(), Some(0x9FFF));
        assert_eq!(g.scroll_row(), (covered - 1) / 16);
    }

    /// The whole Basic Multilingual Plane: a hundred and sixty blocks, so the list of
    /// blocks is longer than any terminal and every one of them has to be reachable.
    #[test]
    fn a_face_covering_a_whole_plane_walks_every_block_it_has() {
        let mut g = Glyphs::for_face(&face_covering(vec![[0x0000, 0xFFFF]]));
        let last = g.blocks().len() - 1;
        assert!(last > 100, "{} blocks", last + 1);
        assert_eq!(
            g.covered(),
            0x10000 - 0x800,
            "surrogates are not characters, so they are not covered"
        );
        g.select(i32::MAX / 2);
        assert_eq!(g.selected_index(), last);
        g.select(i32::MIN / 2);
        assert_eq!(g.selected_index(), 0);
        for i in 0..=last {
            assert_eq!(g.selected_index(), i);
            assert_eq!(g.scroll_row(), 0, "block {i} did not start at its top");
            g.scroll_by(i32::MAX / 2, 16);
            let n = g.selected().unwrap().codepoints.len();
            assert_eq!(g.scroll_row(), n.div_ceil(16) - 1);
            g.select(1);
        }
    }

    #[test]
    fn a_face_that_covers_nothing_answers_every_key_without_moving() {
        let mut g = Glyphs::for_face(&face_covering(Vec::new()));
        assert!(g.is_empty());
        assert!(g.selected().is_none());
        assert!(g.blocks().is_empty());
        assert_eq!(g.covered(), 0);
        assert_eq!(g.selected_map_name(), "");
        for cols in WIDTHS {
            for key in SCROLL_KEYS {
                g.scroll_by(key, cols);
                g.select(key);
                g.clamp_scroll(cols);
            }
            assert!(!g.find("U+0041", cols));
            assert!(!g.find("Latin", cols));
            assert_eq!(g.selected_index(), 0);
            assert_eq!(g.scroll_row(), 0);
            assert_eq!(g.found(), None);
        }
        // And a map that was never given a face at all behaves the same way.
        let empty = Glyphs::default();
        assert!(empty.is_empty());
        assert_eq!(empty.covered(), 0);
        assert_eq!(empty.selected_map_name(), "");

        // So does a face whose coverage holds nothing that is a character. `ui::mod`
        // is what turns an empty map into "this face maps no codepoints" rather than
        // an empty screen, and it asks `is_empty`.
        for ranges in [vec![[0xD800, 0xDFFF]], vec![[0x11_0000, 0x11_0005]]] {
            let g = Glyphs::for_face(&face_covering(ranges));
            assert!(g.is_empty());
            assert_eq!(g.covered(), 0);
        }
    }

    // ----- searching -----

    #[test]
    fn a_codepoint_the_face_does_not_cover_is_not_found() {
        let mut g = Glyphs::for_face(&face("Amiri-Regular.ttf"));
        g.select(3);
        g.scroll_by(2, 16);
        let before = (g.selected_index(), g.scroll_row());
        for query in ["U+4E00", "4E00", "0x4E00", "中"] {
            assert!(!g.find(query, 16), "Amiri covers no CJK, so not {query:?}");
        }
        assert_eq!((g.selected_index(), g.scroll_row()), before);
        assert_eq!(g.found(), None);
    }

    #[test]
    fn a_number_that_is_not_a_codepoint_is_not_searched_for_as_one() {
        assert_eq!(parse_codepoint("110000"), None, "past the last plane");
        assert_eq!(parse_codepoint("U+110000"), None);
        assert_eq!(parse_codepoint("0x110000"), None);
        assert_eq!(parse_codepoint("FFFFFFFF"), None);
        assert_eq!(parse_codepoint("FFFFFFFFFFFF"), None, "wider than a u32");
        assert_eq!(parse_codepoint("-1"), None);
        assert_eq!(parse_codepoint("0x"), None, "a prefix and nothing else");
        assert_eq!(parse_codepoint("U+"), None);
        assert_eq!(parse_codepoint("D800"), Some(0xD800), "a surrogate parses");

        let mut g = Glyphs::for_face(&face("Amiri-Regular.ttf"));
        for query in ["110000", "U+110000", "FFFFFFFF", "0x", "U+", "D800"] {
            assert!(!g.find(query, 16), "{query:?} found something");
        }
        assert_eq!(g.selected_index(), 0);
        assert_eq!(g.found(), None);

        // Falling through to the block names is the documented behaviour and it is
        // wider than it looks: "-1" is not a codepoint, and "Latin-1 Supplement" is a
        // block whose name contains it. The jump is honest — it says it found a block
        // and it points at one — but nothing about the query said "Latin".
        assert!(g.find("-1", 16));
        assert_eq!(g.selected().unwrap().block, "Latin-1 Supplement");
        assert_eq!(g.found(), None, "a block match points at no character");
    }

    /// The search scrolls to the codepoint, so the codepoint has to be on the row it
    /// scrolls to — at every width, in every block, at the top, the middle and the end.
    #[test]
    fn a_found_codepoint_is_on_the_first_row_on_show() {
        let f = face("Amiri-Regular.ttf");
        let probes: Vec<u32> = Glyphs::for_face(&f)
            .blocks()
            .iter()
            .flat_map(|b| {
                let n = b.codepoints.len();
                [b.codepoints[0], b.codepoints[n / 2], b.codepoints[n - 1]]
            })
            .collect();
        for cols in [1usize, 2, 3, 16, 64, 4096] {
            let mut g = Glyphs::for_face(&f);
            for cp in &probes {
                assert!(g.find(&format!("U+{cp:04X}"), cols), "U+{cp:04X}");
                assert_eq!(g.found(), Some(*cp));
                let block = g.selected().unwrap();
                assert!(block.codepoints.contains(cp), "the block holds it");
                let row = &block.codepoints[g.scroll_row() * cols..];
                assert!(
                    row.iter().take(cols).any(|c| c == cp),
                    "U+{cp:04X} is not on the first row on show at {cols} column(s)"
                );
            }
        }
    }

    /// One letter is that letter and nothing else: matching block names by a single
    /// character would report success for a jump that landed anywhere.
    #[test]
    fn one_letter_never_lands_on_a_block_by_accident() {
        let mut g = Glyphs::for_face(&face_covering(vec![[0x41, 0x43]]));
        assert_eq!(g.selected().unwrap().block, "Basic Latin");
        assert!(
            !g.find("a", 16),
            "\"a\" is a character this face does not cover, not \"Basic Latin\""
        );
        assert!(!g.find("t", 16));
        assert!(g.find("la", 16), "two letters are a block name again");
        assert_eq!(g.found(), None);
        assert!(g.find("A", 16), "and a covered character is itself");
        assert_eq!(g.found(), Some(0x41));
    }

    /// The query is trimmed, so the space character cannot be typed as itself. It is
    /// still reachable by number, which is the whole reason the numeric spellings exist.
    #[test]
    fn a_query_of_nothing_but_space_finds_nothing() {
        let mut g = Glyphs::for_face(&face_covering(vec![[0x20, 0x22]]));
        assert!(!g.find("", 16));
        assert!(!g.find(" ", 16));
        assert!(!g.find("   ", 16));
        assert!(!g.find("\t\n", 16));
        assert_eq!(g.found(), None);
        assert!(g.find("U+0020", 16));
        assert_eq!(g.found(), Some(0x20));
        assert!(
            g.find("  U+0021  ", 16),
            "a query is trimmed to what it says"
        );
        assert_eq!(g.found(), Some(0x21));
    }

    #[test]
    fn changing_block_forgets_the_codepoint_the_search_pointed_at() {
        let mut g = Glyphs::for_face(&face("Amiri-Regular.ttf"));
        assert!(g.find("U+0041", 16));
        assert_eq!(g.found(), Some(0x41));
        g.select(0);
        assert_eq!(g.found(), Some(0x41), "a select that moves nothing");
        g.scroll_by(3, 16);
        assert_eq!(g.found(), Some(0x41), "scrolling does not cancel a search");
        g.select(1);
        assert_eq!(
            g.found(),
            None,
            "another block has no found codepoint on it"
        );
        assert_eq!(g.scroll_row(), 0);
    }

    /// The status line is built from this, so its shape is a promise.
    #[test]
    fn a_block_names_itself_and_its_extent() {
        let mut g = Glyphs::for_face(&face("Amiri-Regular.ttf"));
        assert!(g.find("arabic supplement", 16));
        assert_eq!(g.selected_map_name(), "Arabic Supplement (U+0750–U+077F)");
        let block = g.selected().unwrap();
        assert!(block.start <= block.codepoints[0]);
        assert!(*block.codepoints.last().unwrap() <= block.end);
    }

    // ----- frames -----
    //
    // The map is a mode rather than a pane: it covers the browser and has to stay
    // inside the area it was handed. `ui::mod` draws it, so these go through a frame.

    /// The browser over the fixture fonts. The first family is selected, which is the
    /// face the map opens on.
    fn app() -> crate::ui::App {
        let mut index = fontina_core::Index::open_in_memory().unwrap();
        let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures");
        fontina_core::scan::scan(&mut index, &[fixtures], &Default::default()).unwrap();
        let mut app = crate::ui::App::new(index).unwrap();
        // A blank sample text keeps the rasteriser out of these frames; what the
        // preview draws has its own tests in `ui::mod`.
        app.preview_text = Some(" ".into());
        app
    }

    /// The browser drawn into an in-memory terminal, one string per terminal row.
    fn frame(app: &mut crate::ui::App, width: u16, height: u16) -> Vec<String> {
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(width, height)).unwrap();
        terminal.draw(|f| app.draw(f)).unwrap();
        let buffer = terminal.backend().buffer().clone();
        (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect()
    }

    /// Every terminal size the map can be asked to draw in: one column, one row, no
    /// usable area at all once the border has taken its two columns, a pane narrower
    /// than one cell of the grid, and a wall of them. Nothing may panic, and the map
    /// must stay inside the pane it was given — the key line at the foot of the screen
    /// belongs to the browser underneath, and is the same row either way.
    #[test]
    fn the_glyph_map_draws_at_any_terminal_size() {
        // 36 columns leave the grid pane nothing at all once the block list has had
        // its 34; 40, 43 and 45 leave it less than the seven the codepoint label needs,
        // and 46 is the first width with room for one cell of the grid beside it.
        const SIZES: [(u16, u16); 19] = [
            (1, 1),
            (1, 2),
            (2, 1),
            (2, 20),
            (1, 40),
            (40, 1),
            (3, 3),
            (8, 3),
            (20, 4),
            (34, 10),
            (35, 10),
            (36, 10),
            (40, 20),
            (43, 20),
            (45, 20),
            (46, 20),
            (80, 24),
            (200, 60),
            (400, 120),
        ];
        for (w, h) in SIZES {
            let mut app = app();
            let closed = frame(&mut app, w, h);
            app.open_glyphs();
            assert!(app.glyphs.is_some(), "the first fixture maps codepoints");
            let open = frame(&mut app, w, h);
            assert_eq!(
                open.len(),
                h as usize,
                "one row per terminal line on {w}x{h}"
            );
            for row in &open {
                assert!(
                    row.chars().count() <= w as usize,
                    "a row is wider than the {w}-column terminal it is drawn in"
                );
            }
            if h >= 5 {
                assert_eq!(
                    open.last(),
                    closed.last(),
                    "the map drew over the key line on a {w}x{h} terminal"
                );
            }
            // And what the frame left behind is a layout the next keypress can use.
            let cols = app.glyph_cols;
            assert!(cols >= 1, "a grid is at least one cell wide");
            let map = app.glyphs.as_ref().unwrap();
            assert!(
                first_row_holds_characters(map, cols),
                "a {w}x{h} frame left the grid past the end of its block"
            );
        }
    }

    /// Every cell outside the pane the map was handed belongs to something else. The
    /// panes underneath are drawn first and the map goes over them, so a cell it writes
    /// outside its own area is a pane it has silently eaten.
    #[test]
    fn the_glyph_map_writes_nothing_outside_the_pane_it_is_given() {
        use ratatui::layout::{Position, Rect};
        let mut app = app();
        app.open_glyphs();
        for pane in [
            Rect::new(3, 2, 60, 20),
            Rect::new(0, 0, 1, 1),
            Rect::new(10, 10, 2, 2),
            Rect::new(5, 1, 40, 1),
            Rect::new(0, 0, 80, 30),
            Rect::new(79, 29, 1, 1),
        ] {
            let mut terminal =
                ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 30)).unwrap();
            terminal.draw(|f| app.draw_glyphs(f, pane)).unwrap();
            let buffer = terminal.backend().buffer().clone();
            for y in 0..30 {
                for x in 0..80 {
                    if pane.contains(Position::new(x, y)) {
                        continue;
                    }
                    let cell = &buffer[(x, y)];
                    assert!(
                        cell.symbol() == " "
                            && cell.fg == ratatui::style::Color::Reset
                            && cell.bg == ratatui::style::Color::Reset,
                        "the map wrote {:?} at ({x}, {y}), outside {pane:?}",
                        cell.symbol()
                    );
                }
            }
        }
    }

    /// The cursor a search leaves behind. Every cell in the grid is two columns wide
    /// whatever it holds, so that the rows line up whether the block is Latin, CJK or
    /// combining marks; the cursor is one of those cells, both columns of it, and
    /// nothing else on the screen wears it. `ui::mod` uses a cyan background in exactly
    /// one place, so counting them counts cursors.
    #[test]
    fn the_search_puts_one_block_cursor_on_the_codepoint_it_found() {
        let mut app = app();
        app.open_glyphs();

        /// Where the cursor is, as (row, column, what is printed there).
        fn cursor(app: &mut crate::ui::App) -> Vec<(u16, u16, String)> {
            let mut terminal =
                ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 40)).unwrap();
            terminal.draw(|f| app.draw(f)).unwrap();
            let buffer = terminal.backend().buffer().clone();
            (0..buffer.area.height)
                .flat_map(|y| (0..buffer.area.width).map(move |x| (x, y)))
                .filter(|&(x, y)| buffer[(x, y)].bg == ratatui::style::Color::Cyan)
                .map(|(x, y)| (y, x, buffer[(x, y)].symbol().to_string()))
                .collect()
        }

        assert!(cursor(&mut app).is_empty(), "nothing searched for yet");
        let cols = app.glyph_cols;
        assert!(app.glyphs.as_mut().unwrap().find("U+0641", cols));

        let on = cursor(&mut app);
        assert_eq!(
            on.len(),
            2,
            "the cursor is one cell, and a cell is two wide"
        );
        assert_eq!(on[0].0, on[1].0, "on one row");
        assert_eq!(on[1].1, on[0].1 + 1, "in two columns side by side");
        assert_eq!(on[0].2, "\u{641}", "the character that was searched for");

        // A block found by name points at no character, so it draws no cursor.
        assert!(app.glyphs.as_mut().unwrap().find("arabic supp", cols));
        assert!(cursor(&mut app).is_empty());
    }

    /// The list of blocks is longer than most terminals. Whichever block is selected
    /// has to be one of the ones drawn, or the grid on the right belongs to a block the
    /// reader cannot see the name of.
    #[test]
    fn the_selected_block_is_always_one_of_the_ones_drawn() {
        let mut app = app();
        app.open_glyphs();
        let names: Vec<String> = app
            .glyphs
            .as_ref()
            .unwrap()
            .blocks()
            .iter()
            .map(|b| b.block.clone())
            .collect();
        assert!(names.len() > 10, "this face has blocks to scroll through");
        for height in [6u16, 24] {
            for (i, name) in names.iter().enumerate() {
                app.glyphs = None;
                app.open_glyphs();
                app.glyphs.as_mut().unwrap().select(i as i32);
                assert_eq!(app.glyphs.as_ref().unwrap().selected_index(), i);
                let drawn = frame(&mut app, 120, height);
                let marked: Vec<&String> = drawn
                    .iter()
                    .filter(|l| l.chars().nth(1) == Some('>'))
                    .collect();
                assert_eq!(marked.len(), 1, "one block is marked, at 120x{height}");
                let head: String = name.chars().take(8).collect();
                assert!(
                    marked[0].contains(&head),
                    "block {i} ({name}) is selected but {:?} is what is marked at 120x{height}",
                    marked[0].trim()
                );
            }
        }
    }
}
