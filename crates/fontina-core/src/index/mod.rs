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

//! SQLite index. One file, WAL mode, FTS5 for names. The full `FaceMetadata` JSON is
//! stored per face so `info` round-trips without re-parsing the font.
//!
//! - this module: open, scan bookkeeping, listing and filtering, duplicates, stats
//! - [`library`]: tags, collections (with JSON export/import), sources, activation state,
//!   conflicts
//! - [`facets`]: facet counts and family grouping over a filter

mod facets;
mod library;
mod schema;

pub use facets::{
    FacetCount, Facets, Family, weight_bucket, weight_name, width_bucket, width_name,
};
pub use library::{
    ActivationRecord, ActivationState, CollectionExport, CollectionFace, CollectionInfo, Conflict,
    ImportReport, Source, SourceKind, TagInfo,
};

use crate::FileInfo;
use crate::error::Result;
use crate::freedom::{self, Freedom};
use crate::model::FaceMetadata;
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub struct Index {
    conn: Connection,
}

/// Compact per-face row used by listings.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FaceSummary {
    pub id: i64,
    pub path: String,
    pub index: u32,
    pub family: String,
    pub subfamily: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postscript_name: Option<String>,
    pub weight: f32,
    pub width: f32,
    pub italic: bool,
    pub variable: bool,
    pub color: bool,
    pub glyph_count: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    /// Whether `license` grants the four freedoms. Derived on read, never stored.
    #[serde(default)]
    pub freedom: Freedom,
    pub scripts: Vec<String>,
    pub container: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vendor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub designer: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Activation state recorded by fontina, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activation: Option<ActivationState>,
}

