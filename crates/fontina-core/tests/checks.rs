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

//! One case per health check id.
//!
//! Check ids are a published interface — the CLI prints them and users filter on them —
//! so `ALL_IDS` pins the whole set and `every_check_id_has_a_case` fails when a new id
//! appears in `check.rs` without a case here. Between them the two tests keep the
//! working agreement's rule ("every check needs a fixture-backed test that triggers
//! it") from rotting.
//!
//! The fixtures are healthy fonts, so almost nothing here can be triggered by loading
//! one unchanged. Rather than carry broken fonts in the repository, each case breaks a
//! healthy fixture on the fly, in one of two ways:
//!
//! * `sfnt`: byte surgery on the fixture's table directory — drop a table, or patch a
//!   field inside one — and then parse the result the way a scan would. fontations does
//!   not verify table checksums on load, so a patched directory is enough. These cases
//!   prove the whole path: bytes on disk, through the parser, into a finding.
//! * metadata: parse a fixture and edit the resulting `FaceMetadata`. Used where the
//!   condition is the *content* of a string or a set of mapped codepoints. Rewriting a
//!   `name` record or re-encoding a `cmap` subtable in a test would be more fragile
//!   than the thing it tests, and would exercise nothing the `sfnt` cases miss.
//!
//! Each case says what its mutation represents; together they document what each check
//! actually means.

use fontina_core::model::*;
use fontina_core::{Severity, check_face, load_file};
use std::collections::BTreeSet;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name)
}

/// Parse a fixture through the public API and take its first face.
fn face(name: &str) -> FaceMetadata {
    load_file(&fixture(name))
        .expect("fixture parses")
        .1
        .remove(0)
}

/// Minimal sfnt surgery, for tests only.
///
/// An sfnt file opens with a 12-byte offset table (the table count is a `u16` at byte 4)
/// followed by one 16-byte record per table: tag, checksum, offset, length. Table data
/// lives at absolute file offsets, so any edit that leaves those offsets alone is valid
/// as far as a reader is concerned. fontations does not verify checksums when it opens a
/// font, so neither the per-table checksum nor `head.checkSumAdjustment` needs
/// recomputing after a patch.
mod sfnt {
    use fontina_core::model::{Container, FaceMetadata, FileInfo};

    /// Size of the offset table; the records start here.
    const DIR: usize = 12;
    /// Size of one table record.
    const REC: usize = 16;

    pub struct Sfnt {
        bytes: Vec<u8>,
    }

    impl Sfnt {
        /// Load a fixture's raw bytes. Only uncompressed sfnt fixtures (`.ttf`, `.otf`)
        /// can be patched this way; a WOFF would have to be recompressed afterwards.
        pub fn load(name: &str) -> Sfnt {
            let bytes = std::fs::read(super::fixture(name)).expect("fixture is readable");
            assert!(
                matches!(
                    Container::detect(&bytes),
                    Some(Container::Ttf | Container::Otf)
                ),
                "{name} is not a bare sfnt"
            );
            Sfnt { bytes }
        }

        fn read_u16_at(&self, at: usize) -> u16 {
            u16::from_be_bytes([self.bytes[at], self.bytes[at + 1]])
        }

        fn write_u16_at(&mut self, at: usize, v: u16) {
            self.bytes[at..at + 2].copy_from_slice(&v.to_be_bytes());
        }

        fn num_tables(&self) -> usize {
            self.read_u16_at(4) as usize
        }

        fn record_index(&self, tag: &[u8; 4]) -> usize {
            (0..self.num_tables())
                .find(|i| &self.bytes[DIR + REC * i..DIR + REC * i + 4] == tag)
                .unwrap_or_else(|| panic!("fixture has no {} table", name_of(tag)))
        }

        /// File offset of a table's data.
        fn table_at(&self, tag: &[u8; 4]) -> usize {
            let off = DIR + REC * self.record_index(tag) + 8;
            u32::from_be_bytes(self.bytes[off..off + 4].try_into().unwrap()) as usize
        }

