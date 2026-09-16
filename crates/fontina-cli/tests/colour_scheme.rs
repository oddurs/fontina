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

//! A colour scheme inherits: the file changes what it names and nothing else.
//!
//! The unit tests in `scheme` cover the vocabulary. These cover the chain — that a
//! value written in a file reaches the escape on the wire, that the roles it does not
//! name keep what they had, and that `NO_COLOR` still outranks all of it.
//!
//! Escapes rather than screenshots: what a terminal *does* with `\x1b[36m` is its
//! business, and asserting on the byte is asserting on the only part that is ours.

use fontina_testkit::cli;
use std::collections::BTreeSet;

/// Every distinct SGR escape in some output.
fn escapes(out: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let mut rest = out;
    while let Some(at) = rest.find("\u{1b}[") {
        rest = &rest[at + 2..];
        if let Some(end) = rest.find('m') {
            found.insert(rest[..end].to_string());
            rest = &rest[end + 1..];
        }
    }
    // The reset is not a colour anybody chose.
    found.remove("0");
    found
}

/// The same text with every SGR escape removed.
///
/// `fontina config` paints its own key column, so a line does not start with the key it
/// prints — it starts with `\x1b[90m`. A test that matched on `starts_with` without
/// this found nothing, and one of these tests passed for that reason before it was
/// noticed: it round-tripped an empty table and asserted that nothing equalled nothing.
fn plain(out: &str) -> String {
    let mut clean = String::with_capacity(out.len());
    let mut rest = out;
    while let Some(at) = rest.find("\u{1b}[") {
        clean.push_str(&rest[..at]);
        rest = &rest[at + 2..];
        match rest.find('m') {
            Some(end) => rest = &rest[end + 1..],
            None => break,
        }
    }
    clean.push_str(rest);
    clean
}

/// A sandbox with the fixtures scanned and colour forced on.
///
/// Forced because a test has no terminal, and without it every assertion here would be
/// about the plain bytes a pipe gets — which is the right output, and not the one these
/// tests are about.
fn painted(name: &str) -> fontina_testkit::Cli {
    let cli = cli!(name).with_env("CLICOLOR_FORCE", "1");
    let fixtures = cli.fixtures();
    cli.ok(&["scan", &fixtures.to_string_lossy()]);
    cli
}

/// Writes a configuration file and points the program at it.
///
/// The sandbox clears `FONTINA_CONFIG` so a developer's own file cannot reach a test,
/// so a test that wants one has to put it back deliberately. Returns a new `Cli`
/// because the variable is fixed at construction.
fn with_config(cli: fontina_testkit::Cli, toml: &str) -> fontina_testkit::Cli {
    let path = cli.root().join("config.toml");
    std::fs::write(&path, toml).expect("writing the config");
    cli.with_env("FONTINA_CONFIG", &path.to_string_lossy())
}

/// What the program shipped with, which is what somebody who has configured nothing
/// must keep seeing. These are the literals that used to live in `term::sgr`.
#[test]
fn a_reader_with_no_configuration_sees_what_shipped() {
    let cli = painted("scheme-default");
    let seen = escapes(&cli.ok(&["list"]));

    assert!(seen.contains("1"), "head is bold: {seen:?}");
    assert!(seen.contains("90"), "dim is bright black: {seen:?}");
    assert!(seen.contains("36"), "accent is cyan: {seen:?}");
    assert!(seen.contains("32"), "good is green: {seen:?}");
}

/// The whole of "inheritance": naming one role moves that role, and no other.
#[test]
fn a_file_changes_the_roles_it_names_and_leaves_the_rest() {
    let cli = with_config(
        painted("scheme-one-role"),
        "[colours]\naccent = \"magenta\"\n",
    );
    let seen = escapes(&cli.ok(&["list"]));

    assert!(seen.contains("35"), "accent moved to magenta: {seen:?}");
    assert!(
        !seen.contains("36"),
        "and nothing is cyan any more: {seen:?}"
    );

    // The four the file said nothing about are untouched. A scheme that replaced the
    // whole palette would have silently taken these with it.
    assert!(seen.contains("1"), "head kept bold: {seen:?}");
    assert!(seen.contains("90"), "dim kept bright black: {seen:?}");
    assert!(seen.contains("32"), "good kept green: {seen:?}");
}

