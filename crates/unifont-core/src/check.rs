//! Health checks: a fontbakery-lite pass over one face's metadata. Every check has a
//! stable id so results can be filtered, suppressed or tracked over time.

use crate::model::*;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct Finding {
    /// Stable identifier, `area/check`.
    pub id: &'static str,
    pub severity: Severity,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct CheckReport {
    pub path: String,
    pub index: u32,
    pub family: String,
    pub subfamily: String,
    pub findings: Vec<Finding>,
    pub errors: usize,
    pub warnings: usize,
}

impl CheckReport {
    pub fn passed(&self, strict: bool) -> bool {
        self.errors == 0 && (!strict || self.warnings == 0)
    }
}

struct Ctx<'a> {
    f: &'a FaceMetadata,
    out: Vec<Finding>,
}

impl Ctx<'_> {
    fn push(&mut self, id: &'static str, severity: Severity, message: impl Into<String>) {
        self.out.push(Finding {
            id,
            severity,
            message: message.into(),
        });
    }
    fn error(&mut self, id: &'static str, m: impl Into<String>) {
        self.push(id, Severity::Error, m)
    }
    fn warn(&mut self, id: &'static str, m: impl Into<String>) {
        self.push(id, Severity::Warn, m)
    }
    fn info(&mut self, id: &'static str, m: impl Into<String>) {
        self.push(id, Severity::Info, m)
    }
}

/// Unicode script (ISO 15924) to the OpenType script tags that shape it. A face that
/// covers one of these scripts without a matching GSUB script will not shape correctly.
const SHAPING_SCRIPTS: &[(&str, &[&str])] = &[
    ("Arab", &["arab"]),
    ("Syrc", &["syrc"]),
    ("Hebr", &["hebr"]),
    ("Deva", &["deva", "dev2"]),
    ("Beng", &["beng", "bng2"]),
    ("Guru", &["guru", "gur2"]),
    ("Gujr", &["gujr", "gjr2"]),
    ("Orya", &["orya", "ory2"]),
    ("Taml", &["taml", "tml2"]),
    ("Telu", &["telu", "tel2"]),
    ("Knda", &["knda", "knd2"]),
    ("Mlym", &["mlym", "mlm2"]),
    ("Sinh", &["sinh"]),
    ("Thai", &["thai"]),
    ("Khmr", &["khmr"]),
    ("Mymr", &["mymr", "mym2"]),
    ("Tibt", &["tibt"]),
];

pub fn check_face(f: &FaceMetadata) -> CheckReport {
    let mut c = Ctx { f, out: Vec::new() };
    names(&mut c);
    os2(&mut c);
    metrics(&mut c);
    coverage(&mut c);
    variable(&mut c);
    layout(&mut c);
    license(&mut c);
    file(&mut c);
    c.out
        .sort_by(|a, b| b.severity.cmp(&a.severity).then_with(|| a.id.cmp(b.id)));
    let errors = c
        .out
        .iter()
        .filter(|x| x.severity == Severity::Error)
        .count();
    let warnings = c
        .out
        .iter()
        .filter(|x| x.severity == Severity::Warn)
        .count();
    CheckReport {
        path: f.file.path.clone(),
        index: f.index,
        family: f.names.family.clone(),
        subfamily: f.names.subfamily.clone(),
        findings: c.out,
        errors,
        warnings,
    }
}

