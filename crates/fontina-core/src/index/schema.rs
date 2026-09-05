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

//! The index schema, as an append-only list of migrations.
//!
//! Index `i` in [`MIGRATIONS`] applies when `PRAGMA user_version` is `i`. An applied
//! migration is never edited: a database in the wild has already run it, and changing
//! it would leave two different schemas claiming the same version. A migration that
//! needs data the old schema did not store reads it back out of the stored metadata
//! JSON in a backfill keyed on its own index, which is what `face_ranges` does.

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
    // 4: the range a variable face spans, so `--weight 400` finds a font whose `wght`
    // axis reaches 400 rather than only one whose default instance sits there. Equal to
    // the static value for a face with no such axis, which is what makes the filter one
    // clause rather than two.
    r#"
ALTER TABLE faces ADD COLUMN weight_min REAL NOT NULL DEFAULT 0;
ALTER TABLE faces ADD COLUMN weight_max REAL NOT NULL DEFAULT 0;
ALTER TABLE faces ADD COLUMN width_min  REAL NOT NULL DEFAULT 0;
ALTER TABLE faces ADD COLUMN width_max  REAL NOT NULL DEFAULT 0;
-- Seed every row with the static value it already had, so a face whose stored metadata
-- will not parse keeps exactly the behaviour it had before v4 instead of collapsing to
-- a zero-width span that no weight filter can ever match. `backfill_spans` then widens
-- the ones with an axis.
UPDATE faces SET weight_min = weight, weight_max = weight,
                 width_min  = width,  width_max  = width;
CREATE INDEX faces_weight_span ON faces(weight_min, weight_max);
CREATE INDEX faces_width_span ON faces(width_min, width_max);
"#,
    // 5: one row per script a face covers, with how many codepoints of it. `faces.scripts`
    // is a denormalised comma-joined string matched with `LIKE '%,Arab,%'`: it cannot ask
    // for two scripts at once, it throws away the depth `Coverage.scripts` already counts,
    // and it scans. `faces.scripts` stays for now — the browser's facet list reads it, and
    // one change at a time.
    r#"
CREATE TABLE face_scripts (
    face_id    INTEGER NOT NULL REFERENCES faces(id) ON DELETE CASCADE,
    script     TEXT NOT NULL,
    codepoints INTEGER NOT NULL,
    PRIMARY KEY (face_id, script)
);
CREATE INDEX face_scripts_script ON face_scripts(script, codepoints);
"#,
    // 6: the languages a face claims, and which claim it is. Both have been parsed and
    // stored since M0 and neither was reachable: `FaceFilter` had no language field at
    // all, so "which of my fonts declare Vietnamese" could not be put to the index that
    // knows the answer.
    r#"
CREATE TABLE face_languages (
    face_id INTEGER NOT NULL REFERENCES faces(id) ON DELETE CASCADE,
    tag     TEXT NOT NULL,
    source  TEXT NOT NULL,
    PRIMARY KEY (face_id, tag, source)
);
CREATE INDEX face_languages_tag ON face_languages(tag COLLATE NOCASE);
"#,
    // 7: whether the font says it is monospaced. `post.isFixedPitch` has been parsed and
    // stored since M0 and reached neither a filter nor a facet, and on a working
    // developer's library it is the most useful single division there is.
    r#"
ALTER TABLE faces ADD COLUMN is_fixed_pitch INTEGER NOT NULL DEFAULT 0;
CREATE INDEX faces_fixed_pitch ON faces(is_fixed_pitch);
"#,
];

