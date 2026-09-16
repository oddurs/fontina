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

//! The first five minutes: 0080, 0081 and 0082 of the `arrival` milestone.
//!
//! The program used to greet a new person with the word *error* and thirty-four nouns.
//! These are the assertions that it does not, that an empty index is a state rather than
//! a table with no rows, and that a first scan says what was found rather than only what
//! was done.

use fontina_testkit::cli;

/// An index that exists and holds nothing, which is not the same as no index at all and
/// is the state every one of these commands has to have an answer for.
fn empty(name: &str) -> fontina_testkit::Cli {
    let cli = cli!(name);
    let nothing = cli.dir("nothing");
    cli.ok(&["scan", &nothing.to_string_lossy()]);
    cli
}

// ── 0080: `fontina` on its own ───────────────────────────────────────────────

/// Typing the program's name is not a mistake, so it does not produce an error.
#[test]
fn a_bare_fontina_is_not_an_error() {
    let cli = empty("arrival-bare");
    let out = cli.ok(&[]);

    assert!(
        !out.contains("requires a subcommand"),
        "the clap error survived:\n{out}"
    );
    assert!(!out.to_lowercase().contains("error"), "{out}");
    // The thirty-four nouns, which were the other half of the problem.
    assert!(!out.contains("[subcommands:"), "{out}");
}

/// With nothing indexed it says so, and names the command that changes that.
#[test]
fn with_no_index_it_says_where_to_begin() {
    let cli = empty("arrival-empty");
    let out = cli.ok(&[]);

    assert!(out.contains("No fonts indexed yet"), "{out}");
    assert!(
        out.contains("fontina scan"),
        "it should name the command that fills an index:\n{out}"
    );
}

/// With an index it says what is in it, without being asked for a table.
#[test]
fn with_an_index_it_says_what_is_there() {
    let cli = cli!("arrival-full");
    let fixtures = cli.fixtures();
    cli.ok(&["scan", &fixtures.to_string_lossy()]);

    let out = cli.ok(&[]);
    assert!(out.contains("6 faces in 5 families"), "{out}");
    assert!(out.contains("free"), "freedom is part of the shape:\n{out}");
    assert!(out.contains("fontina list"), "and what to do next:\n{out}");
    // Not the table itself — this is a summary, not `list` by another name.
    assert!(!out.contains("Amiri"), "{out}");
}

/// It suggests; it never runs.
///
/// The rule this whole milestone is built on: a program that walks your disk because you
/// typed its name is a program nobody trusts twice.
#[test]
fn it_never_scans_on_its_own() {
    let cli = empty("arrival-no-scan");
    cli.ok(&[]);
    cli.ok(&[]);

    // Still nothing indexed after two runs. If the opening screen had scanned, the
    // fixtures directory is not what it would have found — but the index would not be
    // empty either.
    let listed = cli.ok(&["list"]);
    assert!(listed.contains("nothing indexed"), "{listed}");
}

/// `--help` is untouched: it is the reference and still lists everything.
#[test]
fn help_still_lists_every_command() {
    let cli = empty("arrival-help");
    let out = cli.ok(&["--help"]);

    for command in [
        "scan",
        "list",
        "activate",
        "specimen",
        "schema",
        "completions",
    ] {
        assert!(
            out.contains(command),
            "`{command}` is missing from --help:\n{out}"
        );
    }
}

// ── 0081: an empty index is a state ──────────────────────────────────────────

/// No command answers an empty index with an empty table.
#[test]
fn nothing_answers_an_empty_index_with_a_blank() {
    let cli = empty("arrival-blank");

    for args in [
        vec!["list"],
        vec!["families"],
        vec!["facets"],
        vec!["license"],
    ] {
        let out = cli.ok(&args);
        assert!(
            !out.trim().is_empty(),
            "`fontina {}` said nothing at all",
            args.join(" ")
        );
        assert!(
            out.contains("nothing indexed"),
            "`fontina {}` should say the index is empty:\n{out}",
            args.join(" ")
        );
        assert!(
            out.contains("fontina scan"),
            "`fontina {}` should name the command that fills it:\n{out}",
            args.join(" ")
        );
    }
}

/// An empty *result* is a different thing from an empty *index*, and says so.
///
/// This is the distinction the whole item turns on. On a library of nine hundred, "no
/// faces match" means the filter is too narrow. On a fresh install it meant the same
/// thing and no filter would have helped.
#[test]
fn a_filter_that_matches_nothing_is_not_an_empty_index() {
    let cli = cli!("arrival-filter");
    let fixtures = cli.fixtures();
    cli.ok(&["scan", &fixtures.to_string_lossy()]);

    let out = cli.ok(&["list", "--family", "NoSuchFamilyExists"]);
    assert!(out.contains("no faces match"), "{out}");
    assert!(
        !out.contains("nothing indexed"),
        "there is plenty indexed; the filter is the problem:\n{out}"
    );

    let families = cli.ok(&["families", "--family", "NoSuchFamilyExists"]);
    assert!(families.contains("no families match"), "{families}");
}

/// A machine asked a question gets the true answer, which is an empty list.
#[test]
fn json_is_unaffected_by_any_of_this() {
    let cli = empty("arrival-json");

    let listed = cli.ok(&["list", "--json"]);
    assert_eq!(listed.trim(), "[]", "{listed}");
    assert!(!listed.contains("nothing indexed"), "{listed}");

    let families = cli.ok(&["families", "--json"]);
    assert_eq!(families.trim(), "[]", "{families}");
}

/// An empty index is not a failure.
#[test]
fn an_empty_index_exits_zero() {
    let cli = empty("arrival-exit");
    for args in [vec!["list"], vec!["families"], vec!["facets"], vec![]] {
        // `ok` asserts success, so this is the assertion.
        cli.ok(&args);
    }
}

// ── 0082: a first scan says what it found ────────────────────────────────────

#[test]
fn a_first_scan_leads_with_the_library() {
    let cli = cli!("arrival-first-scan");
    let fixtures = cli.fixtures();
    let out = cli.ok(&["scan", &fixtures.to_string_lossy()]);

    let first = out.lines().next().unwrap_or_default();
    assert!(
        first.contains("6 faces in 5 families"),
        "the first line should be what was found, not what was done:\n{out}"
    );
    assert!(out.contains("free"), "{out}");
    // The ledger is still there; it is second, not gone.
    assert!(out.contains("scanned 6 candidates"), "{out}");
}

/// A rescan is a different question and keeps the ledger alone.
#[test]
fn a_rescan_is_just_the_ledger() {
    let cli = cli!("arrival-rescan");
    let fixtures = cli.fixtures();
    cli.ok(&["scan", &fixtures.to_string_lossy()]);

    let again = cli.ok(&["scan", &fixtures.to_string_lossy()]);
    let first = again.lines().next().unwrap_or_default();
    assert!(
        first.contains("scanned"),
        "a rescan opens with the ledger:\n{again}"
    );
    assert!(
        !again.contains("faces in"),
        "and does not repeat the summary:\n{again}"
    );
    assert!(again.contains("6 unchanged"), "{again}");
}

/// The scan report a machine reads is untouched.
#[test]
fn the_scan_json_is_unchanged() {
    let cli = cli!("arrival-scan-json");
    let fixtures = cli.fixtures();
    let out = cli.ok(&["scan", &fixtures.to_string_lossy(), "--json"]);

    let parsed: serde_json::Value = serde_json::from_str(&out).expect("the report is JSON");
    assert_eq!(parsed["faces"], 6, "{out}");
    assert!(
        !out.contains("largest"),
        "no prose leaked into the report:\n{out}"
    );
}