        /// Make a table invisible to the reader by deleting its directory record.
        ///
        /// The records after it shift 16 bytes towards the start of the directory and the
        /// 16 bytes that frees at the end of the directory are zeroed, so every table's
        /// data stays exactly where it was and the records stay sorted by tag. The
        /// dropped table's own bytes are left in the file, unreferenced.
        pub fn drop_table(&mut self, tag: &[u8; 4]) -> &mut Self {
            let i = self.record_index(tag);
            let n = self.num_tables();
            let end = DIR + REC * n;
            self.bytes
                .copy_within(DIR + REC * (i + 1)..end, DIR + REC * i);
            self.bytes[end - REC..end].fill(0);
            // searchRange/entrySelector/rangeShift are deliberately left stale: they are
            // a lookup hint, and read-fonts walks `numTables` records instead.
            self.write_u16_at(4, (n - 1) as u16);
            self
        }

        pub fn read_u16(&self, tag: &[u8; 4], offset: usize) -> u16 {
            self.read_u16_at(self.table_at(tag) + offset)
        }

        pub fn u16(&mut self, tag: &[u8; 4], offset: usize, v: u16) -> &mut Self {
            let at = self.table_at(tag) + offset;
            self.write_u16_at(at, v);
            self
        }

        pub fn i16(&mut self, tag: &[u8; 4], offset: usize, v: i16) -> &mut Self {
            self.u16(tag, offset, v as u16)
        }

        /// Write a 16.16 fixed-point number, the encoding `fvar` and `post` use.
        pub fn fixed(&mut self, tag: &[u8; 4], offset: usize, v: f32) -> &mut Self {
            let at = self.table_at(tag) + offset;
            self.bytes[at..at + 4].copy_from_slice(&((v * 65536.0) as i32).to_be_bytes());
            self
        }

        pub fn raw(&mut self, tag: &[u8; 4], offset: usize, v: &[u8]) -> &mut Self {
            let at = self.table_at(tag) + offset;
            self.bytes[at..at + v.len()].copy_from_slice(v);
            self
        }

        /// Zero a `LONGDATETIME` (8 bytes), which fontina reads back as "unset".
        pub fn zero_date(&mut self, tag: &[u8; 4], offset: usize) -> &mut Self {
            let at = self.table_at(tag) + offset;
            self.bytes[at..at + 8].fill(0);
            self
        }

        /// Offset of one `fvar` axis record, relative to the start of `fvar`.
        /// Header: version (4), axesArrayOffset (2), reserved (2), axisCount (2),
        /// axisSize (2), instanceCount (2), instanceSize (2).
        pub fn fvar_axis(&self, i: usize) -> usize {
            self.read_u16(b"fvar", 4) as usize + i * self.read_u16(b"fvar", 10) as usize
        }

        /// Offset of one `fvar` instance record, relative to the start of `fvar`. The
        /// instances follow the axes: subfamilyNameID (2), flags (2), then one 16.16
        /// coordinate per axis.
        pub fn fvar_instance(&self, i: usize) -> usize {
            let axis_count = self.read_u16(b"fvar", 8) as usize;
            self.fvar_axis(axis_count) + i * self.read_u16(b"fvar", 14) as usize
        }

        /// Parse the patched bytes as if they had just been scanned from `path`.
        pub fn parse(&self, path: &str) -> FaceMetadata {
            let container = Container::detect(&self.bytes).expect("still an sfnt");
            let file = FileInfo {
                path: path.to_string(),
                size: self.bytes.len() as u64,
                mtime: 0,
                blake3: String::new(),
                container,
                face_count: 1,
            };
            fontina_core::parse::parse_sfnt(&self.bytes, &file)
                .expect("patched font still parses")
                .remove(0)
        }
    }

    fn name_of(tag: &[u8; 4]) -> String {
        String::from_utf8_lossy(tag).into_owned()
    }
}

use sfnt::Sfnt;

const AMIRI: &str = "Amiri-Regular.ttf";
const BRICOLAGE: &str = "BricolageGrotesque[opsz,wdth,wght].ttf";
const INTER: &str = "inter-latin-400-normal.woff2";
const SOURCE_SERIF: &str = "SourceSerif4-Regular.otf";

/// A broken face and the finding it must produce.
struct Case {
    /// The healthy fixture the case starts from, when it breaks one. `None` means the
    /// fixture itself is the case and nothing is mutated.
    base: Option<&'static str>,
    id: &'static str,
    severity: Severity,
    /// What the mutation represents, in the terms the check is about.
    represents: &'static str,
    build: fn() -> FaceMetadata,
}

