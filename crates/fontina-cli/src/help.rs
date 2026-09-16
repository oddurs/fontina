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

//! The command list, grouped by what somebody is trying to do.
//!
//! Thirty-four commands, listed flat, in roughly the order they were written. `scan` and
//! `schema` sat at the same level; so did `activate`, which is what people came for, and
//! `completions`, which a shell install script runs once and nobody types again. A flat
//! list of thirty-four is a list nobody reads, and the ones that matter were buried among
//! the ones that do not.
//!
//! Nothing is removed and nothing is renamed here. The command line is the product and
//! the project's rule is that nothing is locked away from it, so this changes the
//! *presentation* and not the surface: every command still runs, spelled exactly as it
//! was.
//!
//! # Where the words come from
//!
//! Not from here. The heading and the order are this file's; the command names and their
//! one-line descriptions are read back out of the `clap::Command` that the derive built
//! from the doc comments in `main.rs`. A second copy of thirty-four descriptions is a
//! second copy that drifts, and this program has spent enough of its life fixing those.
//!
//! [`check`] is what keeps the grouping honest: it fails if a command exists that no
//! group names, so adding one without deciding where it belongs is a test failure rather
//! than a command nobody can find.

use clap::Command;

/// The groups, in the order a person meets them.
///
/// Read down and it is roughly a session: find a face, use it, decide whether it was the
/// right one, keep the decision, get it out. Set-up is near the bottom because it happens
/// once, and `Machinery` is last because it is for shells and scripts rather than people.
const GROUPS: [(&str, &[&str]); 6] = [
    (
        "Find",
        &[
            "list",
            "families",
            "counts",
            "stats",
            "info",
            "covers",
            "duplicates",
            "variants",
        ],
    ),
    (
        "Use",
        &[
            "activate",
            "deactivate",
            "install",
            "uninstall",
            "conflicts",
            "activations",
        ],
    ),
    ("Judge", &["check", "glyphs", "license"]),
    ("Keep", &["tag", "collection", "source"]),
    ("Export", &["css", "specimen", "preview"]),
    (
        "Set up",
        &["scan", "watch", "restore", "config", "dirs", "ui"],
    ),
];

/// Commands that belong to no group on purpose.
///
/// Every one of these is something a program runs rather than something a person types:
/// a shell completion script, a man page build, a JSON Schema dump, the agent protocol.
/// They stay in `--help` under their own heading rather than being hidden, because a
/// hidden command is one a script author cannot discover.
const MACHINERY: [&str; 5] = ["completions", "man", "schema", "agent", "help"];

/// The grouped list, rendered from the command clap built.
///
/// Padding is computed from the longest name rather than fixed, so a command added later
/// cannot push its own description out of line.
#[must_use]
pub fn groups(command: &Command) -> String {
    let names: Vec<&str> = command.get_subcommands().map(Command::get_name).collect();
    let width = names.iter().map(|n| n.len()).max().unwrap_or(12).max(10);

    // Four of indent, the name, two of gap. What is left is the room a description has,
    // and it is bounded so that a wide terminal does not produce a wide wall of text.
    let room = crate::term::term()
        .width()
        .unwrap_or(88)
        .saturating_sub(width + 6)
        .clamp(24, 72);

    let described: std::collections::BTreeMap<&str, String> = command
        .get_subcommands()
        .map(|sub| {
            let about = sub
                .get_about()
                .map(|a| one_line(&a.to_string(), room))
                .unwrap_or_default();
            (sub.get_name(), about)
        })
        .collect();
    let mut out = String::from("Commands:");

    for (heading, names) in GROUPS.iter().copied() {
        out.push_str(&format!("\n  {heading}\n"));
        for name in names {
            let about = described
                .get(name)
                .cloned()
                .unwrap_or_else(|| fallback_about(name));
            out.push_str(&format!("    {name:<width$}  {about}\n"));
        }
    }

    out.push_str("\n  Machinery\n");
    for name in MACHINERY {
        let about = described
            .get(name)
            .cloned()
            .unwrap_or_else(|| fallback_about(name));
        out.push_str(&format!("    {name:<width$}  {about}\n"));
    }

    out.push_str("\nRun `fontina help <command>` for one of them.");
    out
}

/// What a command says about itself when clap has not built one yet.
///
/// `help` is clap's own and is added during `build()`, after this renders — so it is
/// absent from `get_subcommands()` here and would print as a bare name. Its stock wording
/// is "Print this message or the help of the given subcommand(s)", which besides being a
/// mouthful carries the parenthesised plural this program has just spent a change
/// removing everywhere else.
fn fallback_about(name: &str) -> String {
    match name {
        "help" => "Print help for a command".to_string(),
        _ => String::new(),
    }
}

