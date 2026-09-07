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

//! The metadata model. Every type here is `Serialize`, `Deserialize` and `JsonSchema`;
//! `FaceMetadata` is the document stored per face and returned by the CLI.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Font container format, detected from the file's magic bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Container {
    /// TrueType outlines in an sfnt wrapper (`00 01 00 00` or `true`).
    Ttf,
    /// CFF/CFF2 outlines in an sfnt wrapper (`OTTO`).
    Otf,
    /// TrueType/OpenType collection (`ttcf`).
    Ttc,
    /// WOFF 1.0.
    Woff,
    /// WOFF 2.0.
    Woff2,
}

impl Container {
    pub fn detect(bytes: &[u8]) -> Option<Container> {
        let magic = bytes.get(0..4)?;
        Some(match magic {
            [0x00, 0x01, 0x00, 0x00] | b"true" => Container::Ttf,
            b"OTTO" => Container::Otf,
            b"ttcf" => Container::Ttc,
            b"wOFF" => Container::Woff,
            b"wOF2" => Container::Woff2,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Container::Ttf => "ttf",
            Container::Otf => "otf",
            Container::Ttc => "ttc",
            Container::Woff => "woff",
            Container::Woff2 => "woff2",
        }
    }

    /// File extensions that may hold this container.
    pub fn extensions() -> &'static [&'static str] {
        &["ttf", "otf", "ttc", "otc", "woff", "woff2"]
    }

    /// The CSS `format()` hint for `@font-face src`.
    pub fn css_format(self) -> &'static str {
        match self {
            Container::Ttf => "truetype",
            Container::Otf => "opentype",
            Container::Ttc => "collection",
            Container::Woff => "woff",
            Container::Woff2 => "woff2",
        }
    }
}