fn case(
    base: &'static str,
    id: &'static str,
    severity: Severity,
    represents: &'static str,
    build: fn() -> FaceMetadata,
) -> Case {
    Case {
        base: Some(base),
        id,
        severity,
        represents,
        build,
    }
}

/// A case that mutates nothing, for the one check a healthy font is supposed to raise.
fn healthy_case(
    id: &'static str,
    severity: Severity,
    represents: &'static str,
    build: fn() -> FaceMetadata,
) -> Case {
    Case {
        base: None,
        id,
        severity,
        represents,
        build,
    }
}

fn cases() -> Vec<Case> {
    vec![
        // ---- name -----------------------------------------------------------------
        // Dropping the whole `name` table is one mutation standing for "the font says
        // nothing about itself": no family, no PostScript name, no version, no designer,
        // no copyright, no license.
        case(
            AMIRI,
            "name/family",
            Severity::Error,
            "no name table, so name IDs 1 and 16 are both absent",
            || Sfnt::load(AMIRI).drop_table(b"name").parse(AMIRI),
        ),
        case(
            AMIRI,
            "name/postscript",
            Severity::Error,
            "no name table, so name ID 6 is absent",
            || Sfnt::load(AMIRI).drop_table(b"name").parse(AMIRI),
        ),
        case(
            AMIRI,
            "name/postscript",
            Severity::Error,
            "a PostScript name with spaces in it, over the 63-byte limit",
            || {
                let mut f = face(AMIRI);
                f.names.postscript_name = Some(
                    "Has Spaces And Is Far Too Long For A PostScript Name To Be Accepted By Anything"
                        .into(),
                );
                f
            },
        ),
        case(
            AMIRI,
            "name/version",
            Severity::Warn,
            "no name table, so no version string (name ID 5)",
            || Sfnt::load(AMIRI).drop_table(b"name").parse(AMIRI),
        ),
        case(
            AMIRI,
            "name/version",
            Severity::Warn,
            "a version string that disagrees with head.fontRevision",
            || {
                let mut f = face(AMIRI);
                f.names.version = Some("Version 99.000".into());
                f
            },
        ),
        case(
            AMIRI,
            "name/full-name",
            Severity::Info,
            "a full name (ID 4) unrelated to the family name",
            || {
                let mut f = face(AMIRI);
                f.names.full_name = Some("Zzz Sample Face".into());
                f
            },
        ),
        case(
            AMIRI,
            "name/designer",
            Severity::Info,
            "no name table, so neither designer (ID 9) nor manufacturer (ID 8)",
            || Sfnt::load(AMIRI).drop_table(b"name").parse(AMIRI),
        ),
        // ---- OS/2 -----------------------------------------------------------------
        // Field offsets within OS/2: usWeightClass 4, usWidthClass 6, fsType 8,
        // achVendID 58, fsSelection 62, sTypoAscender 68.
        case(
            AMIRI,
            "os2/missing",
            Severity::Error,
            "no OS/2 table",
            || Sfnt::load(AMIRI).drop_table(b"OS/2").parse(AMIRI),
        ),
        case(
            AMIRI,
            "os2/weight-class",
            Severity::Error,
            "usWeightClass 1200, past the top of the 1..1000 range",
            || Sfnt::load(AMIRI).u16(b"OS/2", 4, 1200).parse(AMIRI),
        ),
        case(
            AMIRI,
            "os2/weight-class",
            Severity::Info,
            "usWeightClass 375 on a static font: in range, but off the 50-step grid",
            || Sfnt::load(AMIRI).u16(b"OS/2", 4, 375).parse(AMIRI),
        ),
        case(
            AMIRI,
            "os2/width-class",
            Severity::Error,
            "usWidthClass 12, past the top of the 1..9 range",
            || Sfnt::load(AMIRI).u16(b"OS/2", 6, 12).parse(AMIRI),
        ),
        case(
            AMIRI,
            "os2/vendor-id",
            Severity::Info,
            "achVendID left at the placeholder UKWN",
            || Sfnt::load(AMIRI).raw(b"OS/2", 58, b"UKWN").parse(AMIRI),
        ),
        case(
            AMIRI,
            "os2/fs-selection",
            Severity::Warn,
            "fsSelection claiming REGULAR and BOLD at once",
            || Sfnt::load(AMIRI).u16(b"OS/2", 62, 0x0060).parse(AMIRI),
        ),
        case(
            AMIRI,
            "os2/italic-angle",
            Severity::Warn,
            "fsSelection ITALIC set on a face whose post.italicAngle is 0",
            || Sfnt::load(AMIRI).u16(b"OS/2", 62, 0x0001).parse(AMIRI),
        ),
        case(
            AMIRI,
            "os2/italic-angle",
            Severity::Warn,
            "a slanted post.italicAngle with the fsSelection ITALIC bit clear",
            || Sfnt::load(AMIRI).fixed(b"post", 4, -12.0).parse(AMIRI),
        ),
        case(
            AMIRI,
            "os2/bold-weight",
            Severity::Warn,
            "fsSelection BOLD set on a face that weighs 400",
            || Sfnt::load(AMIRI).u16(b"OS/2", 62, 0x0020).parse(AMIRI),
        ),
        case(
            AMIRI,
            "os2/fs-type",
            Severity::Info,
            "fsType bit 1: the font file forbids embedding",
            || Sfnt::load(AMIRI).u16(b"OS/2", 8, 0x0002).parse(AMIRI),
        ),
        case(
            AMIRI,
            "os2/fs-type",
            Severity::Info,
            "fsType bit 9: embedding restricted to bitmaps",
            || Sfnt::load(AMIRI).u16(b"OS/2", 8, 0x0200).parse(AMIRI),
        ),
        case(
            AMIRI,
            "license/embedding",
            Severity::Info,
            "fsType bit 2 (preview & print): reported, never enforced",
            || Sfnt::load(AMIRI).u16(b"OS/2", 8, 0x0004).parse(AMIRI),
        ),
        case(
            AMIRI,
            "metrics/typo-vs-hhea",
            Severity::Warn,
            "OS/2 typo ascender disagreeing with hhea while USE_TYPO_METRICS is clear",
            || {
                Sfnt::load(AMIRI)
                    .u16(b"OS/2", 62, 0x0040) // REGULAR only, so USE_TYPO_METRICS is off
                    .i16(b"OS/2", 68, 1) // sTypoAscender nowhere near hhea.ascender
                    .parse(AMIRI)
            },
        ),
        // ---- head / hhea ----------------------------------------------------------
        // head: unitsPerEm 18, created 20. hhea: ascender 4, descender 6.
        case(
            AMIRI,
            "head/units-per-em",
            Severity::Error,
            "unitsPerEm 8, below the 16..16384 the spec allows",
            || Sfnt::load(AMIRI).u16(b"head", 18, 8).parse(AMIRI),
        ),
        case(
            AMIRI,
            "head/created",
            Severity::Info,
            "head.created zeroed: the font has no creation date",
            || Sfnt::load(AMIRI).zero_date(b"head", 20).parse(AMIRI),
        ),
        case(
            AMIRI,
            "hhea/ascender",
            Severity::Error,
            "hhea.ascender 0, which collapses the line box",
            || Sfnt::load(AMIRI).i16(b"hhea", 4, 0).parse(AMIRI),
        ),
        case(
            AMIRI,
            "hhea/descender",
            Severity::Warn,
            "a positive hhea.descender, against the sign convention",
            || Sfnt::load(AMIRI).i16(b"hhea", 6, 100).parse(AMIRI),
        ),
        // ---- glyphs, cmap, outlines -----------------------------------------------
        case(
            AMIRI,
            "glyf/empty",
            Severity::Error,
            "no maxp table, so the face reports zero glyphs",
            || Sfnt::load(AMIRI).drop_table(b"maxp").parse(AMIRI),
        ),
        case(
            AMIRI,
            "cmap/empty",
            Severity::Error,
            "no cmap table, so no codepoint reaches a glyph",
            || Sfnt::load(AMIRI).drop_table(b"cmap").parse(AMIRI),
        ),
        case(
            INTER,
            "cmap/space",
            Severity::Warn,
            "coverage that skips U+0020",
            || {
                let mut f = face(INTER);
                f.coverage.ranges = vec![[0x41, 0x5A]];
                f
            },
        ),
        case(
            INTER,
            "cmap/nbsp",
            Severity::Info,
            "printable ASCII only: U+0020 is mapped, U+00A0 is not",
            || {
                let mut f = face(INTER);
                f.coverage.ranges = vec![[0x20, 0x7E]];
                f
            },
        ),
        case(
            INTER,
            "cmap/basic-latin",
            Severity::Warn,
            "a Latin-first face that maps only part of A–Z",
            || {
                let mut f = face(INTER);
                assert_eq!(f.coverage.scripts[0].script, "Latn");
                f.coverage.ranges = vec![[0x20, 0x40], [0x61, 0x7A], [0xA0, 0xA0]];
                f
            },
        ),
        case(
            AMIRI,
            "outlines/none",
            Severity::Error,
            "no glyf, no CFF and no bitmap strikes: nothing to draw",
            || Sfnt::load(AMIRI).drop_table(b"glyf").parse(AMIRI),
        ),
        case(
            AMIRI,
            "hinting/none",
            Severity::Info,
            "TrueType outlines with the hinting programs removed",
            || Sfnt::load(AMIRI).drop_table(b"prep").parse(AMIRI),
        ),
        // ---- variations -----------------------------------------------------------
        // Bricolage's axes are opsz 12..96, wght 200..800, wdth 75..100, all with the
        // default at the maximum, and it carries seven named instances.
        case(
            BRICOLAGE,
            "fvar/stat",
            Severity::Warn,
            "a variable font with no STAT table to link its styles",
            || Sfnt::load(BRICOLAGE).drop_table(b"STAT").parse(BRICOLAGE),
        ),
        case(
            BRICOLAGE,
            "fvar/instances",
            Severity::Warn,
            "instanceCount zeroed: a variable font with no named instances",
            || Sfnt::load(BRICOLAGE).u16(b"fvar", 12, 0).parse(BRICOLAGE),
        ),
        case(
            BRICOLAGE,
            "fvar/axis-range",
            Severity::Error,
            "an axis whose default sits outside its own min..max",
            || {
                let mut s = Sfnt::load(BRICOLAGE);
                let axis = s.fvar_axis(0);
                s.fixed(b"fvar", axis + 8, 1000.0).parse(BRICOLAGE)
            },
        ),
        case(
            BRICOLAGE,
            "fvar/axis-range",
            Severity::Warn,
            "an axis pinned to a single value (min raised to equal max)",
            || {
                let mut s = Sfnt::load(BRICOLAGE);
                let axis = s.fvar_axis(0);
                s.fixed(b"fvar", axis + 4, 96.0).parse(BRICOLAGE)
            },
        ),
        case(
            BRICOLAGE,
            "fvar/axis-tag",
            Severity::Warn,
            "a custom axis on a lowercase tag, which is reserved for registered axes",
            || {
                let mut s = Sfnt::load(BRICOLAGE);
                let axis = s.fvar_axis(0);
                s.raw(b"fvar", axis, b"abcd").parse(BRICOLAGE)
            },
        ),
        case(
            BRICOLAGE,
            "fvar/wght-os2",
            Severity::Warn,
            "usWeightClass disagreeing with the wght axis default",
            || Sfnt::load(BRICOLAGE).u16(b"OS/2", 4, 400).parse(BRICOLAGE),
        ),
        case(
            BRICOLAGE,
            "fvar/instance-name",
            Severity::Warn,
            "a named instance pointing at a name ID the font does not have",
            || {
                let mut s = Sfnt::load(BRICOLAGE);
                let inst = s.fvar_instance(0);
                s.u16(b"fvar", inst, 0xFFFE).parse(BRICOLAGE)
            },
        ),
        case(
            BRICOLAGE,
            "fvar/instance-range",
            Severity::Error,
            "a named instance placed outside the design space",
            || {
                let mut s = Sfnt::load(BRICOLAGE);
                let inst = s.fvar_instance(0);
                s.fixed(b"fvar", inst + 4, 1000.0).parse(BRICOLAGE)
            },
        ),
        // ---- layout ---------------------------------------------------------------
        case(
            AMIRI,
            "layout/shaping",
            Severity::Warn,
            "Arabic coverage with no GSUB or GPOS at all: the text cannot join",
            || {
                Sfnt::load(AMIRI)
                    .drop_table(b"GSUB")
                    .drop_table(b"GPOS")
                    .parse(AMIRI)
            },
        ),
        case(
            AMIRI,
            "layout/kerning",
            Severity::Info,
            "a large face with neither GPOS nor a legacy kern table",
            || Sfnt::load(AMIRI).drop_table(b"GPOS").parse(AMIRI),
        ),
        // ---- license --------------------------------------------------------------
        case(
            AMIRI,
            "license/missing",
            Severity::Warn,
            "no name table, so no license text (ID 13) or URL (ID 14)",
            || Sfnt::load(AMIRI).drop_table(b"name").parse(AMIRI),
        ),
        case(
            AMIRI,
            "license/copyright",
            Severity::Warn,
            "no name table, so no copyright notice (ID 0)",
            || Sfnt::load(AMIRI).drop_table(b"name").parse(AMIRI),
        ),
        case(
            AMIRI,
            "license/unknown",
            Severity::Warn,
            "license text present but matching no license fontina knows",
            || {
                let mut f = face(AMIRI);
                f.license.spdx = Some("LicenseRef-Unknown".into());
                f
            },
        ),
        case(
            AMIRI,
            "license/url",
            Severity::Info,
            "an OFL font that does not say where the license lives",
            || {
                let mut f = face(AMIRI);
                f.license.url = None;
                f
            },
        ),
        case(
            AMIRI,
            "license/rfn",
            Severity::Info,
            "a Reserved Font Name that is not part of the family name",
            || {
                let mut f = face(AMIRI);
                f.license.reserved_font_names = vec!["Nonesuch".into()];
                f
            },
        ),
        // No nonfree font can be a fixture — there would be no right to redistribute
        // one — so a free fixture is relabelled instead.
        case(
            AMIRI,
            "license/nonfree",
            Severity::Warn,
            "a license identifier that withholds the four freedoms",
            || {
                let mut f = face(AMIRI);
                f.license.spdx = Some("LicenseRef-Proprietary".into());
                f
            },
        ),
        case(
            AMIRI,
            "name/empty",
            Severity::Warn,
            "a name record that exists but carries no text, which shows as a blank entry in every font menu",
            || {
                let mut f = face(AMIRI);
                f.name_records[0].value = String::new();
                f
            },
        ),
        case(
            AMIRI,
            "name/whitespace",
            Severity::Warn,
            "a name record padded with space: invisible in the tools that show it, and it sorts wrongly in the ones that do not",
            || {
                let mut f = face(AMIRI);
                f.name_records[0].value = format!(" {} ", f.name_records[0].value);
                f
            },
        ),
        case(
            AMIRI,
            "metrics/line-gap",
            Severity::Info,
            "a non-zero hhea.lineGap, the usual reason a paragraph sets taller on one platform than another",
            || {
                let mut f = face(AMIRI);
                f.metrics.line_gap = 200;
                f
            },
        ),
        case(
            AMIRI,
            "metrics/x-height",
            Severity::Warn,
            "an xHeight above the capHeight, which no Latin design has",
            || {
                let mut f = face(AMIRI);
                f.metrics.cap_height = Some(700);
                f.metrics.x_height = Some(900);
                // A stated cap height, not the Some(0) that means "unset".
                assert!(f.metrics.cap_height.is_some_and(|c| c > 0));
                f
            },
        ),
        case(
            AMIRI,
            "cmap/private-use",
            Severity::Info,
            "coverage inside a Private Use Area, where the codepoints mean nothing without this font",
            || {
                let mut f = face(AMIRI);
                // Comfortably over the threshold that separates an icon set from a
                // font that merely carries a logo.
                f.coverage.ranges.push([0xE000, 0xE0FF]);
                f
            },
        ),
        healthy_case(
            "license/free",
            Severity::Info,
            "an unmodified OFL fixture: the one check a healthy font should raise",
            || face(AMIRI),
        ),
        // ---- file -----------------------------------------------------------------
        case(
            AMIRI,
            "file/extension",
            Severity::Warn,
            "TrueType outlines in a file named .otf",
            || Sfnt::load(AMIRI).parse("Amiri-Regular.otf"),
        ),
    ]
}