/// Modifiers travel with the colour, in one escape.
#[test]
fn a_role_can_take_a_modifier_as_well_as_a_colour() {
    let cli = with_config(
        painted("scheme-modifier"),
        "[colours]\naccent = \"bold magenta\"\n",
    );
    let seen = escapes(&cli.ok(&["list"]));

    assert!(
        seen.contains("1;35"),
        "bold and magenta arrive as one escape: {seen:?}"
    );
}

/// `colors` is accepted for anybody who spells it that way.
#[test]
fn either_spelling_of_the_table_works() {
    let cli = with_config(
        painted("scheme-spelling"),
        "[colors]\naccent = \"magenta\"\n",
    );
    assert!(escapes(&cli.ok(&["list"])).contains("35"));
}

/// A person who said "no colour" said it about the ones they themselves named.
#[test]
fn no_colour_outranks_the_file() {
    let cli = with_config(
        painted("scheme-no-color"),
        "[colours]\naccent = \"bold magenta\"\n",
    )
    .with_env("NO_COLOR", "1");

    let out = cli.ok(&["list"]);
    assert!(
        escapes(&out).is_empty(),
        "an escape survived NO_COLOR: {:?}",
        escapes(&out)
    );
}

/// The message is the documentation somebody gets at the moment they need it.
#[test]
fn a_colour_that_is_not_a_colour_says_what_it_would_have_taken() {
    let cli = with_config(
        painted("scheme-bad-name"),
        "[colours]\naccent = \"chartreuse\"\n",
    );

    let err = cli.fails(&["list"]);
    assert!(err.contains("colours.accent"), "{err}");
    assert!(err.contains("chartreuse"), "{err}");
    assert!(err.contains("bright-black"), "the list is offered: {err}");
}

/// Colour carries hierarchy, which only works if the hierarchy is visible.
#[test]
fn two_roles_that_would_look_identical_are_refused() {
    let cli = with_config(painted("scheme-indistinct"), "[colours]\nbad = \"green\"\n");

    let err = cli.fails(&["list"]);
    assert!(err.contains("good"), "{err}");
    assert!(err.contains("bad"), "{err}");
}

/// `fontina config` says where every colour came from, which is the point of it.
#[test]
fn the_config_report_names_the_source_of_each_role() {
    let cli = with_config(
        painted("scheme-report"),
        "[colours]\naccent = \"magenta\"\n",
    );

    let out = plain(&cli.ok(&["config"]));
    let row = |key: &str| {
        out.lines()
            .find(|l| l.starts_with(key))
            .unwrap_or_else(|| panic!("no {key} row in:\n{out}"))
            .to_string()
    };

    let accent = row("colours.accent");
    assert!(
        accent.contains("magenta") && accent.contains("config"),
        "{accent}"
    );

    // A role nobody set reports the shipped value *and* says it is the default, so the
    // report distinguishes "I chose this" from "this is what it is".
    let good = row("colours.good");
    assert!(good.contains("green") && good.contains("default"), "{good}");
}

/// What `fontina config` prints can be pasted back into the file.
#[test]
fn the_report_prints_values_the_file_would_accept() {
    let cli = painted("scheme-round-trip");
    let out = plain(&cli.ok(&["config"]));

    let mut file = String::from("[colours]\n");
    let mut rows = 0;
    for line in out.lines().filter(|l| l.starts_with("colours.")) {
        rows += 1;
        let mut parts = line.split_whitespace();
        let key = parts.next().expect("a key").trim_start_matches("colours.");
        // Everything between the key and the trailing source word is the value.
        let rest: Vec<&str> = parts.collect();
        let (_, value) = rest.split_last().expect("a source");
        file.push_str(&format!("{key} = \"{}\"\n", value.join(" ")));
    }

    // Without this the test passes on an empty table, which is what it did before the
    // escapes were being stripped: six roles in, six roles out.
    assert_eq!(rows, 6, "every role should be reported:\n{out}");

    let back = with_config(painted("scheme-round-trip-2"), &file);
    // It parses, and it is the scheme it came from: the same escapes as the default.
    assert_eq!(
        escapes(&back.ok(&["list"])),
        escapes(&cli.ok(&["list"])),
        "the report did not round-trip:\n{file}"
    );
}
