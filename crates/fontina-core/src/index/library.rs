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

//! Tags, collections, sources and activation state: the parts of the index that hold
//! what the user decided rather than what the font files say. Everything here survives a
//! rescan of the same file (see `carry_over_*`) and exports to JSON with a schema.

use super::{FaceSummary, Index, Where};
use crate::error::{Error, Result};
use rusqlite::{OptionalExtension, Transaction, params};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// How a face was made available to the OS through fontina.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ActivationState {
    /// Registered until logout or reboot.
    Session,
    /// Registered persistently for the current user, in place.
    User,
    /// Copied (or linked) into the per-user font directory.
    Installed,
}

impl ActivationState {
    pub fn as_str(self) -> &'static str {
        match self {
            ActivationState::Session => "session",
            ActivationState::User => "user",
            ActivationState::Installed => "installed",
        }
    }
}

impl std::str::FromStr for ActivationState {
    type Err = ();
    fn from_str(s: &str) -> std::result::Result<Self, ()> {
        Ok(match s {
            "session" => ActivationState::Session,
            "user" => ActivationState::User,
            "installed" => ActivationState::Installed,
            _ => return Err(()),
        })
    }
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct TagInfo {
    pub name: String,
    pub faces: i64,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct CollectionInfo {
    pub id: i64,
    pub name: String,
    pub faces: i64,
    /// Seconds since the Unix epoch.
    pub created_at: i64,
}

/// Why a directory is in the index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum SourceKind {
    /// Added by the user (`scan <dir>`, `source add`).
    User,
    /// An operating-system font directory (`scan --system`).
    System,
}

impl SourceKind {
    fn as_str(self) -> &'static str {
        match self {
            SourceKind::User => "user",
            SourceKind::System => "system",
        }
    }
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Source {
    pub id: i64,
    pub path: String,
    /// Followed by `fontina watch`.
    pub watch: bool,
    pub kind: SourceKind,
    pub added_at: i64,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ActivationRecord {
    pub face: FaceSummary,
    pub state: ActivationState,
    pub activated_at: i64,
    /// Where `install` put the copy, for `uninstall`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub installed_path: Option<String>,
}

/// A collection as written by `collection export`; `schemas/collection.json`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CollectionExport {
    pub schema_version: u32,
    pub name: String,
    /// RFC 3339.
    pub exported_at: String,
    /// True when every `CollectionFace::path` is relative to the directory holding this
    /// file, which is how a bundle is written. Absent means absolute, as every export
    /// before bundles was.
    ///
    /// The flag is additive and `SCHEMA_VERSION` does not move for it. An older fontina
    /// reading a bundle ignores it and treats the relative paths as absolute, which
    /// costs nothing in practice: the path is the *last* thing `match_collection_face`
    /// tries, after the identity hash and the PostScript name, and those do not depend
    /// on where the file lives.
    #[serde(default, skip_serializing_if = "is_false")]
    pub relative_paths: bool,
    pub faces: Vec<CollectionFace>,
}

fn is_false(b: &bool) -> bool {
    !*b
}

impl CollectionExport {
    /// Rewrite the paths relative to `base`, for an export that travels with its fonts.
    ///
    /// An absolute path names a directory that exists on one machine, so it is dead
    /// weight in a file meant to be shared — and it carries a home directory into
    /// whatever the collection is shared through.
    ///
    /// Fails rather than half-succeeding. `base` has to exist to be canonicalised, and
    /// scanned paths are canonical, so a base carrying `..` or a symlink would match
    /// nothing; and every face has to end up under it, because a file that says its
    /// paths are relative while some are absolute is worse than one that says nothing —
    /// a reader joining the base onto an absolute path gets nonsense.
    pub fn relative_to(&mut self, base: &Path) -> Result<()> {
        let base = base
            .canonicalize()
            .map_err(|e| Error::Io(base.to_path_buf(), e))?;
        let mut rewritten = Vec::with_capacity(self.faces.len());
        for f in &self.faces {
            let rel = Path::new(&f.path).strip_prefix(&base).map_err(|_| {
                Error::Other(format!(
                    "{} is outside {}, so the collection cannot travel with its fonts",
                    f.path,
                    base.display()
                ))
            })?;
            // `/` on the wire whatever wrote it. A bundle is made to be carried to
            // another machine, and a Windows separator resolves to one filename with a
            // backslash in it there; `/` is read correctly by every platform.
            rewritten.push(
                rel.components()
                    .map(|c| c.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/"),
            );
        }
        for (f, rel) in self.faces.iter_mut().zip(rewritten) {
            f.path = rel;
        }
        self.relative_paths = true;
        Ok(())
    }

    /// Put the paths back together against the directory this file came from.
    ///
    /// A no-op for an export that was never made relative, so a caller can apply it to
    /// anything it reads without asking which kind it has.
    ///
    /// Entries that climb out of the bundle are left alone. A collection file is written
    /// by someone else, `../../../etc/hosts` is a path a hostile one can contain, and
    /// nothing downstream should be handed it as though the bundle vouched for it.
    pub fn resolve_paths(&mut self, base: &Path) -> usize {
        if !self.relative_paths {
            return 0;
        }
        // The index stores canonical absolute paths, so a relative base would resolve to
        // something that can never match one.
        let base = base.canonicalize().unwrap_or_else(|_| base.to_path_buf());
        let mut escaped = 0;
        for f in &mut self.faces {
            let p = Path::new(&f.path);
            if !p.is_relative() {
                continue;
            }
            if p.components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
            {
                escaped += 1;
                continue;
            }
            f.path = base.join(p).to_string_lossy().into_owned();
        }
        self.relative_paths = false;
        escaped
    }

    /// Write this collection as a bundle in `dir`: the JSON beside a copy of every font
    /// it names, with every path relative to `dir`.
    ///
    /// This is the shareable form. A collection file on its own is a list of names and
    /// hashes, useful only to somebody who already has the fonts; a bundle is the whole
    /// thing, and it opens the same on any machine because nothing in it is absolute.
    ///
    /// `self` is left alone. The rewritten paths belong to the copy on disk, not to the
    /// export the caller is holding.
    pub fn write_bundle(&self, dir: &Path) -> Result<BundleReport> {
        if dir.join(BUNDLE_FILE).exists() {
            return Err(Error::Other(format!(
                "{} already holds a bundle; write to a new directory rather than mixing \
                 two collections' fonts",
                dir.display()
            )));
        }
        let fonts = dir.join(BUNDLE_FONTS);
        std::fs::create_dir_all(&fonts).map_err(|e| Error::Io(fonts, e))?;
        // `relative_to` strips a canonical base, and the caller's `dir` need not be one —
        // `.`, a trailing `..`, or /tmp on macOS all fail to match otherwise.
        let dir = dir
            .canonicalize()
            .map_err(|e| Error::Io(dir.to_path_buf(), e))?;
        let fonts = dir.join(BUNDLE_FONTS);

        let mut bundled = self.clone();
        // One file holds several faces whenever the collection has a TrueType collection
        // or two instances of the same variable font in it, so copy per source path and
        // let those faces point at the single copy.
        let mut copied: HashMap<String, std::path::PathBuf> = HashMap::new();
        let mut taken: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut bytes = 0u64;
        for f in &mut bundled.faces {
            if let Some(dest) = copied.get(&f.path) {
                f.path = dest.to_string_lossy().into_owned();
                continue;
            }
            let source = std::mem::take(&mut f.path);
            let src = Path::new(&source);
            let name = src
                .file_name()
                .ok_or_else(|| Error::Other(format!("{} does not name a file", src.display())))?;
            // Two families both shipping Regular.ttf are one collision away from the
            // bundle holding one of them twice under the other's name.
            let dest = fonts.join(unique_name(&name.to_string_lossy(), &mut taken));
            bytes += std::fs::copy(src, &dest).map_err(|e| Error::Io(src.to_path_buf(), e))?;
            f.path = dest.to_string_lossy().into_owned();
            copied.insert(source, dest);
        }
        bundled.relative_to(&dir)?;

        let json = serde_json::to_string_pretty(&bundled)
            .map_err(|e| Error::Other(format!("serialising the collection: {e}")))?;
        let marker = dir.join(BUNDLE_FILE);
        std::fs::write(&marker, format!("{json}\n")).map_err(|e| Error::Io(marker, e))?;

        Ok(BundleReport {
            dir: dir.to_string_lossy().into_owned(),
            faces: bundled.faces.len(),
            files: copied.len(),
            bytes,
        })
    }
}

/// The JSON file at the root of a bundle.
pub const BUNDLE_FILE: &str = "collection.json";
/// The directory inside a bundle that holds the fonts themselves.
pub const BUNDLE_FONTS: &str = "fonts";

/// What writing a bundle did.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct BundleReport {
    pub dir: String,
    pub faces: usize,
    /// Font files copied. Fewer than `faces` when one file holds several of them.
    pub files: usize,
    pub bytes: u64,
}

/// `name`, or `name-2`, `name-3`, ... until it is one nothing else in the bundle has.
fn unique_name(name: &str, taken: &mut std::collections::HashSet<String>) -> String {
    let (stem, ext) = match name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => (stem, format!(".{ext}")),
        _ => (name, String::new()),
    };
    let mut candidate = name.to_string();
    let mut n = 1;
    while !taken.insert(candidate.clone()) {
        n += 1;
        candidate = format!("{stem}-{n}{ext}");
    }
    candidate
}