fn names(c: &mut Ctx) {
    let n = &c.f.names;
    if n.family.trim().is_empty() {
        c.error(
            "name/family",
            "no family name (name IDs 1 and 16 are empty)",
        );
    }
    match &n.postscript_name {
        None => c.error("name/postscript", "no PostScript name (name ID 6)"),
        Some(ps) => {
            if ps.len() > 63 {
                c.error(
                    "name/postscript",
                    format!("PostScript name is {} bytes; the limit is 63", ps.len()),
                );
            }
            if ps
                .chars()
                .any(|ch| !(ch.is_ascii_graphic()) || "[](){}<>/%".contains(ch))
            {
                c.error("name/postscript", format!("PostScript name {ps:?} contains spaces or characters outside printable ASCII"));
            }
        }
    }
    match &n.version {
        None => c.warn("name/version", "no version string (name ID 5)"),
        Some(v) => {
            let parsed = v
                .trim_start_matches(|ch: char| !ch.is_ascii_digit())
                .split(|ch: char| !(ch.is_ascii_digit() || ch == '.'))
                .next()
                .and_then(|s| s.parse::<f64>().ok());
            if let Some(p) = parsed
                && (p - c.f.metrics.revision).abs() > 0.0005
            {
                c.warn(
                    "name/version",
                    format!(
                        "version string {v:?} does not match head.fontRevision {:.3}",
                        c.f.metrics.revision
                    ),
                );
            }
        }
    }
    if let (Some(full), false) = (&n.full_name, n.family.is_empty()) {
        let expect_rr = if n.subfamily == "Regular" {
            n.family.clone()
        } else {
            format!("{} {}", n.family, n.subfamily)
        };
        if !full.eq_ignore_ascii_case(&expect_rr) && !full.starts_with(&n.family) {
            c.info(
                "name/full-name",
                format!(
                    "full name {full:?} does not start with the family name {:?}",
                    n.family
                ),
            );
        }
    }
    if n.designer.is_none() && n.manufacturer.is_none() {
        c.info(
            "name/designer",
            "no designer or manufacturer recorded (name IDs 8, 9)",
        );
    }
}

fn os2(c: &mut Ctx) {
    let Some(o) = &c.f.os2 else {
        c.error("os2/missing", "no OS/2 table");
        return;
    };
    if !(1..=1000).contains(&o.weight_class) {
        c.error(
            "os2/weight-class",
            format!("usWeightClass {} is outside 1..1000", o.weight_class),
        );
    } else if !o.weight_class.is_multiple_of(50) && o.weight_class > 50 && !c.f.is_variable() {
        c.info(
            "os2/weight-class",
            format!("usWeightClass {} is not a multiple of 50", o.weight_class),
        );
    }
    if !(1..=9).contains(&o.width_class) {
        c.error(
            "os2/width-class",
            format!("usWidthClass {} is outside 1..9", o.width_class),
        );
    }
    if o.vendor_id.is_empty() || o.vendor_id == "UKWN" {
        c.info("os2/vendor-id", "achVendID is unset or UKWN");
    }
    let sel_italic = o.fs_selection & 0x01 != 0;
    let sel_bold = o.fs_selection & 0x20 != 0;
    let sel_regular = o.fs_selection & 0x40 != 0;
    if sel_regular && (sel_bold || sel_italic) {
        c.warn(
            "os2/fs-selection",
            "fsSelection sets REGULAR together with BOLD or ITALIC",
        );
    }
    if sel_italic && c.f.metrics.italic_angle == 0.0 && !c.f.is_variable() {
        c.warn(
            "os2/italic-angle",
            "fsSelection says italic but post.italicAngle is 0",
        );
    }
    if !sel_italic && c.f.metrics.italic_angle != 0.0 {
        c.warn(
            "os2/italic-angle",
            format!(
                "post.italicAngle is {} but fsSelection ITALIC is not set",
                c.f.metrics.italic_angle
            ),
        );
    }
    if sel_bold && o.weight_class < 600 {
        c.warn(
            "os2/bold-weight",
            format!(
                "fsSelection BOLD is set but usWeightClass is {}",
                o.weight_class
            ),
        );
    }
    match o.embedding.level {
        EmbeddingLevel::RestrictedLicense => c.info(
            "os2/fs-type",
            "fsType forbids embedding (Restricted License)",
        ),
        EmbeddingLevel::PreviewAndPrint => c.info(
            "os2/fs-type",
            "fsType allows preview & print embedding only",
        ),
        _ => {}
    }
    if o.embedding.bitmap_only {
        c.info("os2/fs-type", "fsType allows bitmap embedding only");
    }
    if let (Some(ta), Some(td)) = (o.typo_ascender, o.typo_descender) {
        let m = &c.f.metrics;
        if !o.use_typo_metrics && (ta != m.ascender || td != m.descender) {
            c.warn(
                "metrics/typo-vs-hhea",
                format!("OS/2 typo metrics ({ta}/{td}) differ from hhea ({}/{}) and USE_TYPO_METRICS is not set; line height will vary by platform", m.ascender, m.descender),
            );
        }
    }
}

