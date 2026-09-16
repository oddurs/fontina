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

//! An old name keeps working, for ever.
//!
//! `facets` is what a search index calls counting things and `dupes` is an abbreviation
//! of a word nobody abbreviates. Both are now the second spelling of a command whose
//! first spelling somebody could guess — and both still run, because the rule that a
//! script written against 1.0 runs against 3.0 is worth more than tidiness.
//!
//! That rule only holds if something checks it, which is what this is. Removing an alias
//! should take a deliberate act and a failing test, not a tidy-up.

use fontina_testkit::cli;

/// Every rename, as (what it is called now, what it was called before).
const RENAMED: [(&str, &str); 2] = [("counts", "facets"), ("duplicates", "dupes")];

fn scanned(name: &str) -> fontina_testkit::Cli {
    let cli = cli!(name);
    let fixtures = cli.fixtures();
    cli.ok(&["scan", &fixtures.to_string_lossy()]);
    cli
}

/// Both spellings run, and they are the same command.
#[test]
fn an_old_name_still_runs_and_does_the_same_thing() {
    let cli = scanned("names-aliases");

    for (now, before) in RENAMED {
        let new = cli.ok(&[now]);
        let old = cli.ok(&[before]);
        assert_eq!(
            new, old,
            "`fontina {before}` and `fontina {now}` should be one command"
        );
        assert!(!new.trim().is_empty(), "`fontina {now}` printed nothing");
    }
}

/// The new name is what `--help` leads with; the old one is not advertised.
///
/// An alias that appears in the help beside its own primary is two commands as far as a
/// reader is concerned, which is the opposite of the point.
#[test]
fn the_help_shows_the_name_worth_learning() {
    let cli = cli!("names-help");
    let out = cli.ok(&["--help"]);

    for (now, before) in RENAMED {
        assert!(out.contains(now), "`{now}` is missing from --help:\n{out}");
        // The old spelling may appear inside a description; what it must not do is stand
        // at the start of a line as a command in its own right.
        assert!(
            !out.lines()
                .any(|l| l.trim_start().starts_with(&format!("{before} "))),
            "`{before}` is still listed as a command of its own:\n{out}"
        );
    }
}

/// Every command in `--help` has a description.
///
/// `source list` had none, so it printed as a bare word in a column of explanations —
/// which is how a command ends up looking like a mistake in the help rather than a thing
/// somebody might want.
#[test]
fn no_command_is_listed_without_a_description() {
    let cli = cli!("names-described");

    for group in [
        vec!["--help"],
        vec!["help", "source"],
        vec!["help", "tag"],
        vec!["help", "collection"],
    ] {
        let out = cli.ok(&group);
        // The helper picks command lines by indentation, which is what tells a command
        // from a group heading and from a description clap has wrapped. An earlier
        // version of this test read every single-word line and reported `path` — the
        // tail of `import`'s wrapped description — as a command with no description.
        for name in fontina_testkit::commands_in_help(&out) {
            let line = out
                .lines()
                .find(|l| {
                    let t = l.trim_start();
                    t.starts_with(&format!("{name} ")) || t == name
                })
                .unwrap_or_default();
            assert!(
                line.split_whitespace().count() > 1,
                "`{name}` is listed with no description in `fontina {}`:\n{out}",
                group.join(" ")
            );
        }
    }
}
