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

//! Three ISO 15924 codes are not scripts, and the command line says so last.
//!
//! `Zyyy` is Common — digits, punctuation, spaces. `Zinh` is Inherited — combining
//! marks. `Zzzz` is Unknown. They are in very nearly every font, so ranked by face
//! count they lead every list, and both places the command line prints scripts are
//! bounded: the `facets` row caps at 24 values and the `families` column fits four. On
//! a real library the pseudo-scripts were taking those slots from the scripts somebody
//! was looking for.
//!
//! The browser has sorted them last since M3. These tests are what was missing when the
//! command line did not: the whole suite passed both before the fix and after it.

use fontina_testkit::cli;

/// The order the `facets` script row prints in.
///
/// Parsed rather than asserted whole, because the counts move whenever a fixture is
/// added and a test that pins the whole line would fail for that instead of for this.
fn script_row(out: &str) -> Vec<String> {
    let line = out
        .lines()
        .find(|l| l.starts_with("script "))
        .unwrap_or_else(|| panic!("no script row in:\n{out}"));

    line.trim_start_matches("script")
        .split('·')
        .filter_map(|part| part.split_whitespace().next())
        .map(str::to_string)
        .collect()
}

/// Every pseudo-script comes after every real one.
fn assert_real_first(codes: &[String], whole: &str) {
    let last_real = codes
        .iter()
        .rposition(|c| !fontina_core::unicode::is_pseudo_script(c));
    let first_pseudo = codes
        .iter()
        .position(|c| fontina_core::unicode::is_pseudo_script(c));

    if let (Some(real), Some(pseudo)) = (last_real, first_pseudo) {
        assert!(
            pseudo > real,
            "`{}` is not a script and is printed before `{}`, which is:\n{whole}",
            codes[pseudo],
            codes[real],
        );
    }
}

/// A sandbox with the fixtures scanned. Amiri carries Arabic and Source Serif carries
/// Cyrillic and Greek, so there are real scripts for the pseudo ones to outrank.
fn scanned(name: &str) -> fontina_testkit::Cli {
    let cli = cli!(name);
    let fixtures = cli.fixtures();
    cli.ok(&["scan", &fixtures.to_string_lossy()]);
    cli
}

#[test]
fn the_facets_script_row_leads_with_real_scripts() {
    let cli = scanned("facets-scripts");
    let out = cli.ok(&["facets"]);
    let codes = script_row(&out);

    assert!(
        codes.iter().any(|c| c == "Latn"),
        "the fixtures cover Latin, so it should be listed:\n{out}"
    );
    assert!(
        codes.iter().any(|c| c == "Zyyy"),
        "Zyyy is in every fixture, and is listed rather than hidden:\n{out}"
    );
    assert_real_first(&codes, &out);
}

/// The row names them, too: `Zinh` alone is a code a reader has to go and look up.
#[test]
fn a_pseudo_script_is_given_its_word() {
    let cli = scanned("facets-words");
    let out = cli.ok(&["facets"]);
    let line = out
        .lines()
        .find(|l| l.starts_with("script "))
        .unwrap_or_else(|| panic!("no script row in:\n{out}"));

    assert!(line.contains("Zyyy common"), "{out}");
    assert!(line.contains("Zinh inherited"), "{out}");

    // A real script needs no gloss and does not get one: what follows `Latn` is its
    // count. Checked as "the next character is a digit" rather than as the absence of
    // a substring, because `Latn 6` contains `Latn ` and the obvious assertion passes
    // for the wrong reason.
    let after = line
        .split_once("Latn ")
        .map(|(_, rest)| rest)
        .unwrap_or_else(|| panic!("no Latn in:\n{out}"));
    assert!(
        after.starts_with(|c: char| c.is_ascii_digit()),
        "Latn was given a gloss it does not need:\n{out}"
    );
}

/// The `families` column fits four codes, so which four is the whole question.
#[test]
fn the_families_script_column_spends_its_four_slots_on_real_scripts() {
    let cli = scanned("families-scripts");
    let out = cli.ok(&["families"]);

    let header = out.lines().next().expect("a header row");
    let at = header.find("scripts").expect("a scripts column");

    let mut checked = 0;
    for row in out.lines().skip(1) {
        if row.trim().is_empty() || row.starts_with(char::is_numeric) {
            continue;
        }
        let codes: Vec<String> = row[at.min(row.len())..]
            .split_whitespace()
            .map(str::to_string)
            .collect();
        if codes.is_empty() {
            continue;
        }
        assert_real_first(&codes, &out);
        checked += 1;
    }
    assert!(checked > 0, "no family rows were checked:\n{out}");

    // Source Serif 4 covers Latin, Cyrillic and Greek — three real scripts, which is
    // more than fit beside two pseudo ones. Before the fix its row read
    // `Latn Cyrl Zyyy Zinh` and Greek fell off the end.
    let row = out
        .lines()
        .find(|l| l.contains("Source Serif"))
        .unwrap_or_else(|| panic!("no Source Serif row in:\n{out}"));
    assert!(
        row.contains("Grek"),
        "Greek is one of the face's three real scripts and should not lose its slot \
         to a pseudo-script:\n{out}"
    );
}
