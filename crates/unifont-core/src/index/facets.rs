//! Facet counts and family grouping over a filter. Both are computed from one lean row
//! scan; with 50k faces that is a few milliseconds, well inside the search budget.

use super::{FaceFilter, FaceSummary, Index};
use crate::error::Result;
use schemars::JsonSchema;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct FacetCount {
    pub value: String,
    pub count: i64,
}

/// Counts of faces per facet value, for the faces matching a filter.
#[derive(Debug, Clone, Default, Serialize, JsonSchema)]
pub struct Facets {
    pub faces: i64,
    pub families: i64,
    /// CSS weight buckets, `100`..`900`.
    pub weight: Vec<FacetCount>,
    /// Width buckets as percentages, `50`..`200`.
    pub width: Vec<FacetCount>,
    /// `upright` or `italic`.
    pub style: Vec<FacetCount>,
    pub variable: i64,
    pub color: i64,
    pub container: Vec<FacetCount>,
    /// ISO 15924 script codes.
    pub script: Vec<FacetCount>,
    pub license: Vec<FacetCount>,
    pub vendor: Vec<FacetCount>,
    pub tag: Vec<FacetCount>,
    pub collection: Vec<FacetCount>,
    /// `session`, `user`, `installed`, or `none`.
    pub activation: Vec<FacetCount>,
    /// Registered source directories the faces live under.
    pub source: Vec<FacetCount>,
}

/// Faces grouped by typographic family name.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Family {
    pub name: String,
    pub faces: usize,
    pub ids: Vec<i64>,
    /// The face to show for the family: upright, closest to weight 400 and width 100.
    pub representative: i64,
    pub variable: bool,
    pub color: bool,
    pub italic: bool,
    /// Lowest and highest weight in the family.
    pub weights: [f32; 2],
    pub widths: [f32; 2],
    pub scripts: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vendor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub designer: Option<String>,
    pub containers: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Faces with an activation record.
    pub active: usize,
    #[serde(skip)]
    rep_score: f32,
}

/// Nearest CSS weight bucket, 100..=900.
pub fn weight_bucket(weight: f32) -> u16 {
    ((weight / 100.0).round() as u16 * 100).clamp(100, 900)
}

/// Name of a CSS weight bucket.
pub fn weight_name(bucket: u16) -> &'static str {
    match bucket {
        100 => "Thin",
        200 => "ExtraLight",
        300 => "Light",
        400 => "Regular",
        500 => "Medium",
        600 => "SemiBold",
        700 => "Bold",
        800 => "ExtraBold",
        _ => "Black",
    }
}

const WIDTH_BUCKETS: &[(f32, &str)] = &[
    (50.0, "UltraCondensed"),
    (62.5, "ExtraCondensed"),
    (75.0, "Condensed"),
    (87.5, "SemiCondensed"),
    (100.0, "Normal"),
    (112.5, "SemiExpanded"),
    (125.0, "Expanded"),
    (150.0, "ExtraExpanded"),
    (200.0, "UltraExpanded"),
];

/// Nearest `usWidthClass` percentage.
pub fn width_bucket(width: f32) -> f32 {
    WIDTH_BUCKETS
        .iter()
        .min_by(|a, b| (a.0 - width).abs().total_cmp(&(b.0 - width).abs()))
        .map(|b| b.0)
        .unwrap_or(100.0)
}

pub fn width_name(bucket: f32) -> &'static str {
    WIDTH_BUCKETS
        .iter()
        .find(|b| (b.0 - bucket).abs() < 0.01)
        .map(|b| b.1)
        .unwrap_or("Normal")
}

fn counts(map: BTreeMap<String, i64>) -> Vec<FacetCount> {
    map.into_iter()
        .map(|(value, count)| FacetCount { value, count })
        .collect()
}

fn counts_by_count(map: BTreeMap<String, i64>) -> Vec<FacetCount> {
    let mut v = counts(map);
    v.sort_by(|a, b| b.count.cmp(&a.count).then(a.value.cmp(&b.value)));
    v
}