/// One line of description, cut to fit beside the name.
///
/// Two cuts, and both are needed. Several doc comments in `main.rs` run to a paragraph —
/// `activate` explains its exit codes — so the first sentence is taken; and several first
/// sentences are still eighty characters, so what is left is fitted to the room there is.
/// Without the second cut the overflow wrapped to column four and read as another command.
///
/// `unicode::fit` rather than a byte slice, because a description can hold a name with an
/// accent in it and cutting a string by bytes is how a program prints half a character.
fn one_line(about: &str, room: usize) -> String {
    let line = about.split('\n').next().unwrap_or(about).trim();
    let sentence = match line.split_once(". ") {
        Some((first, _)) => format!("{first}."),
        None => line.to_string(),
    };
    fontina_core::unicode::fit(&sentence, room)
}

/// Every command clap knows is in exactly one group.
///
/// Only the test calls it: the grouping is checked when the suite runs rather than on
/// every start-up, because a mistake here is a mistake in this file and not something a
/// reader can cause.
#[cfg(test)]
///
/// # Errors
///
/// Returns the names that are in no group, or in more than one. A command added to
/// `main.rs` and nowhere else would otherwise be a command that exists, runs, and appears
/// in no list a person reads.
pub fn check(command: &Command) -> Result<(), String> {
    let mut placed: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for name in GROUPS
        .iter()
        .flat_map(|(_, names)| names.iter())
        .chain(MACHINERY.iter())
    {
        *placed.entry(name).or_insert(0) += 1;
    }

    let known: std::collections::BTreeSet<&str> =
        command.get_subcommands().map(Command::get_name).collect();

    let ungrouped: Vec<&str> = known
        .iter()
        .copied()
        .filter(|n| !placed.contains_key(n))
        .collect();
    // `help` is grouped and is not in `get_subcommands()` at this point: clap adds it
    // during `build()`, after both this and the rendering run. Excluding it here is the
    // one exemption, and it is narrow enough to name.
    let unknown: Vec<&str> = placed
        .keys()
        .copied()
        .filter(|n| !known.contains(n) && *n != "help")
        .collect();
    let twice: Vec<&str> = placed
        .iter()
        .filter(|(_, count)| **count > 1)
        .map(|(name, _)| *name)
        .collect();

    let mut wrong = Vec::new();
    if !ungrouped.is_empty() {
        wrong.push(format!(
            "in no group: {} — add each to GROUPS in help.rs",
            ungrouped.join(", ")
        ));
    }
    if !unknown.is_empty() {
        wrong.push(format!(
            "grouped but not a command: {} — renamed or removed?",
            unknown.join(", ")
        ));
    }
    if !twice.is_empty() {
        wrong.push(format!("in more than one group: {}", twice.join(", ")));
    }

    if wrong.is_empty() {
        Ok(())
    } else {
        Err(wrong.join("; "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    /// The assertion that makes the grouping maintainable rather than a second list to
    /// forget. A command added to `main.rs` without a home here fails this.
    #[test]
    fn every_command_is_in_exactly_one_group() {
        check(&crate::Cli::command()).expect("every command has a group");
    }

    /// The rendered list names every command, so nothing is grouped out of sight.
    #[test]
    fn the_rendered_list_holds_every_command() {
        let command = crate::Cli::command();
        let rendered = groups(&command);
        for sub in command.get_subcommands() {
            assert!(
                rendered.contains(sub.get_name()),
                "`{}` is missing from the grouped list:\n{rendered}",
                sub.get_name()
            );
        }
    }

    /// A paragraph in a doc comment does not become a paragraph in the list.
    #[test]
    fn a_description_is_cut_to_one_line() {
        assert_eq!(one_line("One thing. And then another.", 40), "One thing.");
        assert_eq!(one_line("Just the one", 40), "Just the one");
        // A trailing period with no following sentence is left alone rather than
        // doubled.
        assert_eq!(one_line("Just the one.", 40), "Just the one.");
        assert_eq!(one_line("First line\nsecond line", 40), "First line");
    }

    /// The second cut: a first sentence longer than the room it has.
    ///
    /// This is the one that was missing. Without it the overflow wrapped to column four
    /// and every long description looked like two more commands.
    #[test]
    fn a_long_sentence_is_fitted_to_the_room_there_is() {
        let long = "Count faces per weight, width, style, script, license, vendor, tag \
                    and collection, for the faces matching the filters";
        let cut = one_line(long, 30);
        assert!(
            fontina_core::unicode::columns(&cut) <= 30,
            "{cut:?} is {} columns",
            fontina_core::unicode::columns(&cut)
        );
        assert!(!cut.contains('\n'));
    }

    /// Every line of the rendered list is one line.
    #[test]
    fn no_entry_wraps() {
        let command = crate::Cli::command();
        for line in groups(&command).lines() {
            assert!(
                fontina_core::unicode::columns(line) <= 110,
                "this line would wrap in a normal terminal:\n{line}"
            );
        }
    }
}
