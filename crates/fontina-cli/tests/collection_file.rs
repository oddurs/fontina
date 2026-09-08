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

//! Several faces in one file, which is what a `.ttc` is.
//!
//! `fixtures/` has none, so nothing exercised the paths that turn on it: a face named by
//! `path#index` on the way in and printed the same way on the way out, an `info` that
//! says which face of how many, a `@font-face` rule whose URL carries the fragment that
//! selects the face, and a specimen that does the same. Every one of those is a
//! different piece of code reading the same `face_count > 1`.
//!
//! The collection is built here rather than committed: see `common::write_collection`.

mod common;

use fontina_testkit::Cli;
use serde_json::Value;
use std::path::Path;

/// A sandbox holding one `.ttc` of two fixtures, indexed, with a terminal wide enough
/// that the table does not elide the paths these tests read.
fn session(name: &str) -> (Cli, std::path::PathBuf) {
    let cli = fontina_testkit::cli!(name).with_env("COLUMNS", "200");
    let fonts = cli.dir("fonts");
    let ttc = fonts.join("two.ttc");
    common::write_collection(
        &[
            &cli.fixtures().join("Amiri-Regular.ttf"),
            &cli.fixtures().join("SourceSerif4-Regular.otf"),
        ],
        &ttc,
    );
    let ttc = std::fs::canonicalize(&ttc).expect("the collection was written");
    cli.ok(&["scan", &fonts.to_string_lossy()]);
    (cli, ttc)
}

#[test]
fn each_face_of_a_collection_is_indexed_named_and_addressable() {
    let (s, ttc) = session("addressable");
    let faces: Value = serde_json::from_str(&s.ok(&["list", "--json"])).unwrap();
    let faces = faces.as_array().expect("a list");
    assert_eq!(faces.len(), 2, "{faces:?}");
    assert_eq!(faces[0]["index"], 0);
    assert_eq!(faces[1]["index"], 1);
    assert_eq!(faces[0]["container"], "ttc");
    assert_eq!(faces[0]["family"], "Amiri");
    assert_eq!(faces[1]["family"], "Source Serif 4");

    // The human listing names the face within the file, so a reader can tell the two
    // rows apart by more than their ids.
    let listed = s.ok(&["list"]);
    let path = self::path_of(&ttc);
    assert!(listed.contains(&format!("{path}#0")), "{listed}");
    assert!(listed.contains(&format!("{path}#1")), "{listed}");

    // And that is an address `info` accepts, not only a display form.
    let by_path = s.ok(&["info", "--json", &format!("{path}#1")]);
    let by_path: Value = serde_json::from_str(&by_path).unwrap();
    assert_eq!(by_path[0]["names"]["family"], "Source Serif 4");

    let human = s.ok(&["info", &faces[1]["id"].to_string()]);
    assert!(
        human.contains("face 1 of 2"),
        "info says which face of how many:\n{human}"
    );
}

fn path_of(ttc: &Path) -> String {
    ttc.to_string_lossy().into_owned()
}

/// The address a listing prints is an address a command accepts.
///
/// A row of `fontina list` ends in `path#1` for the second face of a collection, and the
/// obvious thing to do with a line of output is paste it back. It used to answer "no such
/// file, and not a face id" for a path the reader was looking at.
#[test]
fn the_address_a_listing_prints_can_be_pasted_back() {
    let (s, ttc) = session("address");
    let path = path_of(&ttc);

    for (want, family) in [(0, "Amiri"), (1, "Source Serif 4")] {
        let target = format!("{path}#{want}");
        let faces: Value = serde_json::from_str(&s.ok(&["info", "--json", &target])).unwrap();
        assert_eq!(
            faces.as_array().expect("a list").len(),
            1,
            "one face: {faces}"
        );
        assert_eq!(faces[0]["names"]["family"], family);
        assert_eq!(faces[0]["index"], want);
    }

    // A face the file does not have is an error naming the file, not a panic.
    let out = s.run(&["info", &format!("{path}#7")]);
    assert_eq!(out.status.code(), Some(1));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("no face 7"), "{err}");

    // The whole path is still a target, and it means every face in the file.
    let all: Value = serde_json::from_str(&s.ok(&["info", "--json", &path])).unwrap();
    assert_eq!(all.as_array().expect("a list").len(), 2);
}

/// The rules and documents that reference the file say which face they mean.
///
/// CSS Fonts 4 §4.3: a `url()` naming a collection selects the face with a fragment.
/// Without it a browser takes the first face of the file for every rule, so a stylesheet
/// for a collection would set everything in one font and look like a rendering bug.
#[test]
fn what_is_exported_carries_the_fragment_that_selects_the_face() {
    let (s, _ttc) = session("fragment");
    let faces: Value = serde_json::from_str(&s.ok(&["list", "--json"])).unwrap();
    let second = faces[1]["id"].to_string();

    let css = s.ok(&["css", &second]);
    assert!(
        css.contains("#1\"") || css.contains("#1'"),
        "the rule selects face 1 of the file:\n{css}"
    );
    assert!(
        css.contains("format(\"collection\")"),
        "and says what kind of file it is:\n{css}"
    );

    let out = s.root().join("specimen.html");
    s.ok(&[
        "specimen",
        "--link",
        "--output",
        &out.to_string_lossy(),
        &second,
    ]);
    let html = std::fs::read_to_string(&out).unwrap();
    assert!(
        html.contains("#1\""),
        "the specimen selects the same face it was asked for"
    );
}

/// A collection scanned twice is the same two faces, not four.
#[test]
fn rescanning_a_collection_does_not_double_it() {
    let (s, ttc) = session("rescan");
    let before: Value = serde_json::from_str(&s.ok(&["list", "--json"])).unwrap();
    s.ok(&["scan", &s.root().join("fonts").to_string_lossy()]);
    let after: Value = serde_json::from_str(&s.ok(&["list", "--json"])).unwrap();
    assert_eq!(
        after.as_array().unwrap().len(),
        before.as_array().unwrap().len(),
        "a second scan of the same collection changed the count"
    );

    // A file that has gone away stays in the index until someone says otherwise: a
    // scan of a directory that is temporarily unmounted must not empty the library.
    std::fs::remove_file(Path::new(&path_of(&ttc))).unwrap();
    s.ok(&["scan", &s.root().join("fonts").to_string_lossy()]);
    let kept: Value = serde_json::from_str(&s.ok(&["list", "--json"])).unwrap();
    assert_eq!(
        kept.as_array().unwrap().len(),
        2,
        "a plain scan dropped faces whose file was missing"
    );

    // `--prune` is how someone says otherwise, and it takes every face of the file, not
    // the first one it finds.
    s.ok(&["scan", "--prune", &s.root().join("fonts").to_string_lossy()]);
    let gone: Value = serde_json::from_str(&s.ok(&["list", "--json"])).unwrap();
    assert!(
        gone.as_array().unwrap().is_empty(),
        "pruning left a face of a collection behind: {gone}"
    );
}