#[test]
fn every_case_triggers_its_check() {
    for c in cases() {
        let f = (c.build)();
        let report = check_face(&f);
        let hit = report
            .findings
            .iter()
            .find(|x| x.id == c.id && x.severity == c.severity);
        assert!(
            hit.is_some(),
            "{}: expected {} at {:?}, got {:?}",
            c.represents,
            c.id,
            c.severity,
            report.findings
        );
    }
}

/// A face with no cmap is still checked for everything a cmap has nothing to do with.
///
/// `coverage()` used to return the moment it raised `cmap/empty`, so a font with no
/// character map was never asked whether it had outlines at all: the single most broken
/// kind of file got the shortest report.
#[test]
fn an_empty_cmap_does_not_hide_the_rest_of_the_report() {
    let broken = Sfnt::load(AMIRI)
        .drop_table(b"cmap")
        .drop_table(b"glyf")
        .parse(AMIRI);
    let report = check_face(&broken);
    let ids: Vec<&str> = report.findings.iter().map(|f| f.id).collect();
    assert!(ids.contains(&"cmap/empty"), "{ids:?}");
    assert!(
        ids.contains(&"outlines/none"),
        "outlines/none must still fire when the cmap is gone: {ids:?}"
    );
}

/// Every check id fontina can emit, sorted. Ids are part of the published interface:
/// never rename one, only add. Adding a check means adding its id here and adding a
/// case above.
const ALL_IDS: &[&str] = &[
    "cmap/basic-latin",
    "cmap/empty",
    "cmap/nbsp",
    "cmap/private-use",
    "cmap/space",
    "file/extension",
    "fvar/axis-range",
    "fvar/axis-tag",
    "fvar/instance-name",
    "fvar/instance-range",
    "fvar/instances",
    "fvar/stat",
    "fvar/wght-os2",
    "glyf/empty",
    "head/created",
    "head/units-per-em",
    "hhea/ascender",
    "hhea/descender",
    "hinting/none",
    "layout/kerning",
    "layout/shaping",
    "license/copyright",
    "license/embedding",
    "license/free",
    "license/missing",
    "license/nonfree",
    "license/rfn",
    "license/unknown",
    "license/url",
    "metrics/line-gap",
    "metrics/typo-vs-hhea",
    "metrics/x-height",
    "name/designer",
    "name/empty",
    "name/family",
    "name/full-name",
    "name/postscript",
    "name/version",
    "name/whitespace",
    "os2/bold-weight",
    "os2/fs-selection",
    "os2/fs-type",
    "os2/italic-angle",
    "os2/missing",
    "os2/vendor-id",
    "os2/weight-class",
    "os2/width-class",
    "outlines/none",
];

