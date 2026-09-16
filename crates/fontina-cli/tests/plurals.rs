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

//! One face is a face.
//!
//! The program said `1 face(s)` in twenty-four places and `1 candidates` in four more.
//! The second kind is the one worth a test: it has no `(s)` to grep for, so it survived
//! the pass that fixed the first kind, and it was in the line every `scan` prints.
//!
//! A library of one is not a contrived case. It is what somebody has after
//! `fontina scan ~/Downloads/the-font-i-just-bought.otf`, which is a reasonable first
//! thing to do.

use fontina_testkit::cli;

/// A sandbox holding exactly one font.
fn one_face(name: &str) -> fontina_testkit::Cli {
    let cli = cli!(name);
    let only = cli.fixtures().join("Amiri-Regular.ttf");
    cli.ok(&["scan", &only.to_string_lossy()]);
    cli
}

/// Every noun this prints has to agree with the number in front of it.
#[test]
fn a_library_of_one_says_one_of_each_thing() {
    let cli = one_face("plural-one");

    for (args, want, never) in [
        (vec!["list"], "1 face", "1 faces"),
        (vec!["families"], "1 family", "1 familys"),
        (vec!["facets"], "1 face in 1 family", "1 faces in 1 familys"),
    ] {
        let out = cli.ok(&args);
        assert!(out.contains(want), "expected `{want}` in:\n{out}");
        assert!(!out.contains(never), "found `{never}` in:\n{out}");
    }
}

/// The first line of every scan, which is the one nobody can avoid reading.
#[test]
fn the_scan_summary_counts_in_the_singular() {
    let cli = cli!("plural-scan");
    let only = cli.fixtures().join("Amiri-Regular.ttf");
    let out = cli.ok(&["scan", &only.to_string_lossy()]);

    assert!(out.contains("1 candidate "), "{out}");
    assert!(out.contains("(1 face)"), "{out}");
    assert!(!out.contains("candidates"), "{out}");
    assert!(!out.contains("1 faces"), "{out}");
}

/// And the plural still says the plural, which is the half a naive fix breaks.
#[test]
fn more_than_one_is_still_plural() {
    let cli = cli!("plural-many");
    let fixtures = cli.fixtures();
    let scan = cli.ok(&["scan", &fixtures.to_string_lossy()]);

    assert!(scan.contains("6 candidates"), "{scan}");
    assert!(scan.contains("(6 faces)"), "{scan}");

    let list = cli.ok(&["list"]);
    assert!(list.contains("6 faces"), "{list}");
    let families = cli.ok(&["families"]);
    assert!(families.contains("5 families"), "{families}");
}

/// The commands that report what they just did to a set of faces.
#[test]
fn an_action_on_one_face_reports_one_face() {
    let cli = one_face("plural-actions");
    let id = cli.id_of("Amiri-Regular.ttf");

    let tagged = cli.ok(&["tag", "add", "favourite", &id]);
    assert!(tagged.contains("tagged 1 face with"), "{tagged}");

    let untagged = cli.ok(&["tag", "remove", "favourite", &id]);
    assert!(untagged.contains("from 1 face"), "{untagged}");

    cli.ok(&["collection", "create", "solo"]);
    let added = cli.ok(&["collection", "add", "solo", &id]);
    assert!(added.contains("added 1 face to"), "{added}");

    let removed = cli.ok(&["collection", "remove", "solo", &id]);
    assert!(removed.contains("removed 1 face from"), "{removed}");
}

/// No `(s)` reaches a reader from any command this can reasonably run.
///
/// The guard against the whole class rather than against the two dozen instances: a
/// thirty-fifth `(s)` added next year fails here without anybody remembering this file
/// exists.
#[test]
fn nothing_prints_a_parenthesised_plural() {
    let cli = cli!("plural-sweep");
    let fixtures = cli.fixtures();
    cli.ok(&["scan", &fixtures.to_string_lossy()]);

    let commands: [&[&str]; 9] = [
        &["list"],
        &["families"],
        &["facets"],
        &["stats"],
        &["license"],
        &["dupes"],
        &["source", "list"],
        &["tag", "list"],
        &["collection", "list"],
    ];

    for args in commands {
        let out = cli.ok(args);
        for bad in ["(s)", "(es)", "(ies)"] {
            assert!(
                !out.contains(bad),
                "`fontina {}` printed `{bad}`:\n{out}",
                args.join(" ")
            );
        }
    }
}
