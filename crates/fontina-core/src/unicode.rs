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

//! Unicode script coverage and range compression for `unicode-range`.

use crate::model::{Coverage, ScriptCoverage};
use std::collections::HashMap;

/// Build coverage from a sorted, deduplicated iterator of codepoints.
pub fn coverage_from_codepoints(mut cps: Vec<u32>) -> Coverage {
    cps.sort_unstable();
    cps.dedup();
    let mut scripts: HashMap<&'static str, u32> = HashMap::new();
    let mut ranges: Vec<[u32; 2]> = Vec::new();
    for &cp in &cps {
        if let Some(ch) = char::from_u32(cp) {
            let script = unicode_script::Script::from(ch).short_name();
            *scripts.entry(script).or_insert(0) += 1;
        }
        match ranges.last_mut() {
            Some(last) if last[1] + 1 == cp => last[1] = cp,
            _ => ranges.push([cp, cp]),
        }
    }
    let mut scripts: Vec<ScriptCoverage> = scripts
        .into_iter()
        .map(|(script, codepoints)| ScriptCoverage {
            script: script.to_string(),
            codepoints,
        })
        .collect();
    scripts.sort_by(|a, b| {
        b.codepoints
            .cmp(&a.codepoints)
            .then_with(|| a.script.cmp(&b.script))
    });
    Coverage {
        codepoints: cps.len() as u32,
        scripts,
        ranges,
    }
}

/// How to show one covered codepoint in a fixed grid of cells.
///
/// A coverage grid has to line up, and a great many codepoints a font legitimately
/// covers will not line up on their own. Control characters move the cursor; format
/// characters such as U+202E RIGHT-TO-LEFT OVERRIDE reverse everything after them on the
/// line; combining marks stack onto whatever came before instead of taking a cell of
/// their own; and CJK, Hangul and emoji take two cells rather than one.
///
/// `char::is_control` catches only the first of those four.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    /// What to print: the character itself, or `\u{FFFD}` when it has no standalone shape.
    pub glyph: char,
    /// Terminal columns `glyph` occupies, 1 or 2.
    pub width: usize,
}

/// Decide how to show a codepoint in a grid.
pub fn cell_for(cp: u32) -> Cell {
    let Some(ch) = char::from_u32(cp) else {
        return Cell {
            glyph: '\u{FFFD}',
            width: 1,
        };
    };
    match unicode_width::UnicodeWidthChar::width(ch) {
        // Two cells: CJK, Hangul, emoji and the rest of the East Asian Wide set.
        Some(2) => Cell {
            glyph: ch,
            width: 2,
        },
        // One cell: an ordinary printable character.
        Some(1) => Cell {
            glyph: ch,
            width: 1,
        },
        // Zero cells (combining marks, format characters) or none at all (controls).
        // Both would corrupt the row, so the replacement character stands in for them:
        // the codepoint is covered, and saying so honestly beats printing something that
        // reorders the line.
        _ => Cell {
            glyph: '\u{FFFD}',
            width: 1,
        },
    }
}

/// One Unicode block and the codepoints a face covers within it.
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
pub struct BlockCoverage {
    pub block: String,
    pub start: u32,
    pub end: u32,
    /// Codepoints in the block that the face maps.
    pub codepoints: Vec<u32>,
    /// Size of the block, for a coverage ratio.
    pub block_size: u32,
}

/// Group a face's covered codepoints by Unicode block, in codepoint order.
pub fn glyph_map(ranges: &[[u32; 2]]) -> Vec<BlockCoverage> {
    let mut out: Vec<BlockCoverage> = Vec::new();
    for &[lo, hi] in ranges {
        for cp in lo..=hi {
            let Some(ch) = char::from_u32(cp) else {
                continue;
            };
            let block = unicode_blocks::find_unicode_block(ch);
            let (name, start, end) = match block {
                Some(b) => (b.name().to_string(), b.start(), b.end()),
                None => ("Unassigned".to_string(), cp, cp),
            };
            match out.last_mut() {
                Some(last) if last.block == name => {
                    last.codepoints.push(cp);
                    // "Unassigned" is synthesised from a single codepoint, so it has to
                    // grow to span the ones it goes on to absorb; leaving block_size at
                    // 1 puts its coverage ratio above 100%.
                    if block.is_none() {
                        last.end = last.end.max(cp);
                        last.block_size = last.end - last.start + 1;
                    }
                }
                _ => out.push(BlockCoverage {
                    block: name,
                    start,
                    end,
                    codepoints: vec![cp],
                    block_size: end - start + 1,
                }),
            }
        }
    }
    out
}

