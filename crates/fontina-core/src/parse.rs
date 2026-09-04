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

//! Extract `FaceMetadata` from sfnt bytes using fontations (`read-fonts` + `skrifa`).

use crate::error::{Error, Result};
use crate::license::spdx_from_names;
use crate::model::*;
use crate::unicode::{bcp47_for_name_language, coverage_from_codepoints, unicode_range_css};
use read_fonts::types::Tag;
use read_fonts::{FileRef, FontRef, TableProvider};
use skrifa::MetadataProvider;
use skrifa::string::StringId;
use std::collections::{BTreeMap, BTreeSet};

/// Parse every face in an sfnt (single font or collection).
pub fn parse_sfnt(sfnt: &[u8], file: &FileInfo) -> Result<Vec<FaceMetadata>> {
    match FileRef::new(sfnt)? {
        FileRef::Font(font) => Ok(vec![parse_face(&font, 0, file)?]),
        FileRef::Collection(coll) => {
            let mut faces = Vec::with_capacity(coll.len() as usize);
            for i in 0..coll.len() {
                let font = coll.get(i)?;
                faces.push(parse_face(&font, i, file)?);
            }
            Ok(faces)
        }
    }
}

fn english(font: &FontRef, id: StringId) -> Option<String> {
    font.localized_strings(id)
        .english_or_first()
        .map(|s| s.to_string())
        .filter(|s| !s.trim().is_empty())
}

fn name_records(font: &FontRef) -> Vec<NameRecord> {
    let Ok(name) = font.name() else {
        return Vec::new();
    };
    let data = name.string_data();
    name.name_record()
        .iter()
        .filter_map(|r| {
            let value = r.string(data).ok()?.to_string();
            Some(NameRecord {
                name_id: r.name_id().to_u16(),
                platform_id: r.platform_id(),
                encoding_id: r.encoding_id(),
                language_id: r.language_id(),
                language: bcp47_for_name_language(r.platform_id(), r.language_id())
                    .map(String::from),
                value,
            })
        })
        .collect()
}

fn names(font: &FontRef) -> Names {
    let legacy_family = english(font, StringId::FAMILY_NAME);
    let legacy_subfamily = english(font, StringId::SUBFAMILY_NAME);
    let typo_family = english(font, StringId::TYPOGRAPHIC_FAMILY_NAME);
    let typo_subfamily = english(font, StringId::TYPOGRAPHIC_SUBFAMILY_NAME);
    Names {
        family: typo_family
            .clone()
            .or_else(|| legacy_family.clone())
            .unwrap_or_default(),
        subfamily: typo_subfamily
            .clone()
            .or_else(|| legacy_subfamily.clone())
            .unwrap_or_else(|| "Regular".into()),
        legacy_family,
        legacy_subfamily,
        full_name: english(font, StringId::FULL_NAME),
        postscript_name: english(font, StringId::POSTSCRIPT_NAME),
        version: english(font, StringId::VERSION_STRING),
        unique_id: english(font, StringId::UNIQUE_ID),
        designer: english(font, StringId::DESIGNER),
        designer_url: english(font, StringId::DESIGNER_URL),
        manufacturer: english(font, StringId::MANUFACTURER),
        vendor_url: english(font, StringId::VENDOR_URL),
        copyright: english(font, StringId::COPYRIGHT_NOTICE),
        trademark: english(font, StringId::TRADEMARK),
        description: english(font, StringId::DESCRIPTION),
        sample_text: english(font, StringId::SAMPLE_TEXT),
        wws_family: english(font, StringId::WWS_FAMILY_NAME),
        wws_subfamily: english(font, StringId::WWS_SUBFAMILY_NAME),
    }
}

fn variable(font: &FontRef) -> Option<VariableInfo> {
    let axes = font.axes();
    if axes.is_empty() {
        return None;
    }
    let axis_infos: Vec<AxisInfo> = axes
        .iter()
        .map(|a| AxisInfo {
            tag: a.tag().to_string(),
            name: english(font, a.name_id()),
            min: a.min_value(),
            default: a.default_value(),
            max: a.max_value(),
            hidden: a.is_hidden(),
        })
        .collect();
    let instances = font
        .named_instances()
        .iter()
        .map(|inst| InstanceInfo {
            name: english(font, inst.subfamily_name_id()),
            postscript_name: inst.postscript_name_id().and_then(|id| english(font, id)),
            coordinates: inst.user_coords().collect(),
        })
        .collect();
    Some(VariableInfo {
        axes: axis_infos,
        instances,
        has_avar: font.table_data(Tag::new(b"avar")).is_some(),
        has_stat: font.table_data(Tag::new(b"STAT")).is_some(),
    })
}

/// Caps on what a layout table may declare, so that import time stays bounded by
/// something other than an attacker's imagination. OpenType registers a few hundred
/// scripts and a few hundred languages; these are an order of magnitude beyond that.
const MAX_SCRIPT_RECORDS: usize = 1024;
const MAX_LANG_SYS: usize = 1024;