impl std::str::FromStr for Container {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, ()> {
        Ok(match s {
            "ttf" => Container::Ttf,
            "otf" => Container::Otf,
            "ttc" => Container::Ttc,
            "woff" => Container::Woff,
            "woff2" => Container::Woff2,
            _ => return Err(()),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FileInfo {
    /// Absolute path as scanned.
    pub path: String,
    pub size: u64,
    /// Modification time, seconds since the Unix epoch.
    pub mtime: i64,
    /// BLAKE3 hash of the file bytes, hex.
    pub blake3: String,
    pub container: Container,
    /// Number of faces in the file (1 unless a collection).
    pub face_count: u32,
}

/// One record from the `name` table.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NameRecord {
    pub name_id: u16,
    pub platform_id: u16,
    pub encoding_id: u16,
    pub language_id: u16,
    /// BCP 47 tag when the platform language id is recognised.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    pub value: String,
}

/// Resolved names. `family`/`subfamily` prefer the typographic names (IDs 16/17) over
/// the legacy ones (IDs 1/2), which is what groups a family correctly.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct Names {
    pub family: String,
    pub subfamily: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub legacy_family: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub legacy_subfamily: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postscript_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unique_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub designer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub designer_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manufacturer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vendor_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub copyright: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trademark: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wws_family: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wws_subfamily: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Slope {
    Normal,
    Italic,
    Oblique {
        #[serde(skip_serializing_if = "Option::is_none")]
        angle: Option<f32>,
    },
}

/// The face expressed as CSS Fonts Level 4 descriptors.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CssDescriptor {
    pub family: String,
    /// `font-weight` value or range, e.g. `400` or `100 900`.
    pub weight: String,
    /// `font-stretch` value or range, e.g. `100%` or `75% 125%`.
    pub stretch: String,
    /// `font-style`, e.g. `normal`, `italic`, `oblique 12deg`, `oblique -10deg 0deg`.
    pub style: String,
    pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Style {
    /// Weight on the CSS 1–1000 scale (from `OS/2.usWeightClass` or the `wght` default).
    pub weight: f32,
    /// Width as a percentage where 100 is normal (from `usWidthClass` or the `wdth` default).
    pub width: f32,
    pub slope: Slope,
    pub css: CssDescriptor,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Metrics {
    pub units_per_em: u16,
    pub ascender: i16,
    pub descender: i16,
    pub line_gap: i16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x_height: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cap_height: Option<i16>,
    pub italic_angle: f32,
    /// What `post.isFixedPitch` claims.
    pub is_fixed_pitch: bool,
    /// How many distinct non-zero advance widths the face has, at its default instance.
    ///
    /// The fact `is_fixed_pitch` is a claim about. One means every glyph that occupies
    /// space occupies the same amount. `None` when there is no `hmtx` to measure.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distinct_advances: Option<u32>,
    /// `head.fontRevision`.
    pub revision: f64,
    /// `head.created` as RFC 3339, when set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EmbeddingLevel {
    Installable,
    RestrictedLicense,
    PreviewAndPrint,
    Editable,
}

/// Decoded `OS/2.fsType` embedding permissions.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EmbeddingRights {
    pub level: EmbeddingLevel,
    pub no_subsetting: bool,
    pub bitmap_only: bool,
}

impl EmbeddingRights {
    pub fn from_fs_type(fs_type: u16) -> Self {
        // Bits 0-3 are mutually exclusive in practice; the least restrictive wins per spec.
        let level = if fs_type & 0x000F == 0 {
            EmbeddingLevel::Installable
        } else if fs_type & 0x0008 != 0 {
            EmbeddingLevel::Editable
        } else if fs_type & 0x0004 != 0 {
            EmbeddingLevel::PreviewAndPrint
        } else {
            EmbeddingLevel::RestrictedLicense
        };
        EmbeddingRights {
            level,
            no_subsetting: fs_type & 0x0100 != 0,
            bitmap_only: fs_type & 0x0200 != 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Os2Info {
    pub version: u16,
    pub weight_class: u16,
    pub width_class: u16,
    pub fs_type: u16,
    pub embedding: EmbeddingRights,
    pub vendor_id: String,
    pub fs_selection: u16,
    pub use_typo_metrics: bool,
    pub unicode_ranges: [u32; 4],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codepage_ranges: Option<[u32; 2]>,
    /// `OS/2.sFamilyClass`: the IBM font class in the high byte, the subclass in the
    /// low one. Zero means the font declined to classify itself, which a great many do.
    #[serde(default)]
    pub family_class: i16,
    /// The ten PANOSE digits, as they are in the table.
    ///
    /// All zeroes is "any" and all ones is "no fit"; both mean the font is not saying.
    /// Kept whole rather than interpreted here, because the digits mean different
    /// things depending on the first of them, and a reader with the ten numbers can
    /// look up what this program does not.
    #[serde(default)]
    pub panose: [u8; 10],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub typo_ascender: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub typo_descender: Option<i16>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AxisInfo {
    /// Four-character axis tag, e.g. `wght`.
    pub tag: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub min: f32,
    pub default: f32,
    pub max: f32,
    pub hidden: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct InstanceInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postscript_name: Option<String>,
    /// User-space coordinates, in axis order.
    pub coordinates: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VariableInfo {
    pub axes: Vec<AxisInfo>,
    pub instances: Vec<InstanceInfo>,
    pub has_avar: bool,
    pub has_stat: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ScriptInfo {
    /// OpenType script tag, e.g. `latn`, `arab`, `DFLT`.
    pub tag: String,
    /// OpenType language system tags declared under the script.
    pub languages: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct Features {
    /// Distinct GSUB feature tags, sorted.
    pub gsub: Vec<String>,
    /// Distinct GPOS feature tags, sorted.
    pub gpos: Vec<String>,
    pub scripts: Vec<ScriptInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ScriptCoverage {
    /// ISO 15924 code from Unicode script property, e.g. `Latn`, `Arab`, `Zyyy` (Common).
    pub script: String,
    pub codepoints: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct Coverage {
    pub codepoints: u32,
    /// Scripts sorted by codepoint count, descending.
    pub scripts: Vec<ScriptCoverage>,
    /// Inclusive codepoint ranges, merged, suitable for `unicode-range`.
    pub ranges: Vec<[u32; 2]>,
}

/// What kind of typeface a font says it is.
///
/// Derived on every read from `OS/2.sFamilyClass` and PANOSE, never stored — the same
/// rule `freedom` follows, so the verdict tracks this code rather than the day the
/// index was built.
///
/// It is what the font *claims*, which is not always what it is: plenty of faces leave
/// both fields at zero, and a few fill them in wrongly. [`Classification::Unclassified`]
/// is therefore a real and common answer, and must never be reported as "sans" because
/// that is the commonest value among fonts that did answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Classification {
    Serif,
    SansSerif,
    Script,
    Decorative,
    Symbol,
    /// The font said nothing, or said only "any" or "no fit".
    #[default]
    Unclassified,
}

impl Classification {
    pub fn as_str(self) -> &'static str {
        match self {
            Classification::Serif => "serif",
            Classification::SansSerif => "sans-serif",
            Classification::Script => "script",
            Classification::Decorative => "decorative",
            Classification::Symbol => "symbol",
            Classification::Unclassified => "unclassified",
        }
    }

    /// Read the two fields the specification provides, in that order.
    ///
    /// `sFamilyClass` first, because it is the narrower claim: its high byte is the IBM
    /// font class, and a font that filled it in has answered the question directly.
    /// PANOSE second, because its first digit says which *kind* of thing the remaining
    /// nine digits describe, so it can only be read once that is known.
    ///
    /// The IBM classes, from the OS/2 specification: 1 to 7 are serif designs (oldstyle,
    /// transitional, modern, Clarendon, slab, a reserved slot, freeform), 8 is sans
    /// serif, 9 ornamentals, 10 scripts, 12 symbolic. 0, 11, 13 and 14 say nothing.
    ///
    /// In PANOSE, the first digit is the family kind: 2 is Latin text, 3 Latin hand
    /// written, 4 Latin decorative, 5 Latin symbol. For Latin text the second digit is
    /// the serif style, where 11 to 13 are the sans values (normal, obtuse and
    /// perpendicular sans) and 2 to 10 and 14 to 15 are serif ones. 0 and 1 are "any"
    /// and "no fit" everywhere, and both mean the font is not saying.
    pub fn of(family_class: i16, panose: &[u8; 10]) -> Classification {
        match (family_class >> 8) & 0xff {
            1..=5 | 7 => return Classification::Serif,
            8 => return Classification::SansSerif,
            9 => return Classification::Decorative,
            10 => return Classification::Script,
            12 => return Classification::Symbol,
            _ => {}
        }
        match panose[0] {
            2 => match panose[1] {
                11..=13 => Classification::SansSerif,
                2..=10 | 14..=15 => Classification::Serif,
                _ => Classification::Unclassified,
            },
            3 => Classification::Script,
            4 => Classification::Decorative,
            5 => Classification::Symbol,
            _ => Classification::Unclassified,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum OutlineFormat {
    Glyf,
    Cff,
    Cff2,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ColorFormat {
    Colrv0,
    Colrv1,
    Svg,
    Sbix,
    Cbdt,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Capabilities {
    pub outlines: OutlineFormat,
    pub color: Vec<ColorFormat>,
    /// TrueType hinting programs present (`fpgm`/`prep`/`cvt `).
    pub hinting: bool,
    pub bitmap_strikes: bool,
    pub math: bool,
    pub kern_table: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct LicenseInfo {
    /// SPDX identifier or expression when recognisable, e.g. `OFL-1.1`. `LicenseRef-Unknown`
    /// when license text exists but is not recognised. Absent when the font carries none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spdx: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// OFL Reserved Font Names declared in the copyright or license text.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reserved_font_names: Vec<String>,
    /// Whether `spdx` grants the four freedoms. Derived by `crate::freedom`; carried in
    /// the exported JSON so a consumer need not repeat the judgement.
    #[serde(default)]
    pub freedom: crate::freedom::Freedom,
}

/// Everything fontina knows about one face.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FaceMetadata {
    pub schema_version: u32,
    pub file: FileInfo,
    /// Index of the face within its file (0 unless a collection).
    pub index: u32,
    pub names: Names,
    pub name_records: Vec<NameRecord>,
    pub style: Style,
    pub metrics: Metrics,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os2: Option<Os2Info>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variable: Option<VariableInfo>,
    pub features: Features,
    pub coverage: Coverage,
    pub capabilities: Capabilities,
    pub license: LicenseInfo,
    pub glyph_count: u16,
    /// BLAKE3 over the `name` table and outline tables: identifies the same face across
    /// containers (TTF vs WOFF2 of one font hash the same).
    pub identity_hash: String,
}

impl FaceMetadata {
    /// What kind of typeface the font says it is.
    ///
    /// Derived on every read rather than stored, the same rule `freedom` follows: the
    /// verdict then tracks the mapping in [`Classification::of`] rather than the day
    /// the index was built. A font with no `OS/2` table at all is unclassified, which
    /// is the honest answer and not the same as "sans".
    pub fn classification(&self) -> Classification {
        match &self.os2 {
            Some(os2) => Classification::of(os2.family_class, &os2.panose),
            None => Classification::Unclassified,
        }
    }

    pub fn is_variable(&self) -> bool {
        self.variable.as_ref().is_some_and(|v| !v.axes.is_empty())
    }

    /// The CSS weights this face can be set to: its `wght` axis if it has one, and
    /// otherwise the single weight it is.
    ///
    /// `style.weight` is one number — the default instance — and filtering on it alone
    /// under-matches every variable font. Bricolage spans 200 to 800 and defaults to
    /// 800, so asking for 400 misses a font that does 400 perfectly well.
    pub fn weight_span(&self) -> (f32, f32) {
        self.axis_span("wght", self.style.weight)
    }

    /// The same, for width in percent, over the `wdth` axis.
    pub fn width_span(&self) -> (f32, f32) {
        self.axis_span("wdth", self.style.width)
    }

    /// The range of `tag`, widened to include `default` where the two disagree.
    ///
    /// A font whose OS/2 weight sits outside its own `wght` range is malformed, but it
    /// still reports that weight and `list` still prints it. Excluding it from a filter
    /// for the number it shows would be the wrong kind of correct.
    fn axis_span(&self, tag: &str, default: f32) -> (f32, f32) {
        let Some(axis) = self
            .variable
            .as_ref()
            .and_then(|v| v.axes.iter().find(|a| a.tag == tag))
        else {
            return (default, default);
        };
        (axis.min.min(default), axis.max.max(default))
    }
    pub fn is_color(&self) -> bool {
        !self.capabilities.color.is_empty()
    }
    pub fn is_italic(&self) -> bool {
        !matches!(self.style.slope, Slope::Normal)
    }
}
