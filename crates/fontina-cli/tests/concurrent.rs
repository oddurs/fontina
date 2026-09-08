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

//! Two `fontina` processes against one index.
//!
//! This is an ordinary thing to do, not a stress case: a scan in one terminal while the
//! browser is open in another, `watch` running as a login agent while somebody tags a
//! family, two windows of the same shell. The index is one SQLite file and they all want
//! to write to it.
//!
//! The code has clearly thought about it. `index/mod.rs` carries thirty lines on why
//! setting WAL waits for another connection doing the same, why SQLite answers
//! `SQLITE_BUSY` for that pragma immediately rather than consulting the busy handler, and
//! what happens on a filesystem that cannot do WAL at all; `schema.rs` covers the same
//! ground for migration. What was tested is creation —
//! `several_connections_can_create_the_same_index_at_once`, in-process. Nothing tested
//! two *processes* writing, so a careful argument sat unverified, which is the state in
//! which a careful argument rots.
//!
//! What has to hold, from the point of view of the person running the commands:
//!
//! - both finish, and neither reports a lock as an error
//! - the database is not corrupt afterwards
//! - everything both of them wrote is there
//!
//! The third is the one worth stating: "no crash" is a low bar. A writer that quietly
//! lost its rows to the other's transaction would pass the first two.

use fontina_testkit::Cli;
use std::path::PathBuf;

/// A sandbox with `dirs` directories of fonts, each holding `copies` copies of every
/// fixture under a distinct name.
///
/// Distinct names matter: two directories holding the same bytes would be deduplicated
/// by identity, and then a lost write would look exactly like a duplicate correctly
/// collapsed. Every file here is meant to be its own row.
fn fonts(cli: &Cli, dirs: usize, copies: usize) -> Vec<PathBuf> {
    let sources: Vec<PathBuf> = std::fs::read_dir(cli.fixtures())
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            matches!(
                p.extension().and_then(|e| e.to_str()),
                Some("ttf" | "otf" | "woff" | "woff2")
            )
        })
        .collect();
    assert!(
        !sources.is_empty(),
        "the fixtures directory has fonts in it"
    );

    (0..dirs)
        .map(|d| {
            let dir = cli.dir(&format!("fonts{d}"));
            for i in 0..copies {
                for src in &sources {
                    let base = src.file_name().unwrap().to_string_lossy().into_owned();
                    std::fs::copy(src, dir.join(format!("d{d}-{i}-{base}"))).unwrap();
                }
            }
            dir
        })
        .collect()
}

/// How many faces the index holds.
fn face_count(cli: &Cli) -> usize {
    let listed: serde_json::Value = cli.json(&["list", "--json"]);
    listed.as_array().expect("a list").len()
}

/// SQLite's own verdict on the file, which is the only one worth taking.
#[track_caller]
fn assert_intact(cli: &Cli) {
    let conn = rusqlite::Connection::open(cli.db()).expect("the index opens");
    let verdict: String = conn
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .expect("integrity_check runs");
    assert_eq!(
        verdict, "ok",
        "the index is corrupt after concurrent writes"
    );
}

/// A lock is a thing to wait through, never a thing to report.
///
/// `SQLITE_BUSY` reaching the person as an error is the failure this is really about: the
/// command did not do what they asked, and the reason is an implementation detail of a
/// file format. Matched on the wording SQLite and rusqlite use rather than on an exit
/// code, because a busy error that was caught and turned into some other message would
/// still be this bug.
#[track_caller]
fn no_lock_complaint(what: &str, stderr: &str) {
    let lower = stderr.to_lowercase();
    for phrase in [
        "database is locked",
        "database table is locked",
        "sqlite_busy",
    ] {
        assert!(
            !lower.contains(phrase),
            "{what} told the user about a lock ({phrase}): {stderr}"
        );
    }
}

#[test]
fn two_scans_of_different_directories_into_one_index_both_land() {
    // Enough copies that the two runs overlap for a while rather than finishing in turn.
    let sb = fontina_testkit::cli!("concurrent-scan");
    let dirs = fonts(&sb, 2, 40);

    let mut children: Vec<_> = dirs
        .iter()
        .map(|d| {
            sb.cmd(&["scan", &d.to_string_lossy()])
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .expect("fontina starts")
        })
        .collect();

    let mut outputs = Vec::new();
    for child in children.drain(..) {
        outputs.push(child.wait_with_output().expect("fontina finishes"));
    }

    for (i, o) in outputs.iter().enumerate() {
        let stderr = String::from_utf8_lossy(&o.stderr);
        no_lock_complaint(&format!("scan {i}"), &stderr);
        assert!(
            o.status.success(),
            "scan {i} failed with {}: {stderr}",
            o.status
        );
    }

    assert_intact(&sb);

    // Both writers' work is present, not just one of them. A scan that lost its rows to
    // the other's transaction would have passed everything above.
    let listed = sb.ok(&["list", "--json"]);
    for d in 0..2 {
        assert!(
            listed.contains(&format!("d{d}-")),
            "nothing from directory {d} reached the index"
        );
    }
    // Every file is its own face: the fixtures include one collection, so the count is
    // read from a single-directory scan rather than assumed.
    let solo = fontina_testkit::cli!("concurrent-solo");
    let one = fonts(&solo, 1, 40);
    solo.ok(&["scan", &one[0].to_string_lossy()]);
    let expected = face_count(&solo) * 2;
    assert_eq!(
        face_count(&sb),
        expected,
        "both scans' faces should be in the index"
    );
}

#[test]
fn a_tag_while_a_scan_is_running_is_not_a_lock_error() {
    // A scan long enough to still be going when the tag arrives, which is the shape of
    // the browser tagging a family while an agent rescans.
    let sb = fontina_testkit::cli!("concurrent-tag");
    let dirs = fonts(&sb, 1, 60);

    // One face to tag, indexed before the long scan starts so the id exists.
    sb.scan_fixtures();

    let scan = sb
        .cmd(&["scan", &dirs[0].to_string_lossy()])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("fontina starts");

    let tag = sb
        .cmd(&["tag", "add", "concurrent", "1"])
        .output()
        .expect("fontina runs");
    let scan = scan.wait_with_output().expect("fontina finishes");

    no_lock_complaint("tag", &String::from_utf8_lossy(&tag.stderr));
    no_lock_complaint("scan", &String::from_utf8_lossy(&scan.stderr));
    assert!(
        tag.status.success(),
        "tag failed while a scan was running: {}",
        String::from_utf8_lossy(&tag.stderr)
    );
    assert!(scan.status.success(), "the scan failed while tagging");

    assert_intact(&sb);
    assert!(
        sb.ok(&["list", "--json"]).contains("concurrent"),
        "the tag written during the scan is not in the index"
    );
}