fn features(font: &FontRef) -> Features {
    let mut gsub = BTreeSet::new();
    let mut gpos = BTreeSet::new();
    // A map, not a list scanned linearly. The old shape looked for the script with
    // `iter_mut().find` and for the language with `contains`, so a file declaring twelve
    // thousand script records cost minutes: a 241 KB font took 37 seconds to import. Both
    // counts come straight off the wire as `u16`, so they are the attacker's to choose.
    let mut scripts: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    let mut add_scripts = |list: read_fonts::tables::layout::ScriptList| {
        let data = list.offset_data();
        // Records can all point at the same language list, so the work is not bounded by
        // the file's size. These caps are: no real font comes close, the registry knows
        // about a couple of hundred scripts, and a truncated tail of a pathological font
        // is a better answer than a scan that never ends.
        for rec in list.script_records().iter().take(MAX_SCRIPT_RECORDS) {
            let langs = scripts.entry(rec.script_tag().to_string()).or_default();
            if let Ok(script) = rec.script(data) {
                for ls in script.lang_sys_records().iter().take(MAX_LANG_SYS) {
                    langs.insert(ls.lang_sys_tag().to_string());
                }
            }
        }
    };

    if let Ok(t) = font.gsub() {
        if let Ok(fl) = t.feature_list() {
            for f in fl.feature_records() {
                gsub.insert(f.feature_tag().to_string());
            }
        }
        if let Ok(sl) = t.script_list() {
            add_scripts(sl);
        }
    }
    if let Ok(t) = font.gpos() {
        if let Ok(fl) = t.feature_list() {
            for f in fl.feature_records() {
                gpos.insert(f.feature_tag().to_string());
            }
        }
        if let Ok(sl) = t.script_list() {
            add_scripts(sl);
        }
    }
    // Both maps are ordered, so the sorting the old shape did by hand comes for free.
    Features {
        gsub: gsub.into_iter().collect(),
        gpos: gpos.into_iter().collect(),
        scripts: scripts
            .into_iter()
            .map(|(tag, languages)| ScriptInfo {
                tag,
                languages: languages.into_iter().collect(),
            })
            .collect(),
    }
}

fn capabilities(font: &FontRef) -> Capabilities {
    let has = |t: &[u8; 4]| font.table_data(Tag::new(t)).is_some();
    let outlines = if has(b"glyf") {
        OutlineFormat::Glyf
    } else if has(b"CFF2") {
        OutlineFormat::Cff2
    } else if has(b"CFF ") {
        OutlineFormat::Cff
    } else {
        OutlineFormat::None
    };
    let mut color = Vec::new();
    if let Ok(colr) = font.colr() {
        color.push(if colr.version() >= 1 {
            ColorFormat::Colrv1
        } else {
            ColorFormat::Colrv0
        });
    }
    if has(b"SVG ") {
        color.push(ColorFormat::Svg);
    }
    if has(b"sbix") {
        color.push(ColorFormat::Sbix);
    }
    if has(b"CBDT") {
        color.push(ColorFormat::Cbdt);
    }
    Capabilities {
        outlines,
        color,
        hinting: has(b"fpgm") || has(b"prep") || has(b"cvt "),
        bitmap_strikes: has(b"EBDT") || has(b"CBDT") || has(b"sbix") || has(b"bdat"),
        math: has(b"MATH"),
        kern_table: has(b"kern"),
    }
}

fn long_date_time_to_rfc3339(secs_since_1904: i64) -> Option<String> {
    if secs_since_1904 <= 0 {
        return None;
    }
    // Seconds between 1904-01-01 and 1970-01-01.
    let unix = secs_since_1904 - 2_082_844_800;
    let dt = time::OffsetDateTime::from_unix_timestamp(unix).ok()?;
    dt.format(&time::format_description::well_known::Rfc3339)
        .ok()
}

fn identity_hash(font: &FontRef) -> String {
    let mut h = blake3::Hasher::new();
    for tag in [b"name", b"glyf", b"CFF ", b"CFF2", b"gvar"] {
        if let Some(data) = font.table_data(Tag::new(tag)) {
            h.update(tag);
            h.update(data.as_bytes());
        }
    }
    h.finalize().to_hex().to_string()
}

fn css_number(v: f32) -> String {
    if (v - v.round()).abs() < 1e-3 {
        format!("{}", v.round() as i64)
    } else {
        format!("{v:.2}")
    }
}

