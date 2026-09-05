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
    /// Detect the container from the file's first four bytes.
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

    /// Lowercase name, as used in the index and in `--json`.
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
/// The file a face was read from.
pub struct FileInfo {
    /// Absolute path as scanned.
    pub path: String,
    /// File size in bytes.
    pub size: u64,
    /// Modification time, seconds since the Unix epoch.
    pub mtime: i64,
    /// BLAKE3 hash of the file bytes, hex.
    pub blake3: String,
    /// Container format, from the magic bytes rather than the extension.
    pub container: Container,
    /// Number of faces in the file (1 unless a collection).
    pub face_count: u32,
}

/// One record from the `name` table.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NameRecord {
    /// `name` table name ID: 1 family, 2 subfamily, 4 full name, 6 PostScript, 16/17 typographic.
    pub name_id: u16,
    /// Platform ID: 0 Unicode, 1 Macintosh, 3 Windows.
    pub platform_id: u16,
    /// Encoding ID, interpreted against `platform_id`.
    pub encoding_id: u16,
    /// Language ID, interpreted against `platform_id`; `language` is its BCP 47 form.
    pub language_id: u16,
    /// BCP 47 tag when the platform language id is recognised.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// The decoded string.
    pub value: String,
}

/// Resolved names. `family`/`subfamily` prefer the typographic names (IDs 16/17) over
/// the legacy ones (IDs 1/2), which is what groups a family correctly.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct Names {
    pub family: String,
    pub subfamily: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Name ID 1, kept when ID 16 exists and differs from it.
    pub legacy_family: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Name ID 2, kept when ID 17 exists and differs from it.
    pub legacy_subfamily: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Name ID 4.
    pub full_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postscript_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Name ID 5, the version string as the font states it.
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Name ID 3.
    pub unique_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Name ID 9.
    pub designer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Name ID 12.
    pub designer_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Name ID 8.
    pub manufacturer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Name ID 11.
    pub vendor_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Name ID 0. Scanned for a licence and for OFL reserved font names.
    pub copyright: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Name ID 7.
    pub trademark: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Name ID 19. Preferred over the built-in pangram when previewing.
    pub sample_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Name ID 21. Weight/width/slope family, where the font declares one.
    pub wws_family: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Name ID 22.
    pub wws_subfamily: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "lowercase")]