/// One face in an exported collection. On import, faces are matched by identity hash,
/// then PostScript name, then path and index, so a collection survives re-encoding the
/// font (TTF to WOFF2) or moving it.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CollectionFace {
    pub family: String,
    pub subfamily: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub postscript_name: Option<String>,
    pub identity_hash: String,
    pub blake3: String,
    pub path: String,
    #[serde(default)]
    pub index: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ImportReport {
    pub collection: String,
    pub matched: usize,
    /// Faces in the file that are not in this index.
    pub missing: Vec<CollectionFace>,
    pub tags_applied: usize,
}

/// What `fontina tag sync` did, or would do.
///
/// The counts are per *file*, not per face: a tag lives on a file, and a TrueType
/// collection holds several faces in one.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct TagSyncReport {
    /// `to-files` or `from-files`.
    pub direction: String,
    /// Files considered.
    pub files: usize,
    /// Files whose tags differed. With `dry_run`, files that would have been changed.
    pub changed: usize,
    /// Nothing was written.
    pub dry_run: bool,
    /// One entry per file that differed.
    pub changes: Vec<TagSyncChange>,
    /// Whatever could not be carried across, and why. Never fatal: one font in a system
    /// directory, or on a filesystem without extended attributes, should not stop the
    /// other three hundred.
    pub skipped: Vec<TagSyncSkip>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct TagSyncChange {
    pub path: String,
    /// Tags the destination gained.
    pub added: Vec<String>,
    /// Tags the destination lost. Sync mirrors rather than merges: two tag sets with no
    /// common ancestor cannot tell a deletion from an addition, so the direction says
    /// which side is right and the other is made to match it.
    pub removed: Vec<String>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct TagSyncSkip {
    pub path: String,
    pub reason: String,
}

/// A face that would clash with the one being activated.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Conflict {
    pub face: FaceSummary,
    pub reason: String,
}