/// Ids no case above can reach, each with the reason. Currently empty, and meant to
/// stay that way: an entry here is a claim that the check cannot be triggered without a
/// font the repository should not carry, not a place to park an id that is merely
/// awkward. `every_check_id_has_a_case` rejects an entry that some case does in fact
/// cover, so the list cannot go stale in the quiet direction either.
const UNTRIGGERABLE: &[(&str, &str)] = &[];

const CHECK_RS: &str = include_str!("../src/check.rs");

/// The published table of check ids on the web site. It states that every check has an
/// identifier and then lists them, which is a promise that goes stale silently: `ALL_IDS`
/// is guarded by a test and this table was not, so it had drifted by eleven ids.
const CHECKS_DOC: &str = include_str!("../../../site/src/content/docs/checks.md");

/// The ids `check.rs` really emits, read out of the source. Every finding is raised
/// through `Ctx::error`, `Ctx::warn` or `Ctx::info` with the id as a literal first
/// argument, so the set can be recovered without running anything. A future check that
/// computes its id would break this and should be caught here rather than silently
/// escape coverage.
/// Ids the web site documents, read out of its table.
fn ids_in_doc() -> BTreeSet<&'static str> {
    CHECKS_DOC
        .lines()
        .filter_map(|l| l.strip_prefix("| `"))
        .filter_map(|l| l.split_once('`'))
        .map(|(id, _)| id)
        .filter(|id| id.contains('/'))
        .collect()
}