#[derive(Debug, Clone, Default)]
pub struct FaceFilter {
    /// Full-text query over family, subfamily, PostScript name and designer.
    pub query: Option<String>,
    /// Exact (case-insensitive) family match.
    pub family: Option<String>,
    pub variable: Option<bool>,
    pub color: Option<bool>,
    pub italic: Option<bool>,
    /// ISO 15924 script code the face must cover, e.g. `Arab`.
    pub script: Option<String>,
    /// SPDX identifier prefix match, e.g. `OFL`.
    pub license: Option<String>,
    /// Whether the license grants the four freedoms.
    pub freedom: Option<Freedom>,
    pub weight: Option<(u16, u16)>,
    /// Width range in percent, e.g. `(75, 100)`.
    pub width: Option<(u16, u16)>,
    /// Exact (case-insensitive) `OS/2` vendor id.
    pub vendor: Option<String>,
    /// Faces carrying this tag.
    pub tag: Option<String>,
    /// Faces in this collection (by name).
    pub collection: Option<String>,
    /// `Some(true)`: only faces with an activation record; `Some(false)`: only without.
    pub active: Option<bool>,
    /// Only faces in exactly this activation state.
    pub activation: Option<ActivationState>,
    /// Container as in `FaceSummary::container`, e.g. `woff2`.
    pub container: Option<String>,
    pub path_prefix: Option<String>,
    /// Restrict to these face ids.
    pub ids: Option<Vec<i64>>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct DuplicateGroup {
    pub reason: String,
    pub key: String,
    pub faces: Vec<FaceSummary>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Stats {
    pub files: i64,
    pub faces: i64,
    pub families: i64,
    pub variable_faces: i64,
    pub color_faces: i64,
    pub failed_files: i64,
    pub tags: i64,
    pub collections: i64,
    pub sources: i64,
    pub activations: i64,
    pub db_path: String,
}

/// The `WHERE` clauses and their bound values for a filter.
struct Where {
    clauses: Vec<String>,
    args: Vec<Box<dyn rusqlite::ToSql>>,
}

impl Where {
    fn sql(&self) -> String {
        if self.clauses.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", self.clauses.join(" AND "))
        }
    }
    fn params(&self) -> impl Iterator<Item = &dyn rusqlite::ToSql> {
        self.args.iter().map(|a| a.as_ref())
    }
}

impl Index {
    /// Default location: the platform data directory for `fontina`.
    pub fn default_path() -> PathBuf {
        directories::ProjectDirs::from("", "", "fontina")
            .map(|d| d.data_dir().join("index.db"))
            .unwrap_or_else(|| PathBuf::from("fontina-index.db"))
    }

    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| crate::Error::Io(parent.to_path_buf(), e))?;
        }
        let conn = Connection::open(path)?;
        Self::init(conn)
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(mut conn: Connection) -> Result<Self> {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        schema::migrate(&mut conn)?;
        Ok(Index { conn })
    }

    pub fn path(&self) -> String {
        self.conn.path().unwrap_or(":memory:").to_string()
    }

    pub fn begin(&mut self) -> Result<Transaction<'_>> {
        Ok(self.conn.transaction()?)
    }

    pub fn file_is_unchanged(&self, path: &str, size: u64, mtime: i64) -> Result<bool> {
        let row: Option<(i64, i64, Option<String>)> = self
            .conn
            .query_row(
                "SELECT size, mtime, error FROM files WHERE path = ?1",
                params![path],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        Ok(matches!(row, Some((s, m, None)) if s == size as i64 && m == mtime))
    }

    /// Replace a file and its faces. Tags, collection memberships and activation state
    /// of the previous faces carry over by (path, face index).
    pub(crate) fn upsert_file_tx(
        tx: &Transaction,
        file: &FileInfo,
        faces: &[FaceMetadata],
    ) -> Result<()> {
        let carried = library::carry_over_take(tx, &file.path)?;
        tx.execute("DELETE FROM files WHERE path = ?1", params![file.path])?;
        tx.execute(
            "INSERT INTO files (path, size, mtime, blake3, container, face_count, scanned_at, error)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, unixepoch(), NULL)",
            params![file.path, file.size as i64, file.mtime, file.blake3, file.container.as_str(), faces.len() as i64],
        )?;
        let file_id = tx.last_insert_rowid();
        let mut stmt = tx.prepare_cached(
            "INSERT INTO faces (file_id, face_index, postscript_name, family, subfamily, full_name,
                weight, width, italic, is_variable, is_color, glyph_count, license_spdx, vendor,
                version, designer, identity_hash, scripts, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
        )?;
        for face in faces {
            let scripts = format!(
                ",{},",
                face.coverage
                    .scripts
                    .iter()
                    .map(|s| s.script.as_str())
                    .collect::<Vec<_>>()
                    .join(",")
            );
            let face_id = stmt.insert(params![
                file_id,
                face.index,
                face.names.postscript_name,
                face.names.family,
                face.names.subfamily,
                face.names.full_name,
                face.style.weight,
                face.style.width,
                face.is_italic(),
                face.is_variable(),
                face.is_color(),
                face.glyph_count,
                face.license.spdx,
                face.os2.as_ref().map(|o| o.vendor_id.clone()),
                face.names.version,
                face.names.designer,
                face.identity_hash,
                scripts,
                serde_json::to_string(face)?,
            ])?;
            insert_ranges(tx, face_id, &face.coverage.ranges)?;
            library::carry_over_apply(tx, face_id, face.index, &carried)?;
        }
        Ok(())
    }

    pub(crate) fn record_failure_tx(tx: &Transaction, path: &str, error: &str) -> Result<()> {
        tx.execute("DELETE FROM files WHERE path = ?1", params![path])?;
        tx.execute(
            "INSERT INTO files (path, size, mtime, blake3, container, face_count, scanned_at, error)
             VALUES (?1, 0, 0, '', '', 0, unixepoch(), ?2)",
            params![path, error],
        )?;
        Ok(())
    }

    /// Remove files under `root` that no longer exist on disk. Returns the count removed.
    pub fn prune_missing(&mut self, root: &str) -> Result<usize> {
        let paths: Vec<String> = {
            let mut stmt = self
                .conn
                .prepare("SELECT path FROM files WHERE path = ?1 OR path LIKE ?2 ESCAPE '\\'")?;
            let rows =
                stmt.query_map(params![root, like_prefix(root)], |r| r.get::<_, String>(0))?;
            rows.filter_map(|r| r.ok()).collect()
        };
        let missing: Vec<&String> = paths.iter().filter(|p| !Path::new(p).exists()).collect();
        let tx = self.conn.transaction()?;
        for p in &missing {
            tx.execute("DELETE FROM files WHERE path = ?1", params![p])?;
        }
        tx.commit()?;
        Ok(missing.len())
    }

    /// Forget one file (and its faces). Returns whether it was indexed.
    pub fn remove_file(&mut self, path: &str) -> Result<bool> {
        Ok(self
            .conn
            .execute("DELETE FROM files WHERE path = ?1", params![path])?
            > 0)
    }

    /// Remove every file under `root` from the index, present on disk or not.
    pub fn remove_under(&mut self, root: &str) -> Result<usize> {
        Ok(self.conn.execute(
            "DELETE FROM files WHERE path = ?1 OR path LIKE ?2 ESCAPE '\\'",
            params![root, like_prefix(root)],
        )?)
    }

    fn row_to_summary(r: &rusqlite::Row) -> rusqlite::Result<FaceSummary> {
        let scripts: String = r.get("scripts")?;
        let tags: Option<String> = r.get("tags")?;
        let activation: Option<String> = r.get("activation")?;
        let license: Option<String> = r.get("license_spdx")?;
        Ok(FaceSummary {
            id: r.get("id")?,
            path: r.get("path")?,
            index: r.get::<_, i64>("face_index")? as u32,
            family: r.get("family")?,
            subfamily: r.get("subfamily")?,
            postscript_name: r.get("postscript_name")?,
            weight: r.get("weight")?,
            width: r.get("width")?,
            italic: r.get("italic")?,
            variable: r.get("is_variable")?,
            color: r.get("is_color")?,
            glyph_count: r.get::<_, i64>("glyph_count")? as u16,
            freedom: crate::freedom::classify(license.as_deref()),
            license,
            scripts: scripts
                .split(',')
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect(),
            container: r.get("container")?,
            vendor: r.get("vendor")?,
            designer: r.get("designer")?,
            tags: tags
                .map(|t| t.split('\u{1f}').map(String::from).collect())
                .unwrap_or_default(),
            activation: activation.and_then(|a| a.parse().ok()),
        })
    }

    const SUMMARY_SELECT: &'static str = "SELECT f.id, fi.path, f.face_index, f.family, f.subfamily, f.postscript_name,
        f.weight, f.width, f.italic, f.is_variable, f.is_color, f.glyph_count, f.license_spdx, f.scripts, fi.container,
        f.vendor, f.designer,
        (SELECT group_concat(name, char(31)) FROM (SELECT t.name FROM face_tags ft JOIN tags t ON t.id = ft.tag_id
            WHERE ft.face_id = f.id ORDER BY t.name COLLATE NOCASE)) AS tags,
        a.scope AS activation
        FROM faces f JOIN files fi ON fi.id = f.file_id LEFT JOIN activations a ON a.face_id = f.id";

    const SUMMARY_ORDER: &'static str =
        " ORDER BY f.family COLLATE NOCASE, f.weight, f.italic, f.width, fi.path, f.face_index";

    fn where_for(filter: &FaceFilter) -> Where {
        let mut w = Where {
            clauses: Vec::new(),
            args: Vec::new(),
        };
        if let Some(q) = filter
            .query
            .as_deref()
            .map(str::trim)
            .filter(|q| !q.is_empty())
        {
            w.clauses
                .push("f.id IN (SELECT rowid FROM faces_fts WHERE faces_fts MATCH ?)".into());
            w.args.push(Box::new(fts_query(q)));
        }
        if let Some(fam) = &filter.family {
            w.clauses.push("f.family = ? COLLATE NOCASE".into());
            w.args.push(Box::new(fam.clone()));
        }
        if let Some(v) = filter.variable {
            w.clauses.push("f.is_variable = ?".into());
            w.args.push(Box::new(v));
        }
        if let Some(v) = filter.color {
            w.clauses.push("f.is_color = ?".into());
            w.args.push(Box::new(v));
        }
        if let Some(v) = filter.italic {
            w.clauses.push("f.italic = ?".into());
            w.args.push(Box::new(v));
        }
        if let Some(s) = &filter.script {
            w.clauses.push("f.scripts LIKE ?".into());
            w.args.push(Box::new(format!("%,{},%", s)));
        }
        if let Some(l) = &filter.license {
            w.clauses.push("f.license_spdx LIKE ?".into());
            w.args.push(Box::new(format!("{}%", l)));
        }
        if let Some(f) = filter.freedom {
            w.clauses.push(freedom_clause(f));
        }
        if let Some((lo, hi)) = filter.weight {
            w.clauses.push("f.weight BETWEEN ? AND ?".into());
            w.args.push(Box::new(lo));
            w.args.push(Box::new(hi));
        }
        if let Some((lo, hi)) = filter.width {
            w.clauses.push("f.width BETWEEN ? AND ?".into());
            w.args.push(Box::new(lo));
            w.args.push(Box::new(hi));
        }
        if let Some(v) = &filter.vendor {
            w.clauses.push("f.vendor = ? COLLATE NOCASE".into());
            w.args.push(Box::new(v.clone()));
        }
        if let Some(t) = &filter.tag {
            w.clauses.push(
                "f.id IN (SELECT ft.face_id FROM face_tags ft JOIN tags t ON t.id = ft.tag_id WHERE t.name = ? COLLATE NOCASE)"
                    .into(),
            );
            w.args.push(Box::new(t.clone()));
        }
        if let Some(c) = &filter.collection {
            w.clauses.push(
                "f.id IN (SELECT cf.face_id FROM collection_faces cf JOIN collections c ON c.id = cf.collection_id WHERE c.name = ? COLLATE NOCASE)"
                    .into(),
            );
            w.args.push(Box::new(c.clone()));
        }
        if let Some(active) = filter.active {
            w.clauses.push(if active {
                "a.face_id IS NOT NULL".into()
            } else {
                "a.face_id IS NULL".into()
            });
        }
        if let Some(state) = filter.activation {
            w.clauses.push("a.scope = ?".into());
            w.args.push(Box::new(state.as_str()));
        }
        if let Some(c) = &filter.container {
            w.clauses.push("fi.container = ?".into());
            w.args.push(Box::new(c.to_ascii_lowercase()));
        }
        if let Some(p) = &filter.path_prefix {
            w.clauses.push("fi.path LIKE ? ESCAPE '\\'".into());
            w.args.push(Box::new(like_prefix(p)));
        }
        if let Some(ids) = &filter.ids {
            let list = ids
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(",");
            w.clauses.push(format!("f.id IN ({list})"));
        }
        w
    }

    fn query_summaries(&self, sql: &str, w: &Where) -> Result<Vec<FaceSummary>> {
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(w.params()), Self::row_to_summary)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn list(&self, filter: &FaceFilter) -> Result<Vec<FaceSummary>> {
        let w = Self::where_for(filter);
        let mut sql = format!("{}{}{}", Self::SUMMARY_SELECT, w.sql(), Self::SUMMARY_ORDER);
        if let Some(n) = filter.limit {
            sql.push_str(&format!(" LIMIT {n}"));
        }
        self.query_summaries(&sql, &w)
    }

    /// Summaries for specific ids, in the usual listing order.
    pub fn summaries(&self, ids: &[i64]) -> Result<Vec<FaceSummary>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        self.list(&FaceFilter {
            ids: Some(ids.to_vec()),
            ..Default::default()
        })
    }

    /// Face ids stored for a file path, in face order.
    pub fn ids_for_path(&self, path: &str) -> Result<Vec<i64>> {
        let mut stmt = self.conn.prepare(
            "SELECT f.id FROM faces f JOIN files fi ON fi.id = f.file_id WHERE fi.path = ?1 ORDER BY f.face_index",
        )?;
        let rows = stmt.query_map(params![path], |r| r.get::<_, i64>(0))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Ids of every face in the same file as `face_id` (itself included).
    pub fn file_faces(&self, face_id: i64) -> Result<Vec<i64>> {
        let mut stmt = self.conn.prepare(
            "SELECT id FROM faces WHERE file_id = (SELECT file_id FROM faces WHERE id = ?1) ORDER BY face_index",
        )?;
        let rows = stmt.query_map(params![face_id], |r| r.get::<_, i64>(0))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Faces whose cmap covers every character in `text` (whitespace and controls
    /// ignored). At most 500 distinct characters.
    pub fn covering(&self, text: &str, filter: &FaceFilter) -> Result<Vec<FaceSummary>> {
        let mut cps: Vec<u32> = text
            .chars()
            .filter(|c| !c.is_whitespace() && !c.is_control())
            .map(|c| c as u32)
            .collect();
        cps.sort_unstable();
        cps.dedup();
        if cps.is_empty() {
            return Ok(Vec::new());
        }
        if cps.len() > 500 {
            return Err(crate::Error::Other(
                "text has more than 500 distinct characters".into(),
            ));
        }
        let mut w = Self::where_for(filter);
        for cp in &cps {
            w.clauses.push(
                "EXISTS (SELECT 1 FROM face_ranges r WHERE r.face_id = f.id AND r.lo <= ? AND r.hi >= ?)"
                    .into(),
            );
            w.args.push(Box::new(*cp as i64));
            w.args.push(Box::new(*cp as i64));
        }
        let mut sql = format!("{}{}{}", Self::SUMMARY_SELECT, w.sql(), Self::SUMMARY_ORDER);
        if let Some(n) = filter.limit {
            sql.push_str(&format!(" LIMIT {n}"));
        }
        self.query_summaries(&sql, &w)
    }

    pub fn get_face(&self, id: i64) -> Result<Option<FaceMetadata>> {
        let json: Option<String> = self
            .conn
            .query_row(
                "SELECT metadata FROM faces WHERE id = ?1",
                params![id],
                |r| r.get(0),
            )
            .optional()?;
        Ok(json.map(|j| serde_json::from_str(&j)).transpose()?)
    }

    pub fn faces_for_path(&self, path: &str) -> Result<Vec<FaceMetadata>> {
        let mut stmt = self.conn.prepare(
            "SELECT f.metadata FROM faces f JOIN files fi ON fi.id = f.file_id WHERE fi.path = ?1 ORDER BY f.face_index",
        )?;
        let rows = stmt.query_map(params![path], |r| r.get::<_, String>(0))?;
        let mut out = Vec::new();
        for j in rows {
            out.push(serde_json::from_str(&j?)?);
        }
        Ok(out)
    }

    /// Faces that share an identity hash (same outlines and names across containers) or a
    /// PostScript name (installing both would conflict).
    pub fn duplicates(&self) -> Result<Vec<DuplicateGroup>> {
        let mut groups = Vec::new();
        for (reason, column) in [
            ("identical outlines and names", "identity_hash"),
            ("same PostScript name", "postscript_name"),
        ] {
            let sql = format!(
                "{} WHERE f.{column} IN (SELECT {column} FROM faces WHERE {column} IS NOT NULL AND {column} != '' GROUP BY {column} HAVING COUNT(*) > 1)
                 ORDER BY f.{column}, fi.path, f.face_index",
                Self::SUMMARY_SELECT.replace("SELECT f.id,", &format!("SELECT f.{column} AS grp, f.id,"))
            );
            let mut stmt = self.conn.prepare(&sql)?;
            let rows = stmt.query_map([], |r| {
                Ok((r.get::<_, String>("grp")?, Self::row_to_summary(r)?))
            })?;
            let mut current: Option<DuplicateGroup> = None;
            for row in rows {
                let (key, face) = row?;
                match current.as_mut() {
                    Some(g) if g.key == key => g.faces.push(face),
                    _ => {
                        if let Some(g) = current.take() {
                            groups.push(g);
                        }
                        current = Some(DuplicateGroup {
                            reason: reason.into(),
                            key,
                            faces: vec![face],
                        });
                    }
                }
            }
            if let Some(g) = current.take() {
                groups.push(g);
            }
        }
        // A PostScript-name group that is exactly an identity group adds nothing.
        let identity: Vec<Vec<i64>> = groups
            .iter()
            .filter(|g| g.reason.starts_with("identical"))
            .map(|g| g.faces.iter().map(|f| f.id).collect())
            .collect();
        groups.retain(|g| {
            if g.reason.starts_with("same") {
                let ids: Vec<i64> = g.faces.iter().map(|f| f.id).collect();
                !identity.contains(&ids)
            } else {
                true
            }
        });
        Ok(groups)
    }

    pub fn stats(&self) -> Result<Stats> {
        let q = |sql: &str| -> rusqlite::Result<i64> { self.conn.query_row(sql, [], |r| r.get(0)) };
        Ok(Stats {
            files: q("SELECT COUNT(*) FROM files WHERE error IS NULL")?,
            faces: q("SELECT COUNT(*) FROM faces")?,
            families: q("SELECT COUNT(DISTINCT family COLLATE NOCASE) FROM faces")?,
            variable_faces: q("SELECT COUNT(*) FROM faces WHERE is_variable")?,
            color_faces: q("SELECT COUNT(*) FROM faces WHERE is_color")?,
            failed_files: q("SELECT COUNT(*) FROM files WHERE error IS NOT NULL")?,
            tags: q("SELECT COUNT(*) FROM tags")?,
            collections: q("SELECT COUNT(*) FROM collections")?,
            sources: q("SELECT COUNT(*) FROM sources")?,
            activations: q("SELECT COUNT(*) FROM activations")?,
            db_path: self.path(),
        })
    }

    pub fn failures(&self) -> Result<Vec<(String, String)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT path, error FROM files WHERE error IS NOT NULL ORDER BY path")?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }
}

pub(crate) fn insert_ranges(tx: &Transaction, face_id: i64, ranges: &[[u32; 2]]) -> Result<()> {
    let mut stmt =
        tx.prepare_cached("INSERT INTO face_ranges (face_id, lo, hi) VALUES (?1, ?2, ?3)")?;
    for [lo, hi] in ranges {
        stmt.execute(params![face_id, *lo as i64, *hi as i64])?;
    }
    Ok(())
}

/// `LIKE` pattern (with `ESCAPE '\\'`) matching anything below `root`, wildcards and
/// backslashes escaped. The separator is appended before escaping so a Windows `\`
/// does not swallow the `%`.
/// The `WHERE` fragment selecting faces whose license falls in `want`.
///
/// The freedom of a face is derived from its SPDX identifier rather than stored, so the
/// clause is built from `freedom::FREE` and `freedom::NONFREE` on every query and cannot
/// go stale when those tables change. `license_spdx` only ever holds a single identifier,
/// since `license::spdx_from_names` is what writes it; SPDX expressions reach
/// `freedom::classify` through externally supplied metadata, not through the index.
fn freedom_clause(want: Freedom) -> String {
    let unstated = "(f.license_spdx IS NULL OR trim(f.license_spdx) = '')";
    let free = freedom::sql_in("f.license_spdx", freedom::FREE);
    let nonfree = freedom::sql_in("f.license_spdx", freedom::NONFREE);
    match want {
        Freedom::Unstated => unstated.to_string(),
        Freedom::Free => format!("(NOT {unstated} AND {free})"),
        Freedom::Nonfree => format!("(NOT {unstated} AND {nonfree})"),
        Freedom::Unknown => format!("(NOT {unstated} AND NOT {free} AND NOT {nonfree})"),
    }
}

fn like_prefix(root: &str) -> String {
    let mut prefix = root.to_string();
    if !(prefix.ends_with(std::path::MAIN_SEPARATOR) || prefix.ends_with('/')) {
        prefix.push(std::path::MAIN_SEPARATOR);
    }
    let escaped = prefix
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    format!("{escaped}%")
}

/// Turn free text into an FTS5 prefix query: each term quoted and suffixed with `*`.
fn fts_query(q: &str) -> String {
    q.split_whitespace()
        .map(|t| format!("\"{}\"*", t.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" ")
}
