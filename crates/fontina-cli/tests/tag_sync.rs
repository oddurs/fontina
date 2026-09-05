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
    let (dir, db) = library("refused");
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