/// The documented table and the code agree, in both directions.
///
/// Documentation that lists identifiers is a promise, and this one had quietly fallen
/// eleven ids behind while claiming to be complete. A reader filtering on an id they
/// read there deserves it to exist.
#[test]
fn the_documented_check_ids_are_the_real_ones() {
    let code = ids_in_source();
    let doc = ids_in_doc();
    let missing: Vec<_> = code.difference(&doc).collect();
    let stale: Vec<_> = doc.difference(&code).collect();
    assert!(
        missing.is_empty() && stale.is_empty(),
        "site/src/content/docs/checks.md is out of step with check.rs\n  \
         undocumented: {missing:?}\n  documented but gone: {stale:?}"
    );
}

fn ids_in_source() -> BTreeSet<&'static str> {
    let mut ids = BTreeSet::new();
    for marker in [".error(", ".warn(", ".info("] {
        let mut rest = CHECK_RS;
        while let Some(i) = rest.find(marker) {
            let after = &rest[i + marker.len()..];
            let lit = after.trim_start();
            let lit = lit.strip_prefix('"').unwrap_or_else(|| {
                panic!("a `{marker}` call in check.rs does not pass a literal id: {lit:.60}")
            });
            let end = lit.find('"').expect("unterminated id literal in check.rs");
            ids.insert(&lit[..end]);
            rest = after;
        }
    }
    ids
}

