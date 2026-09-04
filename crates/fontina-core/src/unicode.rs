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
            let (name, start, end) = match unicode_blocks::find_unicode_block(ch) {
                Some(b) => (b.name().to_string(), b.start(), b.end()),
                None => ("Unassigned".to_string(), cp, cp),
            };
            match out.last_mut() {
                Some(last) if last.block == name => last.codepoints.push(cp),
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