/// How the face slopes, in the CSS Fonts Level 4 sense.
pub enum Slope {
    /// Upright.
    Normal,
    /// True italic letterforms: `OS/2.fsSelection` ITALIC or `head.macStyle`.
    Italic,
    Oblique {
        #[serde(skip_serializing_if = "Option::is_none")]
        /// Degrees, from the `slnt` axis or `post.italicAngle`, where the font gives one.
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
    /// The `format()` hint for `@font-face src`, e.g. `woff2`.
    pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
/// Where the face sits on the three CSS style axes.
pub struct Style {
    /// Weight on the CSS 1–1000 scale (from `OS/2.usWeightClass` or the `wght` default).
    pub weight: f32,
    /// Width as a percentage where 100 is normal (from `usWidthClass` or the `wdth` default).
    pub width: f32,
    /// Upright, italic or oblique.
    pub slope: Slope,
    /// The same face as CSS descriptors, carrying ranges where it is variable.
    pub css: CssDescriptor,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
/// Vertical metrics and the dates, from `head`, `hhea` and `OS/2`.
pub struct Metrics {
    /// `head.unitsPerEm`. Every other value here is in these units.
    pub units_per_em: u16,
    /// `hhea.ascender`.
    pub ascender: i16,
    /// `hhea.descender`, negative below the baseline.
    pub descender: i16,
    /// `hhea.lineGap`.
    pub line_gap: i16,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// `OS/2.sxHeight`, present from version 2.
    pub x_height: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// `OS/2.sCapHeight`, present from version 2.
    pub cap_height: Option<i16>,
    /// `post.italicAngle`, degrees counter-clockwise from vertical.
    pub italic_angle: f32,
    /// `post.isFixedPitch`. What the font claims; advance widths are not re-measured.
    pub is_fixed_pitch: bool,
    /// `head.fontRevision`.
    pub revision: f64,
    /// `head.created` as RFC 3339, when set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// `head.modified` as RFC 3339, when set.
    pub modified: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
/// The `OS/2.fsType` embedding level. Reported, never enforced; see `freedom`.
pub enum EmbeddingLevel {
    /// No embedding restriction stated.
    Installable,
    /// The font claims it may not be embedded without a separate licence.
    RestrictedLicense,
    /// The font claims embedding is for viewing and printing only.
    PreviewAndPrint,
    /// The font claims embedded documents may be edited.
    Editable,
}

/// Decoded `OS/2.fsType` embedding permissions.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EmbeddingRights {
    /// The least restrictive level the bits allow.
    pub level: EmbeddingLevel,
    /// `fsType` bit 8.
    pub no_subsetting: bool,
    /// `fsType` bit 9.
    pub bitmap_only: bool,
}

impl EmbeddingRights {
    /// Decode the raw `OS/2.fsType` value.
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
/// The `OS/2` table, kept whole because callers disagree about which fields matter.
pub struct Os2Info {
    /// `OS/2.version`; later versions carry more fields.
    pub version: u16,
    /// `OS/2.usWeightClass`, 1–1000.
    pub weight_class: u16,
    /// `OS/2.usWidthClass`, 1–9.
    pub width_class: u16,
    /// Raw `fsType` bits; `embedding` is the decoded form.
    pub fs_type: u16,
    /// `fs_type`, decoded.
    pub embedding: EmbeddingRights,
    /// `OS/2.achVendID`, four characters.
    pub vendor_id: String,
    /// `OS/2.fsSelection` bits.
    pub fs_selection: u16,
    /// `fsSelection` bit 7: prefer the typo metrics over the hhea ones.
    pub use_typo_metrics: bool,
    /// `OS/2.ulUnicodeRange1..4`. What the font claims; `coverage` is what it has.
    pub unicode_ranges: [u32; 4],
    #[serde(skip_serializing_if = "Option::is_none")]
    /// `OS/2.ulCodePageRange1..2`, present from version 1.
    pub codepage_ranges: Option<[u32; 2]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// `OS/2.sTypoAscender`.
    pub typo_ascender: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// `OS/2.sTypoDescender`.
    pub typo_descender: Option<i16>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
/// One variation axis from `fvar`.
pub struct AxisInfo {
    /// Four-character axis tag, e.g. `wght`.
    pub tag: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Minimum in user coordinates.
    pub min: f32,
    /// The value this face takes when no axis is set.
    pub default: f32,
    /// Maximum in user coordinates.
    pub max: f32,
    /// `fvar` flag: the axis is not meant to be shown in a user interface.
    pub hidden: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
/// A named instance from `fvar`.
pub struct InstanceInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Name ID 6. Unique per face, and what `dupes` matches on first.
    pub postscript_name: Option<String>,
    /// User-space coordinates, in axis order.
    pub coordinates: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
/// The variable-font tables, absent for a static face.
pub struct VariableInfo {
    /// Axes in `fvar` order; `coordinates` on an instance follows the same order.
    pub axes: Vec<AxisInfo>,
    /// Named instances, in `fvar` order.
    pub instances: Vec<InstanceInfo>,
    /// An `avar` table is present, so user and normalised coordinates differ.
    pub has_avar: bool,
    /// A `STAT` table is present.
    pub has_stat: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
/// One OpenType script and the language systems declared under it.
pub struct ScriptInfo {
    /// OpenType script tag, e.g. `latn`, `arab`, `DFLT`.
    pub tag: String,
    /// OpenType language system tags declared under the script.
    pub languages: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
/// What the shaping tables declare, as opposed to what the font covers.
pub struct Features {
    /// Distinct GSUB feature tags, sorted.
    pub gsub: Vec<String>,
    /// Distinct GPOS feature tags, sorted.
    pub gpos: Vec<String>,
    pub scripts: Vec<ScriptInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
/// How much of one Unicode script the face covers.
pub struct ScriptCoverage {
    /// ISO 15924 code from Unicode script property, e.g. `Latn`, `Arab`, `Zyyy` (Common).
    pub script: String,
    pub codepoints: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
/// What the face can actually set, derived from `cmap` rather than claimed.
pub struct Coverage {
    /// Total codepoints in `cmap`.
    pub codepoints: u32,
    /// Scripts sorted by codepoint count, descending.
    pub scripts: Vec<ScriptCoverage>,
    /// Inclusive codepoint ranges, merged, suitable for `unicode-range`.
    pub ranges: Vec<[u32; 2]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
/// Which outline table the glyphs live in.
pub enum OutlineFormat {
    /// TrueType outlines.
    Glyf,
    /// CFF outlines.
    Cff,
    /// CFF2 outlines, which are variable.
    Cff2,
    /// No outline table: a bitmap-only font.
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
/// A colour glyph format the face carries. A face may carry several.
pub enum ColorFormat {
    /// `COLR` version 0: layered solid colours.
    Colrv0,
    /// `COLR` version 1: gradients and compositing.
    Colrv1,
    /// An `SVG ` table.
    Svg,
    /// An `sbix` table of colour bitmaps.
    Sbix,
    /// `CBDT`/`CBLC` colour bitmaps.
    Cbdt,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
/// What the face can do, read from which tables are present.
pub struct Capabilities {
    /// Where the outlines are.
    pub outlines: OutlineFormat,
    /// Colour formats present, empty for a monochrome face.
    pub color: Vec<ColorFormat>,
    /// TrueType hinting programs present (`fpgm`/`prep`/`cvt `).
    pub hinting: bool,
    /// An `EBDT`/`EBLC` or `bloc`/`bdat` strike is present.
    pub bitmap_strikes: bool,
    /// A `MATH` table is present.
    pub math: bool,
    /// A legacy `kern` table is present, independent of GPOS kerning.
    pub kern_table: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
/// What the font says about its own licence.
pub struct LicenseInfo {
    /// SPDX identifier or expression when recognisable, e.g. `OFL-1.1`. `LicenseRef-Unknown`
    /// when license text exists but is not recognised. Absent when the font carries none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spdx: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Name ID 10.
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
    /// Bumped when this model changes in a way a reader cannot ignore.
    pub schema_version: u32,
    /// The file this face was read from.
    pub file: FileInfo,
    /// Index of the face within its file (0 unless a collection).
    pub index: u32,
    /// Resolved names, typographic where the font provides them.
    pub names: Names,
    /// The whole `name` table, so nothing is lost to resolution.
    pub name_records: Vec<NameRecord>,
    /// Weight, width and slope, on the CSS scales.
    pub style: Style,
    /// Vertical metrics and dates.
    pub metrics: Metrics,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// The `OS/2` table, absent only if the font has none.
    pub os2: Option<Os2Info>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Axes and named instances, absent for a static face.
    pub variable: Option<VariableInfo>,
    /// GSUB and GPOS feature tags and the scripts that declare them.
    pub features: Features,
    /// Codepoints, scripts and ranges, from `cmap`.
    pub coverage: Coverage,
    /// Outline format, colour, hinting, bitmaps, MATH, kern.
    pub capabilities: Capabilities,
    /// SPDX identifier where recognised, and whether it is free.
    pub license: LicenseInfo,
    /// `maxp.numGlyphs`.
    pub glyph_count: u16,
    /// BLAKE3 over the `name` table and outline tables: identifies the same face across
    /// containers (TTF vs WOFF2 of one font hash the same).
    pub identity_hash: String,
}

impl FaceMetadata {
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