#[test]
fn all_ids_is_the_complete_set_of_check_ids() {
    let pinned: BTreeSet<&str> = ALL_IDS.iter().copied().collect();
    assert_eq!(pinned.len(), ALL_IDS.len(), "ALL_IDS has a duplicate");
    let mut sorted = ALL_IDS.to_vec();
    sorted.sort_unstable();
    assert_eq!(sorted, ALL_IDS, "ALL_IDS is not sorted");
    for id in ALL_IDS {
        assert!(id.contains('/'), "{id} is not an area/check id");
    }
    assert_eq!(
        ids_in_source(),
        pinned,
        "check.rs and ALL_IDS disagree: a check id was added, removed or renamed"
    );
}

#[test]
fn every_check_id_has_a_case() {
    let covered: BTreeSet<&str> = cases().iter().map(|c| c.id).collect();
    let allowed: BTreeSet<&str> = UNTRIGGERABLE.iter().map(|(id, _)| *id).collect();

    for (id, why) in UNTRIGGERABLE {
        assert!(
            ALL_IDS.contains(id),
            "{id} is listed as untriggerable but is not a check id"
        );
        assert!(
            !covered.contains(id),
            "{id} is listed as untriggerable ({why}) but a case covers it; drop the entry"
        );
    }
    for id in &covered {
        assert!(ALL_IDS.contains(id), "there is a case for unknown id {id}");
    }

    let missing: Vec<&&str> = ALL_IDS
        .iter()
        .filter(|id| !covered.contains(*id) && !allowed.contains(*id))
        .collect();
    assert!(
        missing.is_empty(),
        "no test triggers these checks: {missing:?}. Add a case, or an UNTRIGGERABLE \
         entry saying why the check cannot be triggered."
    );
}