/// Format merged ranges as a CSS `unicode-range` value.
pub fn unicode_range_css(ranges: &[[u32; 2]]) -> String {
    ranges
        .iter()
        .map(|[a, b]| {
            if a == b {
                format!("U+{a:04X}")
            } else {
                format!("U+{a:04X}-{b:04X}")
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Map an OpenType `name` table (platform, language) pair to a BCP 47 tag.
pub fn bcp47_for_name_language(platform_id: u16, language_id: u16) -> Option<&'static str> {
    match platform_id {
        1 => Some(match language_id {
            0 => "en",
            1 => "fr",
            2 => "de",
            3 => "it",
            4 => "nl",
            5 => "sv",
            6 => "es",
            7 => "da",
            8 => "pt",
            9 => "nb",
            10 => "he",
            11 => "ja",
            12 => "ar",
            13 => "fi",
            14 => "el",
            15 => "is",
            17 => "tr",
            18 => "hr",
            19 => "zh-Hant",
            20 => "ur",
            21 => "hi",
            22 => "th",
            23 => "ko",
            24 => "lt",
            25 => "pl",
            26 => "hu",
            27 => "et",
            28 => "lv",
            30 => "fo",
            32 => "ru",
            33 => "zh-Hans",
            36 => "cs",
            37 => "sk",
            38 => "sl",
            39 => "ga",
            _ => return None,
        }),
        3 => Some(match language_id {
            0x0409 => "en",
            0x0809 => "en-GB",
            0x0C09 => "en-AU",
            0x1009 => "en-CA",
            0x0407 => "de",
            0x040C => "fr",
            0x0410 => "it",
            0x0C0A | 0x040A => "es",
            0x0411 => "ja",
            0x0412 => "ko",
            0x0804 => "zh-Hans",
            0x0404 => "zh-Hant",
            0x0C04 => "zh-HK",
            0x0419 => "ru",
            0x0416 => "pt-BR",
            0x0816 => "pt",
            0x0413 => "nl",
            0x041D => "sv",
            0x0406 => "da",
            0x0414 => "nb",
            0x040B => "fi",
            0x0415 => "pl",
            0x0405 => "cs",
            0x0408 => "el",
            0x041F => "tr",
            0x0401 => "ar",
            0x040D => "he",
            0x0439 => "hi",
            0x041E => "th",
            0x042A => "vi",
            0x040E => "hu",
            0x0418 => "ro",
            0x0422 => "uk",
            0x040F => "is",
            0x0421 => "id",
            0x0424 => "sl",
            0x041B => "sk",
            0x041A => "hr",
            0x0402 => "bg",
            0x0425 => "et",
            0x0426 => "lv",
            0x0427 => "lt",
            0x0403 => "ca",
            _ => return None,
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cell_never_lets_a_codepoint_break_the_row() {
        // Ordinary printable characters stand for themselves.
        assert_eq!(
            cell_for(0x41),
            Cell {
                glyph: 'A',
                width: 1
            }
        );
        assert_eq!(
            cell_for(0x0641),
            Cell {
                glyph: char::from_u32(0x0641).unwrap(),
                width: 1
            }
        );

        // East Asian Wide takes two cells.
        assert_eq!(cell_for(0x4E2D).width, 2, "CJK is double width");
        assert_eq!(cell_for(0xAC00).width, 2, "Hangul is double width");

        // The four kinds that would corrupt a grid all stand aside, and every one of
        // these is covered by a fixture in this repository.
        for cp in [
            0x0009, // Cc: tab, moves the cursor
            0x200B, // Cf: zero-width space
            0x200E, // Cf: left-to-right mark
            0x202E, // Cf: right-to-left override, reverses the rest of the line
            0x0651, // Mn: Arabic shadda, stacks onto the previous cell
        ] {
            let cell = cell_for(cp);
            assert_eq!(cell.glyph, '\u{FFFD}', "U+{cp:04X} must not be printed raw");
            assert_eq!(cell.width, 1);
        }
    }

    #[test]
    fn a_grid_of_cells_is_always_the_same_width() {
        // Whatever the codepoint, a cell padded to two columns occupies two columns.
        for cp in [0x41, 0x4E2D, 0x202E, 0x0651, 0x1F600] {
            let cell = cell_for(cp);
            assert!(cell.width == 1 || cell.width == 2);
            assert!(cell.width <= 2, "a cell never overflows its two columns");
        }
    }

    /// U+2FE0..U+2FEF belongs to no Unicode block, so `glyph_map` synthesises one.
    #[test]
    fn synthesised_block_spans_the_codepoints_it_absorbs() {
        let blocks = glyph_map(&[[0x2FE0, 0x2FE0], [0x2FEF, 0x2FEF]]);
        assert_eq!(blocks.len(), 1);
        let b = &blocks[0];
        assert_eq!(b.block, "Unassigned");
        assert_eq!(b.codepoints, vec![0x2FE0, 0x2FEF]);
        assert_eq!((b.start, b.end), (0x2FE0, 0x2FEF));
        // Coverage is a ratio of the two, and a ratio above 1 is nonsense.
        assert_eq!(b.block_size, 16);
        assert!(b.codepoints.len() as u32 <= b.block_size);
    }

    #[test]
    fn real_blocks_keep_their_declared_extent() {
        let blocks = glyph_map(&[[0x0041, 0x005A]]);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block, "Basic Latin");
        assert_eq!((blocks[0].start, blocks[0].end), (0, 0x7F));
        assert_eq!(blocks[0].block_size, 128);
    }
}
