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

//! A collection bundle, across two indexes, through the binary.
//!
//! The unit tests in `fontina-core` cover what `write_bundle` puts on disk. What they
//! cannot cover is the claim the feature actually makes: that a directory written by one
//! copy of fontina, carried somewhere else, opens in another one and gives back the same
//! collection. That needs two indexes that share nothing, and it needs the bundle to be
//! read from a directory that is not the one it was written in.

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

/// Run fontina against `db`, expecting it to succeed.
fn run(db: &Path, args: &[&str]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_fontina"))
        .args(["--db", &db.to_string_lossy()])
        .args(args)
        .output()
        .expect("fontina runs");
    assert!(
        out.status.success(),
        "fontina {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("stdout is UTF-8")
}

/// The faces of a collection, as `(family, subfamily, tags)` in the order they are held.
fn shape(db: &Path, name: &str) -> Vec<(String, String, Vec<String>)> {
    let json: Value = serde_json::from_str(&run(db, &["collection", "show", name, "--json"]))
        .expect("collection show --json is JSON");
    json.as_array()
        .expect("an array of faces")
        .iter()
        .map(|f| {
            (
                f["family"].as_str().unwrap().to_string(),
                f["subfamily"].as_str().unwrap().to_string(),
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

#[test]
fn a_bundle_written_by_one_index_opens_in_another_somewhere_else() {
    let dir = std::env::temp_dir().join(format!("fontina-bundle-cli-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let dir = std::fs::canonicalize(&dir).unwrap();

    // The index the collection was curated in.
    let mine = dir.join("mine.db");
    run(&mine, &["scan", &fixtures().to_string_lossy()]);
    run(&mine, &["collection", "create", "Handoff"]);
    run(&mine, &["tag", "add", "shortlist", "family:Amiri"]);
    // Ids rather than families, so the order below is one this test chose.
    run(&mine, &["collection", "add", "Handoff", "3", "1", "2"]);
    let before = shape(&mine, "Handoff");
    assert_eq!(before.len(), 3);
    assert!(
        before.iter().any(|(_, _, tags)| tags == &["shortlist"]),
        "a tag to carry across: {before:?}"
    );

    let written = dir.join("written-here");
    run(
        &mine,
        &[
            "collection",
            "export",
            "Handoff",
            "--bundle",
            &written.to_string_lossy(),
        ],
    );

    // Nothing in the file may name this machine, or it is not a bundle anyone can use.
    let text = std::fs::read_to_string(written.join("collection.json")).unwrap();
    assert!(
        !text.contains(
            fixtures()
                .canonicalize()
                .unwrap()
                .to_string_lossy()
                .as_ref()
        ) && !text.contains(dir.to_string_lossy().as_ref()),
        "the bundle still names where it was made"
    );

    // Carried elsewhere. Renaming the directory is the cheapest honest stand-in for
    // handing it to somebody, and it is exactly what an absolute path would not survive.
    let carried = dir.join("opened-there");
    std::fs::rename(&written, &carried).unwrap();

    // A second index that has never seen the fixtures directory.
    let yours = dir.join("yours.db");
    run(&yours, &["scan", &carried.to_string_lossy()]);
    let report: Value = serde_json::from_str(&run(
        &yours,
        &["collection", "import", &carried.to_string_lossy(), "--json"],
    ))
    .expect("import --json is JSON");
    assert_eq!(report["matched"], 3, "{report}");
    assert_eq!(
        report["missing"].as_array().map(Vec::len),
        Some(0),
        "{report}"
    );

    // The same faces, in the same order, with the tags that were on them.
    assert_eq!(shape(&yours, "Handoff"), before);

    // And every path in the second index is inside the bundle where it now lives.
    let faces: Value =
        serde_json::from_str(&run(&yours, &["collection", "show", "Handoff", "--json"])).unwrap();
    for f in faces.as_array().unwrap() {
        let p = f["path"].as_str().unwrap();
        assert!(p.starts_with(carried.to_string_lossy().as_ref()), "{p}");
        assert!(Path::new(p).is_file(), "{p}");
    }

    let _ = std::fs::remove_dir_all(&dir);
}

/// The paths a bundle carries mean nothing until they are joined onto the directory the
/// file was read from, and only the importer knows what that is.
///
/// Nothing above catches a missing join: `import` matches on the identity hash first, so
/// it finds the faces whether or not the paths were ever resolved. What it reports about
/// the faces it *cannot* find is the one place the resolved path is visible.
#[test]
fn what_a_bundle_says_is_missing_is_a_path_you_can_go_and_look_at() {
    let dir = std::env::temp_dir().join(format!("fontina-bundle-miss-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let dir = std::fs::canonicalize(&dir).unwrap();

    let mine = dir.join("mine.db");
    run(&mine, &["scan", &fixtures().to_string_lossy()]);
    run(&mine, &["collection", "create", "Handoff"]);
    run(&mine, &["collection", "add", "Handoff", "1", "2"]);
    let bundle = dir.join("bundle");
    run(
        &mine,
        &[
            "collection",
            "export",
            "Handoff",
            "--bundle",
            &bundle.to_string_lossy(),
        ],
    );

    // An index that has never scanned anything, so every face is missing.
    let empty = dir.join("empty.db");
    let report: Value = serde_json::from_str(&run(
        &empty,
        &["collection", "import", &bundle.to_string_lossy(), "--json"],
    ))
    .expect("import --json is JSON");
    assert_eq!(report["matched"], 0, "{report}");
    let missing = report["missing"].as_array().expect("an array");
    assert_eq!(missing.len(), 2, "{report}");
    for m in missing {
        let p = m["path"].as_str().unwrap();
        assert!(
            Path::new(p).is_absolute(),
            "a relative path here is one the reader cannot act on: {p}"
        );
        assert!(p.starts_with(bundle.to_string_lossy().as_ref()), "{p}");
        assert!(Path::new(p).is_file(), "the font is right there: {p}");
    }

    let _ = std::fs::remove_dir_all(&dir);
}

/// A teammate who already has the font keeps their own copy.
///
/// Matching is by identity hash first, so the bundle's copy is recognised as the font
/// that is already indexed rather than added beside it. Getting this wrong is not a
/// visible failure — it is a library that quietly doubles every time somebody shares
/// something.
#[test]
fn importing_a_font_you_already_have_does_not_give_you_two_of_it() {
    let dir = std::env::temp_dir().join(format!("fontina-bundle-dup-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let dir = std::fs::canonicalize(&dir).unwrap();

    let mine = dir.join("mine.db");
    run(&mine, &["scan", &fixtures().to_string_lossy()]);
    run(&mine, &["collection", "create", "Handoff"]);
    run(&mine, &["collection", "add", "Handoff", "1", "2", "3"]);
    let bundle = dir.join("bundle");
    run(
        &mine,
        &[
            "collection",
            "export",
            "Handoff",
            "--bundle",
            &bundle.to_string_lossy(),
        ],
    );

    // A colleague with the same fonts, indexed from their own directory. They never scan
    // the bundle — they were handed it and they already own what is in it.
    let theirs = dir.join("theirs.db");
    run(&theirs, &["scan", &fixtures().to_string_lossy()]);
    let all: Value = serde_json::from_str(&run(&theirs, &["list", "--json"])).unwrap();
    let before = all.as_array().unwrap().len();

    let report: Value = serde_json::from_str(&run(
        &theirs,
        &["collection", "import", &bundle.to_string_lossy(), "--json"],
    ))
    .expect("import --json is JSON");
    assert_eq!(report["matched"], 3, "{report}");

    let all: Value = serde_json::from_str(&run(&theirs, &["list", "--json"])).unwrap();
    assert_eq!(
        all.as_array().unwrap().len(),
        before,
        "importing a collection is not a way to acquire fonts"
    );

    // Their faces, at their paths — nothing points into the bundle.
    let faces: Value =
        serde_json::from_str(&run(&theirs, &["collection", "show", "Handoff", "--json"])).unwrap();
    let own = fixtures().canonicalize().unwrap();
    for f in faces.as_array().unwrap() {
        let p = f["path"].as_str().unwrap();
        assert!(p.starts_with(own.to_string_lossy().as_ref()), "{p}");
    }

    let _ = std::fs::remove_dir_all(&dir);
}