/// A mutation that fires a check is only evidence if the unmutated font does not fire
/// it. `health_checks_pass_on_well_formed_fixtures` in `fixtures.rs` holds three
/// fixtures to zero findings of any severity; this holds the rest to no errors.
/// A case proves something only if the fixture it starts from does not already raise
/// the finding. Not every fixture is pristine — the Inter subsets are shipped without
/// designer names or hinting programs, so they raise `name/designer` and `hinting/none`
/// on their own — which is exactly why this is worth asserting rather than assuming.
#[test]
fn each_case_is_caused_by_its_mutation() {
    for c in cases() {
        let Some(base) = c.base else { continue };
        let before = check_face(&face(base));
        assert!(
            !before
                .findings
                .iter()
                .any(|x| x.id == c.id && x.severity == c.severity),
            "{base} raises {} at {:?} unmutated, so the case \"{}\" proves nothing",
            c.id,
            c.severity,
            c.represents
        );
    }
}

#[test]
fn unmutated_fixtures_raise_no_errors() {
    for name in [
        AMIRI,
        BRICOLAGE,
        INTER,
        SOURCE_SERIF,
        "Nabla[EDPT,EHLT].ttf",
    ] {
        let report = check_face(&face(name));
        assert_eq!(report.errors, 0, "{name}: {:?}", report.findings);
    }
}