fn metrics(c: &mut Ctx) {
    let m = &c.f.metrics;
    if !(16..=16384).contains(&m.units_per_em) {
        c.error(
            "head/units-per-em",
            format!("unitsPerEm {} is outside 16..16384", m.units_per_em),
        );
    }
    if m.ascender <= 0 {
        c.error("hhea/ascender", format!("hhea.ascender is {}", m.ascender));
    }
    if m.descender > 0 {
        c.warn(
            "hhea/descender",
            format!("hhea.descender is positive ({})", m.descender),
        );
    }
    if m.created.is_none() {
        c.info("head/created", "head.created is unset");
    }
}

fn coverage(c: &mut Ctx) {
    let cov = &c.f.coverage;
    if c.f.glyph_count == 0 {
        c.error("glyf/empty", "font has no glyphs");
    }
    if cov.codepoints == 0 {
        c.error("cmap/empty", "cmap maps no codepoints");
        return;
    }
    let has = |cp: u32| cov.ranges.iter().any(|[lo, hi]| *lo <= cp && cp <= *hi);
    if !has(0x20) {
        c.warn("cmap/space", "U+0020 SPACE is not mapped");
    }
    if !has(0xA0) && has(0x20) {
        c.info("cmap/nbsp", "U+00A0 NO-BREAK SPACE is not mapped");
    }
    if !(0x41..=0x5A).all(has) && cov.scripts.first().is_some_and(|s| s.script == "Latn") {
        c.warn("cmap/basic-latin", "Latin font does not map all of A–Z");
    }
    if c.f.capabilities.outlines == OutlineFormat::None && !c.f.capabilities.bitmap_strikes {
        c.error(
            "outlines/none",
            "no glyf, CFF or CFF2 outlines and no bitmap strikes",
        );
    }
    if !c.f.capabilities.hinting && c.f.capabilities.outlines == OutlineFormat::Glyf {
        c.info("hinting/none", "TrueType outlines without hinting programs");
    }
}

fn variable(c: &mut Ctx) {
    let Some(v) = &c.f.variable else { return };
    if !v.has_stat {
        c.warn(
            "fvar/stat",
            "variable font without a STAT table; style linking will be wrong in many apps",
        );
    }
    if v.instances.is_empty() {
        c.warn("fvar/instances", "variable font without named instances");
    }
    for a in &v.axes {
        if !(a.min <= a.default && a.default <= a.max) {
            c.error(
                "fvar/axis-range",
                format!(
                    "axis {} default {} is outside {}..{}",
                    a.tag, a.default, a.min, a.max
                ),
            );
        }
        if a.min == a.max {
            c.warn(
                "fvar/axis-range",
                format!("axis {} has a zero-width range", a.tag),
            );
        }
        let registered = ["wght", "wdth", "ital", "slnt", "opsz"];
        if a.tag.chars().all(|ch| ch.is_ascii_lowercase()) && !registered.contains(&a.tag.as_str())
        {
            c.warn("fvar/axis-tag", format!("axis tag {} is lowercase but not a registered axis; custom tags should be uppercase", a.tag));
        }
        if a.tag == "wght"
            && let Some(o) = &c.f.os2
            && (o.weight_class as f32 - a.default).abs() > 0.5
        {
            c.warn(
                "fvar/wght-os2",
                format!(
                    "wght default {} does not match usWeightClass {}",
                    a.default, o.weight_class
                ),
            );
        }
    }
    for inst in &v.instances {
        for (a, &coord) in v.axes.iter().zip(inst.coordinates.iter()) {
            if coord < a.min || coord > a.max {
                c.error(
                    "fvar/instance-range",
                    format!(
                        "instance {:?} sets {} to {} outside {}..{}",
                        inst.name.as_deref().unwrap_or("?"),
                        a.tag,
                        coord,
                        a.min,
                        a.max
                    ),
                );
            }
        }
        if inst.name.is_none() {
            c.warn(
                "fvar/instance-name",
                "named instance without a resolvable name",
            );
        }
    }
}