fn style(font: &FontRef, var: Option<&VariableInfo>, container: Container, family: &str) -> Style {
    let attrs = font.attributes();
    let mut weight = attrs.weight.value();
    let mut width = attrs.stretch.ratio() * 100.0;
    let slope = match attrs.style {
        skrifa::attribute::Style::Normal => Slope::Normal,
        skrifa::attribute::Style::Italic => Slope::Italic,
        skrifa::attribute::Style::Oblique(angle) => Slope::Oblique { angle },
    };

    let axis = |tag: &str| var.and_then(|v| v.axes.iter().find(|a| a.tag == tag));
    let mut css_weight = css_number(weight);
    let mut css_stretch = format!("{}%", css_number(width));
    let mut css_style = match &slope {
        Slope::Normal => "normal".to_string(),
        Slope::Italic => "italic".to_string(),
        Slope::Oblique { angle: Some(a) } => format!("oblique {}deg", css_number(*a)),
        Slope::Oblique { angle: None } => "oblique".to_string(),
    };
    if let Some(a) = axis("wght") {
        weight = a.default;
        css_weight = format!("{} {}", css_number(a.min), css_number(a.max));
    }
    if let Some(a) = axis("wdth") {
        width = a.default;
        css_stretch = format!("{}% {}%", css_number(a.min), css_number(a.max));
    }
    if let Some(a) = axis("slnt") {
        // CSS oblique angle is positive for a forward (rightward) lean; `slnt` is the reverse.
        css_style = format!(
            "oblique {}deg {}deg",
            css_number(-a.max),
            css_number(-a.min)
        );
    }

    Style {
        weight,
        width,
        slope,
        css: CssDescriptor {
            family: family.to_string(),
            weight: css_weight,
            stretch: css_stretch,
            style: css_style,
            format: container.css_format().to_string(),
        },
    }
}

pub(crate) fn parse_face(font: &FontRef, index: u32, file: &FileInfo) -> Result<FaceMetadata> {
    parse_one(font, index, file).map_err(|e| Error::Parse(format!("face {index}: {e}")))
}

fn parse_one(font: &FontRef, index: u32, file: &FileInfo) -> Result<FaceMetadata> {
    let head = font.head()?;
    let hhea = font.hhea()?;
    let post = font.post().ok();
    let os2 = font.os2().ok();
    let glyph_count = font.maxp().map(|m| m.num_glyphs()).unwrap_or(0);

    let names = names(font);
    let variable = variable(font);
    let style = style(font, variable.as_ref(), file.container, &names.family);

    let metrics = Metrics {
        units_per_em: head.units_per_em(),
        ascender: hhea.ascender().to_i16(),
        descender: hhea.descender().to_i16(),
        line_gap: hhea.line_gap().to_i16(),
        x_height: os2.as_ref().and_then(|o| o.sx_height()),
        cap_height: os2.as_ref().and_then(|o| o.s_cap_height()),
        italic_angle: post
            .as_ref()
            .map(|p| p.italic_angle().to_f32())
            .unwrap_or(0.0),
        is_fixed_pitch: post.as_ref().is_some_and(|p| p.is_fixed_pitch() != 0),
        revision: head.font_revision().to_f64(),
        created: long_date_time_to_rfc3339(head.created().as_secs()),
        modified: long_date_time_to_rfc3339(head.modified().as_secs()),
    };

    let os2_info = os2.as_ref().map(|o| Os2Info {
        version: o.version(),
        weight_class: o.us_weight_class(),
        width_class: o.us_width_class(),
        fs_type: o.fs_type(),
        embedding: EmbeddingRights::from_fs_type(o.fs_type()),
        vendor_id: o.ach_vend_id().to_string().trim().to_string(),
        fs_selection: o.fs_selection().bits(),
        use_typo_metrics: o.fs_selection().bits() & 0x80 != 0,
        unicode_ranges: [
            o.ul_unicode_range_1(),
            o.ul_unicode_range_2(),
            o.ul_unicode_range_3(),
            o.ul_unicode_range_4(),
        ],
        codepage_ranges: match (o.ul_code_page_range_1(), o.ul_code_page_range_2()) {
            (Some(a), Some(b)) => Some([a, b]),
            _ => None,
        },
        typo_ascender: Some(o.s_typo_ascender()),
        typo_descender: Some(o.s_typo_descender()),
    });

    let codepoints: Vec<u32> = font.charmap().mappings().map(|(cp, _)| cp).collect();
    let coverage = coverage_from_codepoints(codepoints);

    let license_description = english(font, StringId::LICENSE_DESCRIPTION);
    let license_url = english(font, StringId::LICENSE_URL);
    let spdx = spdx_from_names(license_description.as_deref(), license_url.as_deref());
    let license = LicenseInfo {
        freedom: crate::freedom::classify(spdx.as_deref()),
        spdx,
        reserved_font_names: crate::license::reserved_font_names(&[
            names.copyright.as_deref(),
            license_description.as_deref(),
        ]),
        description: license_description,
        url: license_url,
    };

    Ok(FaceMetadata {
        schema_version: crate::SCHEMA_VERSION,
        file: file.clone(),
        index,
        name_records: name_records(font),
        names,
        style,
        metrics,
        os2: os2_info,
        variable,
        features: features(font),
        coverage,
        capabilities: capabilities(font),
        license,
        glyph_count,
        identity_hash: identity_hash(font),
    })
}

/// CSS `unicode-range` for a face.
pub fn unicode_range(face: &FaceMetadata) -> String {
    unicode_range_css(&face.coverage.ranges)
}
