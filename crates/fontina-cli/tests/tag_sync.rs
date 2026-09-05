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

//! `fontina tag sync`, against the running system's real tag store.

#[cfg(unix)]
mod common;

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

fn out(db: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_fontina"))
        .args(["--db", &db.to_string_lossy()])
        .args(args)
        .output()
        .expect("fontina runs")
}

fn run(db: &Path, args: &[&str]) -> String {
    let o = out(db, args);
    assert!(
        o.status.success(),
        "fontina {args:?} failed: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    String::from_utf8(o.stdout).expect("stdout is UTF-8")
}

/// The index's tags for every face, as `(id, tags)`.
///
/// Only the tests that actually sync use it, and those are Unix-only; CI builds with
/// `-D warnings`, so an ungated helper is a build failure on Windows.
#[cfg(unix)]
fn indexed(db: &Path) -> Vec<(i64, Vec<String>)> {
    let faces: Value = serde_json::from_str(&run(db, &["list", "--json"])).unwrap();
    faces
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                f["id"].as_i64().unwrap(),
                f["tags"]
                    .as_array()
                    .map(|t| {
                        t.iter()
                            .map(|s| s.as_str().unwrap().to_string())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default(),
            )
        })
        .collect()
}

/// A directory holding two fonts and an index over them.
fn library(what: &str) -> (PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("fontina-sync-{what}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("fonts")).unwrap();
    for f in ["Amiri-Regular.ttf", "SourceSerif4-Regular.otf"] {
        std::fs::copy(fixtures().join(f), dir.join("fonts").join(f)).unwrap();
    }
    let db = dir.join("index.db");
    run(&db, &["scan", &dir.join("fonts").to_string_lossy()]);
    (dir, db)
}

#[test]
fn a_direction_is_required_because_guessing_would_lose_tags() {
    let (dir, db) = library("direction");
    let o = out(&db, &["tag", "sync"]);
    assert!(!o.status.success());
    let stderr = String::from_utf8_lossy(&o.stderr);
    assert!(
        stderr.contains("--to-files") && stderr.contains("--from-files"),
        "{stderr}"
    );

    // And not both at once.
    let o = out(&db, &["tag", "sync", "--to-files", "--from-files"]);
    assert!(!o.status.success());
    let _ = std::fs::remove_dir_all(&dir);
}

#[cfg(unix)]
#[test]
fn tags_go_out_to_the_files_and_come_back_from_them() {
    let (dir, db) = library("roundtrip");
    run(&db, &["tag", "add", "shortlist", "1"]);
    run(&db, &["tag", "add", "serif", "1", "2"]);

    // A dry run says what it would do and does none of it.
    let report: Value = serde_json::from_str(&run(
        &db,
        &["tag", "sync", "--to-files", "--dry-run", "--json"],
    ))
    .unwrap();
    if report["skipped"].as_array().is_some_and(|s| {
        !s.is_empty() && s[0]["reason"].as_str().unwrap_or("").contains("filesystem")
    }) {
        eprintln!("skipped: no extended attributes here");
        let _ = std::fs::remove_dir_all(&dir);
        return;
    }
    assert_eq!(report["changed"], 2, "{report}");
    assert_eq!(report["dry_run"], true);
    let unchanged: Value = serde_json::from_str(&run(
        &db,
        &["tag", "sync", "--to-files", "--dry-run", "--json"],
    ))
    .unwrap();
    assert_eq!(unchanged["changed"], 2, "the dry run wrote nothing");

    // For real, and then a second time, which must find nothing to do.
    let report: Value =
        serde_json::from_str(&run(&db, &["tag", "sync", "--to-files", "--json"])).unwrap();
    assert_eq!(report["changed"], 2, "{report}");
    let again: Value =
        serde_json::from_str(&run(&db, &["tag", "sync", "--to-files", "--json"])).unwrap();
    assert_eq!(again["changed"], 0, "syncing twice is syncing once");

    // The tags are on the files, not just in the index: a fresh index over the same
    // directory knows nothing, and reading the files gives the tags back.
    let fresh = dir.join("fresh.db");
    run(&fresh, &["scan", &dir.join("fonts").to_string_lossy()]);
    assert!(
        indexed(&fresh).iter().all(|(_, t)| t.is_empty()),
        "a new index starts with no tags"
    );
    let report: Value =
        serde_json::from_str(&run(&fresh, &["tag", "sync", "--from-files", "--json"])).unwrap();
    assert_eq!(report["changed"], 2, "{report}");
    let mut got: Vec<Vec<String>> = indexed(&fresh).into_iter().map(|(_, t)| t).collect();
    got.sort();
    assert_eq!(
        got,
        vec![
            vec!["serif".to_string()],
            vec!["serif".to_string(), "shortlist".to_string()]
        ]
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// Sync mirrors: the side you name is right, and the other loses what it has that the
/// first does not. Anything else would need a common ancestor neither side has.
#[cfg(unix)]
#[test]
fn the_side_you_name_wins_including_its_deletions() {
    let (dir, db) = library("mirror");
    run(&db, &["tag", "add", "keep", "1"]);
    run(&db, &["tag", "add", "drop", "1"]);
    if out(&db, &["tag", "sync", "--to-files"]).status.success() {
        // fine
    } else {
        eprintln!("skipped: no extended attributes here");
        let _ = std::fs::remove_dir_all(&dir);
        return;
    }

    // The index loses one. `--to-files` must take it off the file too.
    run(&db, &["tag", "remove", "drop", "1"]);
    let report: Value =
        serde_json::from_str(&run(&db, &["tag", "sync", "--to-files", "--json"])).unwrap();
    assert_eq!(report["changes"][0]["removed"][0], "drop", "{report}");

    // And the other way: a file with no tags empties the index for that face.
    let report: Value =
        serde_json::from_str(&run(&db, &["tag", "sync", "--from-files", "--json"])).unwrap();
    assert_eq!(report["changed"], 0, "they already agree: {report}");
    run(&db, &["tag", "add", "invented", "2"]);
    let report: Value =
        serde_json::from_str(&run(&db, &["tag", "sync", "--from-files", "--json"])).unwrap();
    assert_eq!(report["changes"][0]["removed"][0], "invented", "{report}");
    assert!(
        indexed(&db)
            .iter()
            .find(|(id, _)| *id == 2)
            .unwrap()
            .1
            .is_empty(),
        "the file said it had none"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// A tag the file store cannot hold is left in the index and reported, rather than
/// abandoning the sync or mangling it on the way out.
#[cfg(unix)]
#[test]
fn a_tag_the_files_cannot_hold_stays_in_the_index_and_is_named() {
    let (dir, db) = library("cannot-hold");
    run(&db, &["tag", "add", "fine", "1"]);
    run(&db, &["tag", "add", "not,fine", "1"]);
    let report: Value =
        serde_json::from_str(&run(&db, &["tag", "sync", "--to-files", "--json"])).unwrap();
    let skipped = report["skipped"].as_array().unwrap();
    if skipped
        .iter()
        .any(|s| s["reason"].as_str().unwrap_or("").contains("filesystem"))
    {
        eprintln!("skipped: no extended attributes here");
        let _ = std::fs::remove_dir_all(&dir);
        return;
    }
    assert!(
        skipped
            .iter()
            .any(|s| s["reason"].as_str().unwrap().contains("not,fine")),
        "{report}"
    );
    assert_eq!(
        report["changes"][0]["added"],
        serde_json::json!(["fine"]),
        "the storable one still went: {report}"
    );
    // And it is still in the index, where it works.
    assert!(
        indexed(&db)[0].1.contains(&"not,fine".to_string()),
        "{:?}",
        indexed(&db)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Never modify a system font directory. That rule is older than this command, and
/// `--to-files` over a library scanned with `--system` is the way it would be broken.
#[cfg(unix)]
#[test]
fn a_font_the_operating_system_ships_is_never_written_to() {
    let dirs = fontina_platform::system_font_dirs();
    let Some(readonly) = dirs.iter().find(|d| !d.user_writable && d.path.is_dir()) else {
        eprintln!("skipped: no read-only system font directory here");
        return;
    };
    let Some(font) = std::fs::read_dir(&readonly.path)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| ["ttf", "otf", "ttc"].contains(&e.to_ascii_lowercase().as_str()))
        })
    else {
        eprintln!("skipped: {} holds no font", readonly.path.display());
        return;
    };

    let dir = std::env::temp_dir().join(format!("fontina-sync-system-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let db = dir.join("index.db");
    run(&db, &["scan", &font.to_string_lossy()]);
    run(&db, &["tag", "add", "mine", "1"]);

    let report: Value =
        serde_json::from_str(&run(&db, &["tag", "sync", "--to-files", "--json"])).unwrap();
    assert_eq!(report["changed"], 0, "{report}");
    assert!(
        report["skipped"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["reason"].as_str().unwrap().contains("does not write to")),
        "{report}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// ----- what a review of this command found -----------------------------------------

/// A tag that differs only in case is one tag, not an addition and a removal.
///
/// The index folds tag names, so `Work` and `work` are the same row there. Comparing the
/// two sides byte-wise made a difference of case look like both an add and a remove: the
/// add resolved to the row that already existed and did nothing, the remove then deleted
/// it, and the run after that put it back. The report said "added" while the tag went
/// away.
#[cfg(unix)]
#[test]
fn a_difference_of_case_is_not_a_difference() {
    let (dir, db) = library("case");
    run(&db, &["tag", "add", "work", "1"]);
    run(&db, &["tag", "sync", "--to-files", "1"]);

    // Something else rewrites the file's tag with a capital, as Finder would.
    let font = dir.join("fonts/Amiri-Regular.ttf");
    fontina_platform::tags::write(&font, &["Work".into()]).unwrap();

    let before = indexed(&db);
    run(&db, &["tag", "sync", "--from-files", "1"]);
    let after = indexed(&db);
    assert_eq!(before, after, "a change of case is not a change");

    // And it stays that way: the defect oscillated, so one more round is the test.
    run(&db, &["tag", "sync", "--from-files", "1"]);
    assert_eq!(indexed(&db), after, "and the run after that");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A tag the file store cannot hold survives a sync in both directions.
///
/// The index accepts a comma; `user.xdg.tags` separates on one and a Finder tag uses a
/// newline for its colour. Such a tag is not the file's to hold, and it was being deleted
/// from the index by `--from-files` with nothing printed, and dropped from the file by
/// `--to-files` while the report said it had been kept.
#[cfg(unix)]
#[test]
fn a_tag_the_file_cannot_hold_is_kept_rather_than_lost() {
    let (dir, db) = library("kept-both-ways");
    run(&db, &["tag", "add", "not,fine", "1"]);
    run(&db, &["tag", "add", "fine", "1"]);

    let out = run(&db, &["tag", "sync", "--to-files", "1", "--json"]);
    let report: Value = serde_json::from_str(&out).unwrap();
    let skips = report["skipped"].as_array().unwrap();
    assert!(
        skips
            .iter()
            .any(|s| s["reason"].as_str().unwrap().contains("not,fine")),
        "the tag it cannot write is named: {out}"
    );
    let has = |t: &str| {
        indexed(&db)
            .iter()
            .any(|(_, tags)| tags.iter().any(|x| x == t))
    };
    assert!(has("not,fine"), "kept in the index, as the skip line says");

    // Reading back does not delete it either.
    run(&db, &["tag", "sync", "--from-files", "1"]);
    assert!(
        has("not,fine"),
        "--from-files must not delete what it cannot read back"
    );
    assert!(has("fine"), "and the ordinary tag is still there");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A selection rewrites only the files it named.
///
/// This is the half of the "partial selection" defect that two separate files can reach.
/// The other half is a collection, and it is below.
#[cfg(unix)]
#[test]
fn a_selection_rewrites_only_the_files_it_named() {
    let (dir, db) = library("partial");
    // Both faces of the same file, tagged differently. Amiri is one face, so tag the two
    // fonts and sync only one of them: the same shape, one file at a time.
    run(&db, &["tag", "add", "mine", "1"]);
    run(&db, &["tag", "add", "theirs", "2"]);
    run(&db, &["tag", "sync", "--to-files"]);

    // Now sync face 1 alone. Face 2 lives in another file here, so its file must be
    // untouched — the regression is that a selection rewrites files it did not name.
    run(&db, &["tag", "sync", "--to-files", "1"]);
    let other = dir.join("fonts/SourceSerif4-Regular.otf");
    let on_other = fontina_platform::tags::read(&other).unwrap();
    assert!(
        on_other.iter().any(|t| t == "theirs"),
        "syncing one face rewrote another file: {on_other:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Syncing one face of a collection writes every face's tags, not just that one's.
///
/// The other half of the partial-selection defect, and the one the fixtures could not
/// reach until this test built its own collection. Several faces share one file and one
/// set of file tags, so writing only the named face's tags over the file silently takes
/// the others' away. `faces_by_file` widens each file to every face in it; this is what
/// that is for.
#[cfg(unix)]
#[test]
fn syncing_one_face_of_a_collection_keeps_the_others_tags() {
    let dir = std::env::temp_dir().join(format!("fontina-sync-ttc-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("fonts")).unwrap();
    let collection = dir.join("fonts/two.ttc");
    common::write_collection(
        &[
            &fixtures().join("Amiri-Regular.ttf"),
            &fixtures().join("SourceSerif4-Regular.otf"),
        ],
        &collection,
    );
    let db = dir.join("index.db");
    run(&db, &["scan", &dir.join("fonts").to_string_lossy()]);

    // Canonical, because that is what the index stores: on macOS the temporary directory
    // is reached through a symlink.
    let collection = std::fs::canonicalize(&collection).unwrap();
    let faces: Value = serde_json::from_slice(&out(&db, &["list", "--json"]).stdout).unwrap();
    let faces = faces.as_array().expect("a list");
    assert_eq!(faces.len(), 2, "one file, two faces: {faces:?}");
    assert!(
        faces
            .iter()
            .all(|f| f["path"].as_str().unwrap_or_default() == collection.to_string_lossy()),
        "both faces are in the one file: {faces:?}"
    );
    let first = faces[0]["id"].to_string();
    let second = faces[1]["id"].to_string();

    run(&db, &["tag", "add", "mine", &first]);
    run(&db, &["tag", "add", "theirs", &second]);

    // One face named, one file written, and the file carries what both faces have.
    run(&db, &["tag", "sync", "--to-files", &first]);
    let on_file = fontina_platform::tags::read(&collection).unwrap();
    assert!(
        on_file.iter().any(|t| t == "mine"),
        "the named face's tag is on the file: {on_file:?}"
    );
    assert!(
        on_file.iter().any(|t| t == "theirs"),
        "and so is the other face's, which shares the file: {on_file:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// A run in which nothing could be done fails, so a script can tell.
///
/// Every write error becomes a skip, by design: one unreadable font should not stop three
/// hundred others. But a run where every file was skipped by a real failure printed
/// `0 of n file(s) changed` and exited 0, which a script cannot tell from "nothing to do".
#[cfg(unix)]
#[test]
fn a_run_where_everything_failed_is_a_failure() {
    let (dir, db) = library("allfailed");
    run(&db, &["tag", "add", "shortlist", "1", "2"]);
    // Take the fonts away: reading their tags now fails for every file.
    std::fs::remove_dir_all(dir.join("fonts")).unwrap();

    let o = out(&db, &["tag", "sync", "--to-files"]);
    assert!(
        !o.status.success(),
        "a run that did nothing at all must not exit 0: {}",
        String::from_utf8_lossy(&o.stdout)
    );
    let _ = std::fs::remove_dir_all(&dir);
}
