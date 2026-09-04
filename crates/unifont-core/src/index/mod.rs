//! SQLite index. One file, WAL mode, FTS5 for names. The full `FaceMetadata` JSON is
//! stored per face so `info` round-trips without re-parsing the font.

mod schema;

use crate::FileInfo;
use crate::error::Result;
use crate::model::FaceMetadata;
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub struct Index {
    conn: Connection,
}

/// Compact per-face row used by listings.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub scripts: Vec<String>,
    pub container: String,
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
    pub weight: Option<(u16, u16)>,
    pub path_prefix: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DuplicateGroup {
    pub reason: String,
    pub key: String,
    pub faces: Vec<FaceSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Stats {
    pub files: i64,
    pub faces: i64,
    pub families: i64,
    pub variable_faces: i64,
    pub color_faces: i64,
    pub failed_files: i64,
    pub db_path: String,
}

impl Index {
    /// Default location: the platform data directory for `unifont`.
    pub fn default_path() -> PathBuf {
        directories::ProjectDirs::from("", "", "unifont")
            .map(|d| d.data_dir().join("index.db"))
            .unwrap_or_else(|| PathBuf::from("unifont-index.db"))
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

    pub(crate) fn upsert_file_tx(
        tx: &Transaction,
        file: &FileInfo,
        faces: &[FaceMetadata],
    ) -> Result<()> {
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
            stmt.execute(params![
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
        let prefix = format!(
            "{}{}",
            root.trim_end_matches(std::path::MAIN_SEPARATOR),
            std::path::MAIN_SEPARATOR
        );
        let paths: Vec<String> = {
            let mut stmt = self
                .conn
                .prepare("SELECT path FROM files WHERE path = ?1 OR path LIKE ?2 ESCAPE '\\'")?;
            let like = format!(
                "{}%",
                prefix
                    .replace('\\', "\\\\")
                    .replace('%', "\\%")
                    .replace('_', "\\_")
            );
            let rows = stmt.query_map(params![root, like], |r| r.get::<_, String>(0))?;
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

    fn row_to_summary(r: &rusqlite::Row) -> rusqlite::Result<FaceSummary> {
        let scripts: String = r.get("scripts")?;
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
            license: r.get("license_spdx")?,
            scripts: scripts
                .split(',')
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect(),
            container: r.get("container")?,
        })
    }

    const SUMMARY_SELECT: &'static str = "SELECT f.id, fi.path, f.face_index, f.family, f.subfamily, f.postscript_name,
        f.weight, f.width, f.italic, f.is_variable, f.is_color, f.glyph_count, f.license_spdx, f.scripts, fi.container
        FROM faces f JOIN files fi ON fi.id = f.file_id";

    pub fn list(&self, filter: &FaceFilter) -> Result<Vec<FaceSummary>> {
        let mut sql = String::from(Self::SUMMARY_SELECT);
        let mut clauses: Vec<String> = Vec::new();
        let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(q) = filter
            .query
            .as_deref()
            .map(str::trim)
            .filter(|q| !q.is_empty())
        {
            clauses.push("f.id IN (SELECT rowid FROM faces_fts WHERE faces_fts MATCH ?)".into());
            args.push(Box::new(fts_query(q)));
        }
        if let Some(fam) = &filter.family {
            clauses.push("f.family = ? COLLATE NOCASE".into());
            args.push(Box::new(fam.clone()));
        }
        if let Some(v) = filter.variable {
            clauses.push("f.is_variable = ?".into());
            args.push(Box::new(v));
        }
        if let Some(v) = filter.color {
            clauses.push("f.is_color = ?".into());
            args.push(Box::new(v));
        }
        if let Some(v) = filter.italic {
            clauses.push("f.italic = ?".into());
            args.push(Box::new(v));
        }
        if let Some(s) = &filter.script {
            clauses.push("f.scripts LIKE ?".into());
            args.push(Box::new(format!("%,{},%", s)));
        }
        if let Some(l) = &filter.license {
            clauses.push("f.license_spdx LIKE ?".into());
            args.push(Box::new(format!("{}%", l)));
        }
        if let Some((lo, hi)) = filter.weight {
            clauses.push("f.weight BETWEEN ? AND ?".into());
            args.push(Box::new(lo));
            args.push(Box::new(hi));
        }
        if let Some(p) = &filter.path_prefix {
            clauses.push("fi.path LIKE ?".into());
            args.push(Box::new(format!("{}%", p)));
        }
        if !clauses.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&clauses.join(" AND "));
        }
        sql.push_str(
            " ORDER BY f.family COLLATE NOCASE, f.weight, f.italic, f.width, fi.path, f.face_index",
        );
        if let Some(n) = filter.limit {
            sql.push_str(&format!(" LIMIT {n}"));
        }
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(
            rusqlite::params_from_iter(args.iter().map(|a| a.as_ref())),
            Self::row_to_summary,
        )?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
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

/// Turn free text into an FTS5 prefix query: each term quoted and suffixed with `*`.
fn fts_query(q: &str) -> String {
    q.split_whitespace()
        .map(|t| format!("\"{}\"*", t.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" ")
}