fn valid_name(kind: &str, name: &str) -> Result<String> {
    let name = name.trim();
    if name.is_empty() || name.chars().any(|c| c.is_control()) {
        return Err(Error::Other(format!(
            "{kind} name must be non-empty printable text"
        )));
    }
    Ok(name.to_string())
}

impl Index {
    // ----- tags -----

    pub fn tags(&self) -> Result<Vec<TagInfo>> {
        let mut stmt = self.conn.prepare(
            "SELECT t.name, COUNT(ft.face_id) FROM tags t LEFT JOIN face_tags ft ON ft.tag_id = t.id
             GROUP BY t.id ORDER BY t.name COLLATE NOCASE",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(TagInfo {
                name: r.get(0)?,
                faces: r.get(1)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Tag faces. Creates the tag when new. Returns how many faces were newly tagged.
    pub fn tag(&mut self, ids: &[i64], name: &str) -> Result<usize> {
        let name = valid_name("tag", name)?;
        let tx = self.begin()?;
        tx.execute(
            "INSERT OR IGNORE INTO tags (name) VALUES (?1)",
            params![name],
        )?;
        let tag_id: i64 = tx.query_row(
            "SELECT id FROM tags WHERE name = ?1 COLLATE NOCASE",
            params![name],
            |r| r.get(0),
        )?;
        let mut n = 0;
        for id in ids {
            n += tx.execute(
                "INSERT OR IGNORE INTO face_tags (face_id, tag_id) VALUES (?1, ?2)",
                params![id, tag_id],
            )?;
        }
        tx.commit()?;
        Ok(n)
    }

    /// Remove a tag from faces. Returns how many faces lost it.
    pub fn untag(&mut self, ids: &[i64], name: &str) -> Result<usize> {
        let tx = self.begin()?;
        let mut n = 0;
        for id in ids {
            n += tx.execute(
                "DELETE FROM face_tags WHERE face_id = ?1 AND tag_id = (SELECT id FROM tags WHERE name = ?2 COLLATE NOCASE)",
                params![id, name.trim()],
            )?;
        }
        tx.commit()?;
        Ok(n)
    }

    pub fn rename_tag(&mut self, old: &str, new: &str) -> Result<bool> {
        let new = valid_name("tag", new)?;
        Ok(self.conn.execute(
            "UPDATE tags SET name = ?2 WHERE name = ?1 COLLATE NOCASE",
            params![old.trim(), new],
        )? > 0)
    }

    pub fn delete_tag(&mut self, name: &str) -> Result<bool> {
        Ok(self.conn.execute(
            "DELETE FROM tags WHERE name = ?1 COLLATE NOCASE",
            params![name.trim()],
        )? > 0)
    }

    // ----- collections -----

    pub fn collections(&self) -> Result<Vec<CollectionInfo>> {
        let mut stmt = self.conn.prepare(
            "SELECT c.id, c.name, COUNT(cf.face_id), c.created_at FROM collections c
             LEFT JOIN collection_faces cf ON cf.collection_id = c.id
             GROUP BY c.id ORDER BY c.name COLLATE NOCASE",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(CollectionInfo {
                id: r.get(0)?,
                name: r.get(1)?,
                faces: r.get(2)?,
                created_at: r.get(3)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    fn collection_id(&self, name: &str) -> Result<Option<i64>> {
        Ok(self
            .conn
            .query_row(
                "SELECT id FROM collections WHERE name = ?1 COLLATE NOCASE",
                params![name.trim()],
                |r| r.get(0),
            )
            .optional()?)
    }

    /// Create a collection; returns its id, or the existing id when the name is taken.
    pub fn create_collection(&mut self, name: &str) -> Result<i64> {
        let name = valid_name("collection", name)?;
        if let Some(id) = self.collection_id(&name)? {
            return Ok(id);
        }
        self.conn
            .execute("INSERT INTO collections (name) VALUES (?1)", params![name])?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn rename_collection(&mut self, old: &str, new: &str) -> Result<bool> {
        let new = valid_name("collection", new)?;
        Ok(self.conn.execute(
            "UPDATE collections SET name = ?2 WHERE name = ?1 COLLATE NOCASE",
            params![old.trim(), new],
        )? > 0)
    }

    pub fn delete_collection(&mut self, name: &str) -> Result<bool> {
        Ok(self.conn.execute(
            "DELETE FROM collections WHERE name = ?1 COLLATE NOCASE",
            params![name.trim()],
        )? > 0)
    }

    /// Append faces to a collection (created when missing). Returns how many were new.
    pub fn add_to_collection(&mut self, name: &str, ids: &[i64]) -> Result<usize> {
        let cid = self.create_collection(name)?;
        let tx = self.begin()?;
        let mut pos: i64 = tx.query_row(
            "SELECT COALESCE(MAX(position), -1) + 1 FROM collection_faces WHERE collection_id = ?1",
            params![cid],
            |r| r.get(0),
        )?;
        let mut n = 0;
        for id in ids {
            let added = tx.execute(
                "INSERT OR IGNORE INTO collection_faces (collection_id, face_id, position) VALUES (?1, ?2, ?3)",
                params![cid, id, pos],
            )?;
            if added > 0 {
                pos += 1;
                n += 1;
            }
        }
        tx.commit()?;
        Ok(n)
    }

    pub fn remove_from_collection(&mut self, name: &str, ids: &[i64]) -> Result<usize> {
        let Some(cid) = self.collection_id(name)? else {
            return Err(Error::Other(format!("no collection named {name:?}")));
        };
        let tx = self.begin()?;
        let mut n = 0;
        for id in ids {
            n += tx.execute(
                "DELETE FROM collection_faces WHERE collection_id = ?1 AND face_id = ?2",
                params![cid, id],
            )?;
        }
        tx.commit()?;
        Ok(n)
    }

    /// Faces of a collection in their stored order.
    pub fn collection_faces(&self, name: &str) -> Result<Vec<FaceSummary>> {
        let Some(cid) = self.collection_id(name)? else {
            return Err(Error::Other(format!("no collection named {name:?}")));
        };
        let sql = format!(
            "{} JOIN collection_faces cf ON cf.face_id = f.id WHERE cf.collection_id = ? ORDER BY cf.position, f.face_index",
            Self::SUMMARY_SELECT
        );
        let w = Where {
            clauses: Vec::new(),
            args: vec![Box::new(cid)],
        };
        self.query_summaries(&sql, &w)
    }

    pub fn export_collection(&self, name: &str) -> Result<CollectionExport> {
        let faces = self.collection_faces(name)?;
        let stored_name: String = self.conn.query_row(
            "SELECT name FROM collections WHERE name = ?1 COLLATE NOCASE",
            params![name.trim()],
            |r| r.get(0),
        )?;
        let mut out = Vec::with_capacity(faces.len());
        for f in faces {
            let (identity_hash, blake3): (String, String) = self.conn.query_row(
                "SELECT f.identity_hash, fi.blake3 FROM faces f JOIN files fi ON fi.id = f.file_id WHERE f.id = ?1",
                params![f.id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            out.push(CollectionFace {
                family: f.family,
                subfamily: f.subfamily,
                postscript_name: f.postscript_name,
                identity_hash,
                blake3,
                path: f.path,
                index: f.index,
                tags: f.tags,
            });
        }
        Ok(CollectionExport {
            schema_version: crate::SCHEMA_VERSION,
            name: stored_name,
            relative_paths: false,
            exported_at: time::OffsetDateTime::now_utc()
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_default(),
            faces: out,
        })
    }

    /// Find the indexed face an exported entry refers to.
    fn match_collection_face(&self, cf: &CollectionFace) -> Result<Option<i64>> {
        let by_hash: Option<i64> = self
            .conn
            .query_row(
                "SELECT id FROM faces WHERE identity_hash = ?1 ORDER BY id LIMIT 1",
                params![cf.identity_hash],
                |r| r.get(0),
            )
            .optional()?;
        if by_hash.is_some() {
            return Ok(by_hash);
        }
        if let Some(ps) = &cf.postscript_name {
            let by_ps: Option<i64> = self
                .conn
                .query_row(
                    "SELECT id FROM faces WHERE postscript_name = ?1 ORDER BY id LIMIT 1",
                    params![ps],
                    |r| r.get(0),
                )
                .optional()?;
            if by_ps.is_some() {
                return Ok(by_ps);
            }
        }
        Ok(self
            .conn
            .query_row(
                "SELECT f.id FROM faces f JOIN files fi ON fi.id = f.file_id WHERE fi.path = ?1 AND f.face_index = ?2",
                params![cf.path, cf.index],
                |r| r.get(0),
            )
            .optional()?)
    }

    /// Import an exported collection, merging into an existing collection of the same
    /// (or the given) name. Tags travel with the faces when `apply_tags` is set.
    pub fn import_collection(
        &mut self,
        export: &CollectionExport,
        name: Option<&str>,
        apply_tags: bool,
    ) -> Result<ImportReport> {
        if export.schema_version > crate::SCHEMA_VERSION {
            return Err(Error::Other(format!(
                "collection was written by a newer fontina (schema {} > {})",
                export.schema_version,
                crate::SCHEMA_VERSION
            )));
        }
        let name = valid_name("collection", name.unwrap_or(&export.name))?;
        let mut ids = Vec::new();
        let mut missing = Vec::new();
        let mut tag_ops: Vec<(i64, String)> = Vec::new();
        for cf in &export.faces {
            match self.match_collection_face(cf)? {
                Some(id) => {
                    ids.push(id);
                    if apply_tags {
                        tag_ops.extend(cf.tags.iter().map(|t| (id, t.clone())));
                    }
                }
                None => missing.push(cf.clone()),
            }
        }
        self.add_to_collection(&name, &ids)?;
        let mut tags_applied = 0;
        for (id, tag) in tag_ops {
            tags_applied += self.tag(&[id], &tag)?;
        }
        Ok(ImportReport {
            collection: name,
            matched: ids.len(),
            missing,
            tags_applied,
        })
    }

    // ----- sources -----

    pub fn sources(&self) -> Result<Vec<Source>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, path, watch, kind, added_at FROM sources ORDER BY path")?;
        let rows = stmt.query_map([], |r| {
            let kind: String = r.get(3)?;
            Ok(Source {
                id: r.get(0)?,
                path: r.get(1)?,
                watch: r.get(2)?,
                kind: if kind == "system" {
                    SourceKind::System
                } else {
                    SourceKind::User
                },
                added_at: r.get(4)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Register (or update) a directory as a source. `path` should be canonical.
    pub fn add_source(&mut self, path: &str, watch: bool, kind: SourceKind) -> Result<Source> {
        self.conn.execute(
            "INSERT INTO sources (path, watch, kind) VALUES (?1, ?2, ?3)
             ON CONFLICT(path) DO UPDATE SET watch = excluded.watch, kind = excluded.kind",
            params![path, watch, kind.as_str()],
        )?;
        self.sources()?
            .into_iter()
            .find(|s| s.path == path)
            .ok_or_else(|| Error::Other("source vanished".into()))
    }

    /// Record a scanned directory without changing the watch flag of a known source.
    pub(crate) fn touch_source(&mut self, path: &str, kind: SourceKind) -> Result<()> {
        self.conn.execute(
            "INSERT INTO sources (path, watch, kind) VALUES (?1, ?2, ?3) ON CONFLICT(path) DO NOTHING",
            params![path, kind == SourceKind::User, kind.as_str()],
        )?;
        Ok(())
    }

    pub fn set_source_watch(&mut self, path: &str, watch: bool) -> Result<bool> {
        Ok(self.conn.execute(
            "UPDATE sources SET watch = ?2 WHERE path = ?1",
            params![path, watch],
        )? > 0)
    }

    /// Forget a source. With `remove_faces`, its indexed files go too.
    pub fn remove_source(&mut self, path: &str, remove_faces: bool) -> Result<bool> {
        let removed = self
            .conn
            .execute("DELETE FROM sources WHERE path = ?1", params![path])?
            > 0;
        if removed && remove_faces {
            self.remove_under(path)?;
        }
        Ok(removed)
    }

    // ----- activations -----

    pub fn set_activation(
        &mut self,
        ids: &[i64],
        state: ActivationState,
        installed_path: Option<&str>,
    ) -> Result<()> {
        let tx = self.begin()?;
        for id in ids {
            tx.execute(
                "INSERT INTO activations (face_id, scope, activated_at, installed_path) VALUES (?1, ?2, unixepoch(), ?3)
                 ON CONFLICT(face_id) DO UPDATE SET scope = excluded.scope, activated_at = excluded.activated_at,
                 installed_path = excluded.installed_path",
                params![id, state.as_str(), installed_path],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn clear_activation(&mut self, ids: &[i64]) -> Result<usize> {
        let tx = self.begin()?;
        let mut n = 0;
        for id in ids {
            n += tx.execute("DELETE FROM activations WHERE face_id = ?1", params![id])?;
        }
        tx.commit()?;
        Ok(n)
    }

    pub fn activation(&self, face_id: i64) -> Result<Option<ActivationRecord>> {
        Ok(self
            .activations_where("a.face_id = ?", vec![Box::new(face_id)])?
            .pop())
    }

    /// Every activation fontina has recorded, in listing order.
    pub fn activations(&self) -> Result<Vec<ActivationRecord>> {
        self.activations_where("a.face_id IS NOT NULL", Vec::new())
    }

    fn activations_where(
        &self,
        clause: &str,
        args: Vec<Box<dyn rusqlite::ToSql>>,
    ) -> Result<Vec<ActivationRecord>> {
        let sql = format!(
            "{} WHERE {clause}{}",
            Self::SUMMARY_SELECT.replace(
                "a.scope AS activation",
                "a.scope AS activation, a.activated_at, a.installed_path"
            ),
            Self::SUMMARY_ORDER
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(
            rusqlite::params_from_iter(args.iter().map(|a| a.as_ref())),
            |r| {
                let face = Self::row_to_summary(r)?;
                Ok((
                    face,
                    r.get::<_, Option<i64>>("activated_at")?,
                    r.get::<_, Option<String>>("installed_path")?,
                ))
            },
        )?;
        let mut out = Vec::new();
        for row in rows {
            let (face, at, installed_path) = row?;
            if let (Some(state), Some(activated_at)) = (face.activation, at) {
                out.push(ActivationRecord {
                    face,
                    state,
                    activated_at,
                    installed_path,
                });
            }
        }
        Ok(out)
    }

    // ----- conflicts -----

    /// Faces from other files that would clash with `face_id` once it is active: same
    /// PostScript name, or same family and style, and already active through fontina or
    /// present under one of `system_roots` (the OS font directories).
    pub fn conflicts(&self, face_id: i64, system_roots: &[String]) -> Result<Vec<Conflict>> {
        let Some(me) = self.summaries(&[face_id])?.pop() else {
            return Err(Error::Other(format!("no face with id {face_id}")));
        };
        let mut w = Where {
            clauses: vec![
                "f.file_id != (SELECT file_id FROM faces WHERE id = ?)".into(),
                "((f.postscript_name IS NOT NULL AND f.postscript_name = ?) OR (f.family = ? COLLATE NOCASE AND f.subfamily = ? COLLATE NOCASE))".into(),
            ],
            args: vec![
                Box::new(face_id),
                Box::new(me.postscript_name.clone().unwrap_or_default()),
                Box::new(me.family.clone()),
                Box::new(me.subfamily.clone()),
            ],
        };
        let mut presence = vec!["a.face_id IS NOT NULL".to_string()];
        for root in system_roots {
            presence.push("fi.path LIKE ? ESCAPE '\\'".into());
            w.args.push(Box::new(super::like_prefix(root)));
        }
        w.clauses.push(format!("({})", presence.join(" OR ")));
        let sql = format!("{}{}{}", Self::SUMMARY_SELECT, w.sql(), Self::SUMMARY_ORDER);
        let faces = self.query_summaries(&sql, &w)?;
        Ok(faces
            .into_iter()
            .map(|face| {
                let what = if face.postscript_name.is_some()
                    && face.postscript_name == me.postscript_name
                {
                    "same PostScript name"
                } else {
                    "same family and style"
                };
                let where_ = match face.activation {
                    Some(s) => format!("active ({})", s.as_str()),
                    None => "present in a system font directory".to_string(),
                };
                Conflict {
                    face,
                    reason: format!("{what}, {where_}"),
                }
            })
            .collect())
    }
}

// ----- carry-over across rescans -----

#[derive(Default)]
pub(super) struct CarriedFace {
    tags: Vec<i64>,
    collections: Vec<(i64, i64)>,
    activation: Option<(String, i64, Option<String>)>,
}

pub(super) type Carried = HashMap<u32, CarriedFace>;

/// Snapshot the user data attached to a file's faces before they are deleted.
pub(super) fn carry_over_take(tx: &Transaction, path: &str) -> Result<Carried> {
    let mut out: Carried = HashMap::new();
    let mut stmt = tx.prepare_cached(
        "SELECT f.face_index, ft.tag_id FROM faces f JOIN files fi ON fi.id = f.file_id
         JOIN face_tags ft ON ft.face_id = f.id WHERE fi.path = ?1",
    )?;
    for row in stmt.query_map(params![path], |r| {
        Ok((r.get::<_, i64>(0)? as u32, r.get(1)?))
    })? {
        let (idx, tag): (u32, i64) = row?;
        out.entry(idx).or_default().tags.push(tag);
    }
    let mut stmt = tx.prepare_cached(
        "SELECT f.face_index, cf.collection_id, cf.position FROM faces f JOIN files fi ON fi.id = f.file_id
         JOIN collection_faces cf ON cf.face_id = f.id WHERE fi.path = ?1",
    )?;
    for row in stmt.query_map(params![path], |r| {
        Ok((r.get::<_, i64>(0)? as u32, r.get(1)?, r.get(2)?))
    })? {
        let (idx, cid, pos): (u32, i64, i64) = row?;
        out.entry(idx).or_default().collections.push((cid, pos));
    }
    let mut stmt = tx.prepare_cached(
        "SELECT f.face_index, a.scope, a.activated_at, a.installed_path FROM faces f JOIN files fi ON fi.id = f.file_id
         JOIN activations a ON a.face_id = f.id WHERE fi.path = ?1",
    )?;
    for row in stmt.query_map(params![path], |r| {
        Ok((r.get::<_, i64>(0)? as u32, r.get(1)?, r.get(2)?, r.get(3)?))
    })? {
        let (idx, scope, at, installed): (u32, String, i64, Option<String>) = row?;
        out.entry(idx).or_default().activation = Some((scope, at, installed));
    }
    Ok(out)
}

pub(super) fn carry_over_apply(
    tx: &Transaction,
    face_id: i64,
    index: u32,
    carried: &Carried,
) -> Result<()> {
    let Some(c) = carried.get(&index) else {
        return Ok(());
    };
    for tag in &c.tags {
        tx.execute(
            "INSERT OR IGNORE INTO face_tags (face_id, tag_id) VALUES (?1, ?2)",
            params![face_id, tag],
        )?;
    }
    for (cid, pos) in &c.collections {
        tx.execute(
            "INSERT OR IGNORE INTO collection_faces (collection_id, face_id, position) VALUES (?1, ?2, ?3)",
            params![cid, face_id, pos],
        )?;
    }
    if let Some((scope, at, installed)) = &c.activation {
        tx.execute(
            "INSERT OR REPLACE INTO activations (face_id, scope, activated_at, installed_path) VALUES (?1, ?2, ?3, ?4)",
            params![face_id, scope, at, installed],
        )?;
    }
    Ok(())
}