pub fn migrate(conn: &mut Connection) -> Result<()> {
    let current: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    // An index written by a newer fontina has columns and tables this build knows nothing
    // about. Refusing is the only safe answer: silently operating on it would write rows
    // that the newer build then has to interpret, and its migrations would already have
    // been applied under a lower version number.
    if current as usize > MIGRATIONS.len() {
        return Err(crate::error::Error::Other(format!(
            "index schema v{current} was written by a newer fontina; this build understands \
             up to v{}. Upgrade fontina, or point --db (or FONTINA_DB) at another index.",
            MIGRATIONS.len()
        )));
    }
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(current as usize) {
        // Immediate, so two processes opening a fresh index at once queue rather than
        // one of them failing on the upgrade from a read lock.
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        tx.execute_batch(sql)?;
        if i == 1 {
            backfill_ranges(&tx)?;
        }
        if i == 3 {
            backfill_spans(&tx)?;
        }
        if i == 4 {
            backfill_scripts(&tx)?;
        }
        if i == 5 {
            backfill_languages(&tx)?;
        }
        if i == 6 {
            backfill_fixed_pitch(&tx)?;
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

/// Populate the weight and width spans of an index created before v4.
///
/// Nothing is parsed and nothing is rescanned: the axes are already in the stored
/// metadata JSON, which is the whole reason this is a migration and not a re-index.
fn backfill_spans(tx: &rusqlite::Transaction) -> Result<()> {
    let rows: Vec<(i64, String)> = {
        let mut stmt = tx.prepare("SELECT id, metadata FROM faces")?;
        let it = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        it.collect::<rusqlite::Result<Vec<_>>>()?
    };
    let mut stmt = tx.prepare(
        "UPDATE faces SET weight_min = ?2, weight_max = ?3, width_min = ?4, width_max = ?5
         WHERE id = ?1",
    )?;
    for (id, json) in rows {
        // A row whose metadata will not parse keeps the static value it already has,
        // which is what every pre-v4 filter was using anyway.
        let Ok(face) = serde_json::from_str::<crate::model::FaceMetadata>(&json) else {
            continue;
        };
        let (wmin, wmax) = face.weight_span();
        let (dmin, dmax) = face.width_span();
        stmt.execute(rusqlite::params![id, wmin, wmax, dmin, dmax])?;
    }
    Ok(())
}

/// Populate `face_scripts` from the stored metadata JSON of an index created before v5.
///
/// Same shape as `backfill_ranges`: the counts are already in `coverage.scripts`, so
/// nothing is parsed and nobody rescans.
fn backfill_scripts(tx: &rusqlite::Transaction) -> Result<()> {
    let rows: Vec<(i64, String)> = {
        let mut stmt = tx.prepare("SELECT id, metadata FROM faces")?;
        let it = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        it.collect::<rusqlite::Result<Vec<_>>>()?
    };
    for (id, json) in rows {
        if let Ok(face) = serde_json::from_str::<crate::model::FaceMetadata>(&json) {
            super::insert_scripts(tx, id, &face.coverage.scripts)?;
        }
    }
    Ok(())
}

/// Populate `face_languages` from the stored metadata JSON of an index created before v6.
fn backfill_languages(tx: &rusqlite::Transaction) -> Result<()> {
    let rows: Vec<(i64, String)> = {
        let mut stmt = tx.prepare("SELECT id, metadata FROM faces")?;
        let it = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        it.collect::<rusqlite::Result<Vec<_>>>()?
    };
    for (id, json) in rows {
        if let Ok(face) = serde_json::from_str::<crate::model::FaceMetadata>(&json) {
            super::insert_languages(tx, id, &face)?;
        }
    }
    Ok(())
}

/// Populate `faces.is_fixed_pitch` from the stored metadata of an index before v7.
///
/// The `DEFAULT 0` is right for a row this cannot read: "the font did not say so" is
/// exactly what a missing or unreadable `post` table means, and it is what the filter
/// would have concluded anyway.
fn backfill_fixed_pitch(tx: &rusqlite::Transaction) -> Result<()> {
    let rows: Vec<(i64, String)> = {
        let mut stmt = tx.prepare("SELECT id, metadata FROM faces")?;
        let it = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        it.collect::<rusqlite::Result<Vec<_>>>()?
    };
    let mut stmt = tx.prepare("UPDATE faces SET is_fixed_pitch = ?2 WHERE id = ?1")?;
    for (id, json) in rows {
        if let Ok(face) = serde_json::from_str::<crate::model::FaceMetadata>(&json) {
            stmt.execute(rusqlite::params![id, face.metrics.is_fixed_pitch])?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_index_from_a_newer_fontina_is_refused() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn).unwrap();
        let applied: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(applied as usize, MIGRATIONS.len());

        conn.pragma_update(None, "user_version", applied + 1)
            .unwrap();
        let err = migrate(&mut conn).expect_err("a newer schema must be refused");
        assert!(err.to_string().contains("newer fontina"), "{err}");
    }

    #[test]
    fn migrating_is_idempotent() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn).unwrap();
        migrate(&mut conn).unwrap();
    }
}