fn layout(c: &mut Ctx) {
    let tags: Vec<&str> =
        c.f.features
            .scripts
            .iter()
            .map(|s| s.tag.as_str())
            .collect();
    for (script, ot) in SHAPING_SCRIPTS {
        let covered =
            c.f.coverage
                .scripts
                .iter()
                .find(|s| s.script == *script)
                .map(|s| s.codepoints)
                .unwrap_or(0);
        if covered >= 20 && !ot.iter().any(|t| tags.contains(t)) {
            c.warn("layout/shaping", format!("covers {covered} {script} codepoints but has no {} script in GSUB/GPOS; text will not shape", ot.join("/")));
        }
    }
    if c.f.features.gpos.is_empty() && !c.f.capabilities.kern_table && c.f.glyph_count > 100 {
        c.info("layout/kerning", "no GPOS table and no legacy kern table");
    }
}

fn license(c: &mut Ctx) {
    let l = &c.f.license;
    match l.spdx.as_deref() {
        None => c.warn("license/missing", "no license text or URL embedded (name IDs 13, 14)"),
        Some("LicenseRef-Unknown") => c.warn("license/unknown", "license text is present but not recognised; consider naming the license and its SPDX identifier"),
        Some("OFL-1.1") => {
            if l.url.is_none() {
                c.info("license/url", "OFL font without a license URL (name ID 14)");
            }
            if !l.reserved_font_names.is_empty() {
                let fam = c.f.names.family.to_ascii_lowercase();
                for rfn in &l.reserved_font_names {
                    if !fam.contains(&rfn.to_ascii_lowercase()) {
                        c.info("license/rfn", format!("Reserved Font Name {rfn:?} is declared but the family is {:?}", c.f.names.family));
                    }
                }
            }
        }
        _ => {}
    }
    if c.f.names.copyright.is_none() {
        c.warn("license/copyright", "no copyright notice (name ID 0)");
    }
}

fn file(c: &mut Ctx) {
    let ext = std::path::Path::new(&c.f.file.path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();
    let expected = match (c.f.file.container, c.f.capabilities.outlines) {
        (Container::Ttf, OutlineFormat::Cff | OutlineFormat::Cff2) => Some("otf"),
        (Container::Otf, OutlineFormat::Glyf) => Some("ttf"),
        (Container::Ttf, _) => Some("ttf"),
        (Container::Otf, _) => Some("otf"),
        (Container::Ttc, _) => Some("ttc"),
        (Container::Woff, _) => Some("woff"),
        (Container::Woff2, _) => Some("woff2"),
    };
    if let Some(exp) = expected
        && ext != exp
        && !(exp == "ttc" && ext == "otc")
    {
        c.warn(
            "file/extension",
            format!(
                "extension .{ext} but the container is {} with {:?} outlines; expected .{exp}",
                c.f.file.container.as_str(),
                c.f.capabilities.outlines
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_orders() {
        assert!(Severity::Error > Severity::Warn && Severity::Warn > Severity::Info);
    }
}
