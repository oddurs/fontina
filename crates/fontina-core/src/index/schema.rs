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

use crate::error::Result;
use rusqlite::Connection;

/// Ordered migrations. Index i applies when `PRAGMA user_version` == i.
const MIGRATIONS: &[&str] = &[
    r#"
CREATE TABLE files (
    id          INTEGER PRIMARY KEY,
    path        TEXT NOT NULL UNIQUE,
    size        INTEGER NOT NULL,
    mtime       INTEGER NOT NULL,
    blake3      TEXT NOT NULL,
    container   TEXT NOT NULL,
    face_count  INTEGER NOT NULL,
    scanned_at  INTEGER NOT NULL,
    error       TEXT
);

CREATE TABLE faces (
    id              INTEGER PRIMARY KEY,
    file_id         INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    face_index      INTEGER NOT NULL,
    postscript_name TEXT,
    family          TEXT NOT NULL,
    subfamily       TEXT NOT NULL,
    full_name       TEXT,
    weight          REAL NOT NULL,
    width           REAL NOT NULL,
    italic          INTEGER NOT NULL,
    is_variable     INTEGER NOT NULL,
    is_color        INTEGER NOT NULL,
    glyph_count     INTEGER NOT NULL,
    license_spdx    TEXT,
    vendor          TEXT,
    version         TEXT,
    designer        TEXT,
    identity_hash   TEXT NOT NULL,
    scripts         TEXT NOT NULL,
    metadata        TEXT NOT NULL,
    UNIQUE (file_id, face_index)
);
CREATE INDEX faces_family ON faces(family COLLATE NOCASE);
CREATE INDEX faces_psname ON faces(postscript_name);
CREATE INDEX faces_identity ON faces(identity_hash);
CREATE INDEX faces_license ON faces(license_spdx);

CREATE VIRTUAL TABLE faces_fts USING fts5(
    family, subfamily, postscript_name, designer,
    content='faces', content_rowid='id', tokenize='unicode61'
);
CREATE TRIGGER faces_ai AFTER INSERT ON faces BEGIN
    INSERT INTO faces_fts(rowid, family, subfamily, postscript_name, designer)
    VALUES (new.id, new.family, new.subfamily, new.postscript_name, new.designer);
END;
CREATE TRIGGER faces_ad AFTER DELETE ON faces BEGIN
    INSERT INTO faces_fts(faces_fts, rowid, family, subfamily, postscript_name, designer)
    VALUES ('delete', old.id, old.family, old.subfamily, old.postscript_name, old.designer);
END;
CREATE TRIGGER faces_au AFTER UPDATE ON faces BEGIN
    INSERT INTO faces_fts(faces_fts, rowid, family, subfamily, postscript_name, designer)
    VALUES ('delete', old.id, old.family, old.subfamily, old.postscript_name, old.designer);
    INSERT INTO faces_fts(rowid, family, subfamily, postscript_name, designer)
    VALUES (new.id, new.family, new.subfamily, new.postscript_name, new.designer);
END;

CREATE TABLE tags (
    id    INTEGER PRIMARY KEY,
    name  TEXT NOT NULL UNIQUE COLLATE NOCASE
);
CREATE TABLE face_tags (
    face_id INTEGER NOT NULL REFERENCES faces(id) ON DELETE CASCADE,
    tag_id  INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (face_id, tag_id)
);
CREATE TABLE collections (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE COLLATE NOCASE,
    created_at  INTEGER NOT NULL DEFAULT (unixepoch())
);
CREATE TABLE collection_faces (
    collection_id INTEGER NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
    face_id       INTEGER NOT NULL REFERENCES faces(id) ON DELETE CASCADE,
    position      INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (collection_id, face_id)
);
CREATE TABLE sources (
    id          INTEGER PRIMARY KEY,
    path        TEXT NOT NULL UNIQUE,
    watch       INTEGER NOT NULL DEFAULT 1,
    added_at    INTEGER NOT NULL DEFAULT (unixepoch())
);
CREATE TABLE activations (
    face_id     INTEGER NOT NULL REFERENCES faces(id) ON DELETE CASCADE,
    scope       TEXT NOT NULL,
    activated_at INTEGER NOT NULL DEFAULT (unixepoch()),
    PRIMARY KEY (face_id)
);
"#,
    // 2: codepoint ranges per face, for "which fonts cover this text" queries.
    r#"
CREATE TABLE face_ranges (
    face_id INTEGER NOT NULL REFERENCES faces(id) ON DELETE CASCADE,
    lo      INTEGER NOT NULL,
    hi      INTEGER NOT NULL
);
CREATE INDEX face_ranges_face ON face_ranges(face_id);
"#,
    // 3: where a persistent install put its copy; vendor facet index; what a source was
    // added as (explicit path or an OS font directory from `scan --system`).
    r#"
ALTER TABLE activations ADD COLUMN installed_path TEXT;
ALTER TABLE sources ADD COLUMN kind TEXT NOT NULL DEFAULT 'user';
CREATE INDEX faces_vendor ON faces(vendor);
CREATE INDEX face_tags_tag ON face_tags(tag_id);
CREATE INDEX collection_faces_face ON collection_faces(face_id);
"#,
];

pub fn migrate(conn: &mut Connection) -> Result<()> {
    let current: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(current as usize) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        if i == 1 {
            backfill_ranges(&tx)?;
        }
        tx.pragma_update(None, "user_version", (i + 1) as i64)?;
        tx.commit()?;
    }
    Ok(())
}

/// Populate `face_ranges` from the stored metadata JSON of an index created before v2.
fn backfill_ranges(tx: &rusqlite::Transaction) -> Result<()> {
    let rows: Vec<(i64, String)> = {
        let mut stmt = tx.prepare("SELECT id, metadata FROM faces")?;
        let it = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        it.collect::<rusqlite::Result<Vec<_>>>()?
    };
    for (id, json) in rows {
        if let Ok(face) = serde_json::from_str::<crate::model::FaceMetadata>(&json) {
            super::insert_ranges(tx, id, &face.coverage.ranges)?;
        }
    }
    Ok(())
}
