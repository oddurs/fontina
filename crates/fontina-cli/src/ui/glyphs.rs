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
        let next = (self.block as i32 + delta).clamp(0, last) as usize;
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
        self.scroll = (self.scroll as i32 + rows).clamp(0, last_row as i32) as usize;
    }

    /// Pull the scroll position back inside a block that is now laid out `cols` wide.
    /// A pane that grew since the last keypress would otherwise start past the end and
    /// render nothing.
    pub fn clamp_scroll(&mut self, cols: usize) {
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

    fn face(name: &str) -> FaceMetadata {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures")
            .join(name);
        fontina_core::load_file(&path).unwrap().1.remove(0)
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
}