fn fmt_width(w: f32) -> String {
    if w.fract() == 0.0 {
        format!("{}", w as i64)
    } else {
        format!("{w}")
    }
}

impl Index {
    pub fn facets(&self, filter: &FaceFilter) -> Result<Facets> {
        let w = Self::where_for(filter);
        let sql = format!(
            "SELECT f.family, f.weight, f.width, f.italic, f.is_variable, f.is_color, fi.container,
                    f.scripts, f.license_spdx, f.vendor, a.scope, fi.path
             FROM faces f JOIN files fi ON fi.id = f.file_id LEFT JOIN activations a ON a.face_id = f.id{}",
            w.sql()
        );
        let sources = self.sources()?;
        let mut out = Facets::default();
        let mut families = std::collections::HashSet::new();
        let (
            mut weight,
            mut width,
            mut style,
            mut container,
            mut script,
            mut license,
            mut vendor,
            mut activation,
            mut source,
        ) = (
            BTreeMap::new(),
            BTreeMap::new(),
            BTreeMap::new(),
            BTreeMap::new(),
            BTreeMap::new(),
            BTreeMap::new(),
            BTreeMap::new(),
            BTreeMap::new(),
            BTreeMap::new(),
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(w.params()), |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, f32>(1)?,
                r.get::<_, f32>(2)?,
                r.get::<_, bool>(3)?,
                r.get::<_, bool>(4)?,
                r.get::<_, bool>(5)?,
                r.get::<_, String>(6)?,
                r.get::<_, String>(7)?,
                r.get::<_, Option<String>>(8)?,
                r.get::<_, Option<String>>(9)?,
                r.get::<_, Option<String>>(10)?,
                r.get::<_, String>(11)?,
            ))
        })?;
        for row in rows {
            let (family, wt, wd, italic, variable, color, cont, scripts, lic, ven, act, path) =
                row?;
            out.faces += 1;
            families.insert(family.to_lowercase());
            *weight.entry(weight_bucket(wt).to_string()).or_default() += 1;
            *width.entry(fmt_width(width_bucket(wd))).or_default() += 1;
            *style
                .entry(if italic { "italic" } else { "upright" }.to_string())
                .or_default() += 1;
            if variable {
                out.variable += 1;
            }
            if color {
                out.color += 1;
            }
            *container.entry(cont).or_default() += 1;
            for s in scripts.split(',').filter(|s| !s.is_empty()) {
                *script.entry(s.to_string()).or_default() += 1;
            }
            *license
                .entry(lic.unwrap_or_else(|| "none".into()))
                .or_default() += 1;
            if let Some(v) = ven.filter(|v| !v.trim().is_empty()) {
                *vendor.entry(v.trim().to_string()).or_default() += 1;
            }
            *activation
                .entry(act.unwrap_or_else(|| "none".into()))
                .or_default() += 1;
            for s in &sources {
                if path.starts_with(&s.path) {
                    *source.entry(s.path.clone()).or_default() += 1;
                }
            }
        }
        out.families = families.len() as i64;
        // Weight and width sort numerically, not lexically.
        let mut weight = counts(weight);
        weight.sort_by_key(|c| c.value.parse::<u16>().unwrap_or(0));
        let mut width = counts(width);
        width.sort_by(|a, b| {
            a.value
                .parse::<f32>()
                .unwrap_or(0.0)
                .total_cmp(&b.value.parse::<f32>().unwrap_or(0.0))
        });
        out.weight = weight;
        out.width = width;
        out.style = counts(style);
        out.container = counts_by_count(container);
        out.script = counts_by_count(script);
        out.license = counts_by_count(license);
        out.vendor = counts_by_count(vendor);
        out.activation = counts(activation);
        out.source = counts(source);

        let inner = format!(
            "SELECT f.id FROM faces f JOIN files fi ON fi.id = f.file_id LEFT JOIN activations a ON a.face_id = f.id{}",
            w.sql()
        );
        let mut stmt = self.conn.prepare(&format!(
            "SELECT t.name, COUNT(*) FROM face_tags ft JOIN tags t ON t.id = ft.tag_id WHERE ft.face_id IN ({inner}) GROUP BY t.id ORDER BY t.name COLLATE NOCASE"
        ))?;
        let rows = stmt.query_map(rusqlite::params_from_iter(w.params()), |r| {
            Ok(FacetCount {
                value: r.get(0)?,
                count: r.get(1)?,
            })
        })?;
        out.tag = rows.collect::<rusqlite::Result<Vec<_>>>()?;
        let mut stmt = self.conn.prepare(&format!(
            "SELECT c.name, COUNT(*) FROM collection_faces cf JOIN collections c ON c.id = cf.collection_id WHERE cf.face_id IN ({inner}) GROUP BY c.id ORDER BY c.name COLLATE NOCASE"
        ))?;
        let rows = stmt.query_map(rusqlite::params_from_iter(w.params()), |r| {
            Ok(FacetCount {
                value: r.get(0)?,
                count: r.get(1)?,
            })
        })?;
        out.collection = rows.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(out)
    }

    /// Faces matching the filter, grouped by family. `filter.limit` caps families.
    pub fn families(&self, filter: &FaceFilter) -> Result<Vec<Family>> {
        let faces = self.list(&FaceFilter {
            limit: None,
            ..filter.clone()
        })?;
        let mut out: Vec<Family> = Vec::new();
        for f in faces {
            match out.last_mut() {
                Some(fam) if fam.name.eq_ignore_ascii_case(&f.family) => fam.push(&f),
                _ => out.push(Family::new(&f)),
            }
        }
        for fam in &mut out {
            fam.finish();
        }
        if let Some(n) = filter.limit {
            out.truncate(n);
        }
        Ok(out)
    }
}

