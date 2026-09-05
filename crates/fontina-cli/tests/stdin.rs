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

//! Targets read from standard input: the other half of `--json`.
//!
//! Every command prints JSON and every printed type has a schema, so a program can pipe
//! out of fontina. These are the tests that it can pipe *in*, which is what makes the
//! CLI a plugin surface rather than a reporting tool.

use serde_json::Value;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

/// Run fontina with `input` on standard input.
fn piped(db: &Path, args: &[&str], input: &str) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_fontina"))
        .args(["--db", &db.to_string_lossy()])
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("fontina runs");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    child.wait_with_output().expect("fontina finishes")
}

fn run(db: &Path, args: &[&str]) -> String {
    let o = Command::new(env!("CARGO_BIN_EXE_fontina"))
        .args(["--db", &db.to_string_lossy()])
        .args(args)
        .output()
        .expect("fontina runs");
    assert!(
        o.status.success(),
        "fontina {args:?} failed: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    String::from_utf8(o.stdout).expect("stdout is UTF-8")
}

fn ok(db: &Path, args: &[&str], input: &str) -> String {
    let o = piped(db, args, input);
    assert!(
        o.status.success(),
        "fontina {args:?} failed: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    String::from_utf8_lossy(&o.stdout).into_owned()
}

/// How many faces carry `tag`.
fn count(db: &Path, tag: &str) -> usize {
    let faces: Value = serde_json::from_str(&run(db, &["list", "--json", "--tag", tag])).unwrap();
    faces.as_array().unwrap().len()
}

fn indexed(what: &str) -> (PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("fontina-stdin-{what}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let db = dir.join("index.db");
    run(&db, &["scan", &fixtures().to_string_lossy()]);
    (dir, db)
}

/// A hand-written list, and what `jq -r` produces: one target per line.
#[test]
fn one_target_per_line_with_room_for_a_comment() {
    let (dir, db) = indexed("lines");
    ok(&db, &["tag", "add", "chosen", "-"], "1\n2\n");
    assert_eq!(count(&db, "chosen"), 2);

    ok(
        &db,
        &["tag", "add", "annotated", "-"],
        "# the ones to look at again\n\n  3  \n",
    );
    assert_eq!(count(&db, "annotated"), 1);

    // A family name and a path are targets like any other.
    ok(&db, &["tag", "add", "byname", "-"], "family:Amiri\n");
    assert_eq!(count(&db, "byname"), 1);

    let _ = std::fs::remove_dir_all(&dir);
}

/// The loop closing: what fontina prints, fontina reads. No `jq` in the middle.
#[test]
fn fontinas_own_json_goes_straight_back_in() {
    let (dir, db) = indexed("roundtrip");
    let listing = run(&db, &["list", "--json"]);
    let n = serde_json::from_str::<Value>(&listing)
        .unwrap()
        .as_array()
        .unwrap()
        .len();
    assert!(n > 1);
    ok(&db, &["tag", "add", "everything", "-"], &listing);
    assert_eq!(count(&db, "everything"), n);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The shapes a filter in between will hand over.
#[test]
fn an_array_a_single_object_and_one_object_per_line_all_work() {
    let (dir, db) = indexed("shapes");

    // `jq '[.[].id]'`
    ok(&db, &["tag", "add", "ids", "-"], "[1, 2, 3]\n");
    assert_eq!(count(&db, "ids"), 3);

    // `jq -c '.[] | select(...)'` — a stream, not a document.
    ok(
        &db,
        &["tag", "add", "streamed", "-"],
        "{\"id\":1,\"family\":\"Amiri\"}\n{\"id\":2,\"family\":\"x\"}\n",
    );
    assert_eq!(count(&db, "streamed"), 2);

    // `jq '.[0]'` — one object on its own.
    ok(&db, &["tag", "add", "single", "-"], "{\"id\":4}\n");
    assert_eq!(count(&db, "single"), 1);

    // An object with no id falls back to its path, which is what `fontina list --json`
    // gives for a face and what a filter that dropped the id would leave.
    let path = fixtures()
        .canonicalize()
        .unwrap()
        .join("Amiri-Regular.ttf")
        .to_string_lossy()
        .into_owned();
    ok(
        &db,
        &["tag", "add", "bypath", "-"],
        &format!("[{}]\n", serde_json::json!({ "path": path })),
    );
    assert_eq!(count(&db, "bypath"), 1);

    let _ = std::fs::remove_dir_all(&dir);
}

/// A `-` beside ordinary targets, and standard input read exactly once.
#[test]
fn a_dash_stands_among_the_other_targets() {
    let (dir, db) = indexed("mixed");
    ok(&db, &["tag", "add", "both", "1", "-"], "2\n");
    assert_eq!(count(&db, "both"), 2);

    // Two dashes must not read twice and hang, and must not duplicate what came in.
    ok(&db, &["tag", "add", "twice", "-", "-"], "3\n");
    assert_eq!(count(&db, "twice"), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

/// A pipe that produced nothing, and JSON that is not a target, are both said out loud.
/// A program in a pipeline that silently does nothing is worse than one that stops.
#[test]
fn an_empty_pipe_and_unusable_json_are_errors_with_a_reason() {
    let (dir, db) = indexed("errors");

    let o = piped(&db, &["tag", "add", "nothing", "-"], "");
    assert!(!o.status.success());
    assert!(
        String::from_utf8_lossy(&o.stderr).contains("nothing on standard input"),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );

    let o = piped(&db, &["tag", "add", "bad", "-"], "[{\"family\":\"Amiri\"}]");
    assert!(!o.status.success());
    let stderr = String::from_utf8_lossy(&o.stderr);
    assert!(
        stderr.contains("`id`") && stderr.contains("`path`"),
        "{stderr}"
    );

    // And nothing was applied on the way to failing.
    assert_eq!(count(&db, "nothing"), 0);
    assert_eq!(count(&db, "bad"), 0);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Not only the id-taking commands: anything that names faces takes them this way.
#[test]
fn the_commands_that_read_whole_faces_take_a_pipe_too() {
    let (dir, db) = indexed("faces");
    let css = ok(&db, &["css", "-"], "1\n2\n");
    assert_eq!(css.matches("@font-face").count(), 2, "{css}");

    let licenses = ok(&db, &["license", "--json", "-"], "1\n");
    assert!(licenses.contains("OFL"), "{licenses}");

    let out = dir.join("s.html");
    ok(
        &db,
        &["specimen", "-", "-o", &out.to_string_lossy()],
        "{\"id\":1}\n",
    );
    assert!(std::fs::metadata(&out).unwrap().len() > 0);

    let _ = std::fs::remove_dir_all(&dir);
}