impl Family {
    fn new(f: &FaceSummary) -> Family {
        let mut fam = Family {
            name: f.family.clone(),
            faces: 0,
            ids: Vec::new(),
            representative: f.id,
            variable: false,
            color: false,
            italic: false,
            weights: [f.weight, f.weight],
            widths: [f.width, f.width],
            scripts: f.scripts.clone(),
            license: f.license.clone(),
            vendor: f.vendor.clone(),
            designer: f.designer.clone(),
            containers: Vec::new(),
            tags: Vec::new(),
            active: 0,
            rep_score: f32::MAX,
        };
        fam.push(f);
        fam
    }

    /// Distance from "the regular face"; lower is more representative.
    fn score(f: &FaceSummary) -> f32 {
        (f.weight - 400.0).abs() + (f.width - 100.0).abs() + if f.italic { 1000.0 } else { 0.0 }
    }

    fn push(&mut self, f: &FaceSummary) {
        self.faces += 1;
        self.ids.push(f.id);
        self.variable |= f.variable;
        self.color |= f.color;
        self.italic |= f.italic;
        self.weights[0] = self.weights[0].min(f.weight);
        self.weights[1] = self.weights[1].max(f.weight);
        self.widths[0] = self.widths[0].min(f.width);
        self.widths[1] = self.widths[1].max(f.width);
        if self.license.is_none() {
            self.license = f.license.clone();
        }
        if !self.containers.contains(&f.container) {
            self.containers.push(f.container.clone());
        }
        for t in &f.tags {
            if !self.tags.contains(t) {
                self.tags.push(t.clone());
            }
        }
        if f.activation.is_some() {
            self.active += 1;
        }
        if Self::score(f) < self.rep_score {
            self.representative = f.id;
            self.rep_score = Self::score(f);
            self.scripts = f.scripts.clone();
        }
    }

    fn finish(&mut self) {
        self.tags.sort();
        self.containers.sort();
    }
}
