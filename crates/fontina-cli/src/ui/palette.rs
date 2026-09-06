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

//! Every command the program has, listed where the reader already is.
//!
//! The browser reaches perhaps a third of what the command line can do. The rest is
//! learned from the manual and typed in another window, and the two surfaces drift
//! because nothing forces them together. `:` opens this: every subcommand `fontina
//! --help` lists, with the same one-line description clap prints, filtered as you type.
//!
//! The list is walked out of clap's own command tree rather than written down, so a new
//! subcommand appears here without anyone remembering to add it. What is written down
//! is the other half — whether the browser can *do* each one — and that is the point of
//! the table below.
//!
//! Not everything can be run from in here, and pretending otherwise would be the drift
//! this is meant to stop. A command that prints wants the screen, and the browser is
//! already using it; `list`, `info`, `dupes` and their kind are shown with the exact
//! command line for what is on the screen, ready to be run in another window. The ones
//! the browser does implement, it runs, by pressing its own key.
//!
//! So the table has an entry for every command, saying which of those it is, and
//! [`tests::every_command_is_accounted_for`] fails the build when a new subcommand
//! appears without anyone deciding. That is the forcing function the item asked for:
//! the two surfaces cannot drift quietly, only loudly.

use clap::CommandFactory;
use ratatui::crossterm::event::KeyCode;

/// How the browser reaches a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reach {
    /// A key the browser already answers to. The palette presses it.
    Key(char),
    /// The same, but it changes files on disk, so it is asked about first.
    Ask(char),
    /// The browser can say what the command would be, but not run it: it prints, and
    /// the browser owns the screen while it is up.
    Line,
    /// Nothing the browser has anything to do with — shell completions, man pages,
    /// the schema dump. Listed so the reader knows they exist.
    Elsewhere,
}

/// Every command, and what the browser can do about it.
///
/// Written down deliberately. The *names* come from clap so they cannot go stale, and
/// this says the one thing clap cannot: whether the browser has an answer for it.
const REACH: &[(&str, Reach)] = &[
    ("scan", Reach::Key('R')),
    ("list", Reach::Line),
    ("families", Reach::Line),
    ("facets", Reach::Line),
    ("tag", Reach::Line),
    ("tag list", Reach::Line),
    ("tag add", Reach::Key('t')),
    ("tag remove", Reach::Line),
    ("tag rename", Reach::Line),
    ("tag delete", Reach::Line),
    ("tag sync", Reach::Line),
    ("collection", Reach::Line),
    ("collection list", Reach::Line),
    ("collection create", Reach::Key('c')),
    ("collection delete", Reach::Line),
    ("collection rename", Reach::Line),
    ("collection add", Reach::Key('c')),
    ("collection remove", Reach::Line),
    ("collection show", Reach::Line),
    ("collection export", Reach::Line),
    ("collection import", Reach::Line),
    ("source", Reach::Line),
    ("source list", Reach::Line),
    ("source add", Reach::Line),
    ("source remove", Reach::Line),
    ("source watch", Reach::Line),
    ("activate", Reach::Key('a')),
    ("deactivate", Reach::Key('d')),
    ("install", Reach::Ask('i')),
    ("uninstall", Reach::Ask('u')),
    ("conflicts", Reach::Line),
    ("activations", Reach::Line),
    ("restore", Reach::Line),
    ("agent", Reach::Line),
    ("agent install", Reach::Line),
    ("agent uninstall", Reach::Line),
    ("agent status", Reach::Line),
    ("watch", Reach::Line),
    ("info", Reach::Line),
    ("dupes", Reach::Line),
    ("variants", Reach::Line),
    ("css", Reach::Line),
    ("stats", Reach::Line),
    ("dirs", Reach::Line),
    ("check", Reach::Line),
    ("covers", Reach::Line),
    ("glyphs", Reach::Key('m')),
    ("license", Reach::Line),
    ("specimen", Reach::Ask('s')),
    ("preview", Reach::Key('w')),
    ("ui", Reach::Elsewhere),
    ("completions", Reach::Elsewhere),
    ("man", Reach::Elsewhere),
    ("config", Reach::Line),
    ("schema", Reach::Elsewhere),
];

/// One command, as the palette shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// `tag add`, the way a reader would type it.
    pub path: String,
    /// The first line of clap's own description, which is the line `fontina --help`
    /// prints. Taken from clap so it cannot say something the command line does not.
    pub about: String,
    pub reach: Reach,
}

impl Entry {
    /// What the reader can do with it, in a few words at the end of the row.
    pub fn hint(&self) -> &'static str {
        match self.reach {
            Reach::Key(_) => "⏎ runs it",
            Reach::Ask(_) => "⏎ asks, then runs it",
            Reach::Line => "⏎ writes the command",
            Reach::Elsewhere => "command line only",
        }
    }
}

/// Every command clap knows about, in the order `--help` lists them.
pub fn entries() -> Vec<Entry> {
    let mut out = Vec::new();
    walk(&crate::Cli::command(), &mut Vec::new(), &mut out);
    out
}

fn walk(cmd: &clap::Command, path: &mut Vec<String>, out: &mut Vec<Entry>) {
    for sub in cmd.get_subcommands() {
        if sub.is_hide_set() {
            continue;
        }
        path.push(sub.get_name().to_string());
        let full = path.join(" ");
        // The first line only: clap's `about` is one line and `long_about` is the
        // paragraph, and a palette row is a row.
        let about = sub
            .get_about()
            .map(|a| a.to_string())
            .unwrap_or_default()
            .lines()
            .next()
            .unwrap_or_default()
            .to_string();
        let reach = REACH
            .iter()
            .find(|(name, _)| *name == full)
            .map(|(_, r)| *r)
            // A command nobody has decided about is shown rather than hidden, and the
            // test below is what makes sure nobody has to see this.
            .unwrap_or(Reach::Elsewhere);
        out.push(Entry {
            path: full,
            about,
            reach,
        });
        walk(sub, path, out);
        path.pop();
    }
}

/// The palette, while it is open.
pub struct Palette {
    all: Vec<Entry>,
    pub query: String,
    cursor: usize,
    /// The command waiting for a yes, when it is one that changes files.
    pub confirming: Option<String>,
}

impl Palette {
    pub fn new() -> Palette {
        Palette {
            all: entries(),
            query: String::new(),
            cursor: 0,
            confirming: None,
        }
    }

    /// The commands matching what has been typed, best first.
    ///
    /// Ranked, and it has to be. A flat list of everything a subsequence touches puts
    /// `scan` above `info` for the query `info`, because s-c-a-n's description happens
    /// to contain those four letters in that order — and then Enter runs the wrong
    /// thing. So a name that starts with what was typed comes before a name that
    /// merely contains it, which comes before one the letters are only scattered
    /// through, which comes before a match in the description.
    ///
    /// The sort is stable and the ranks are coarse, so inside a rank the order is
    /// clap's own and does not move under the reader as they type.
    pub fn matches(&self) -> Vec<&Entry> {
        let mut hits: Vec<(u8, &Entry)> = self
            .all
            .iter()
            .filter_map(|e| rank(&self.query, e).map(|r| (r, e)))
            .collect();
        hits.sort_by_key(|(r, _)| *r);
        hits.into_iter().map(|(_, e)| e).collect()
    }

    pub fn selected(&self) -> Option<&Entry> {
        let m = self.matches();
        m.get(self.cursor.min(m.len().saturating_sub(1))).copied()
    }

    /// How many commands there are altogether, for the title.
    pub fn total(&self) -> usize {
        self.all.len()
    }

    /// Where the cursor is among what is shown, for drawing.
    pub fn cursor(&self) -> usize {
        self.cursor.min(self.matches().len().saturating_sub(1))
    }

    pub fn move_cursor(&mut self, delta: i32) {
        let len = self.matches().len();
        if len == 0 {
            self.cursor = 0;
            return;
        }
        let last = len as i32 - 1;
        self.cursor = (self.cursor.min(len - 1) as i32 + delta).clamp(0, last) as usize;
    }

    pub fn type_char(&mut self, c: char) {
        self.query.push(c);
        // Back to the top: the list under the cursor is a different list now, and
        // leaving the cursor where it was would select something nobody looked at.
        self.cursor = 0;
    }

    pub fn backspace(&mut self) {
        self.query.pop();
        self.cursor = 0;
    }
}

/// How well a command answers a query: lower is better, `None` is not a match.
fn rank(query: &str, entry: &Entry) -> Option<u8> {
    if query.is_empty() {
        return Some(0);
    }
    let q = query.to_ascii_lowercase();
    let path = entry.path.to_ascii_lowercase();
    let about = entry.about.to_ascii_lowercase();
    // The last word too, so `add` finds `tag add` and `collection add` above anything
    // whose description merely mentions adding.
    let last = path.rsplit(' ').next().unwrap_or(&path);
    if path == q {
        Some(0)
    } else if path.starts_with(&q) || last.starts_with(&q) {
        Some(1)
    } else if path.contains(&q) {
        Some(2)
    } else if subsequence(&q, &path) {
        Some(3)
    } else if about.contains(&q) {
        Some(4)
    } else if subsequence(&q, &about) {
        Some(5)
    } else {
        None
    }
}

/// Whether every character of `needle` appears in `haystack`, in order.
///
/// Case-insensitive on ASCII, which is what command names are; a query with an accent
/// in it will simply not match, which is the right answer for a list of commands that
/// have none.
fn subsequence(needle: &str, haystack: &str) -> bool {
    let mut hay = haystack.chars().map(|c| c.to_ascii_lowercase());
    needle
        .chars()
        .map(|c| c.to_ascii_lowercase())
        .all(|c| hay.any(|h| h == c))
}

/// The key a reach presses, if it presses one.
pub fn key_for(reach: Reach) -> Option<KeyCode> {
    match reach {
        Reach::Key(c) | Reach::Ask(c) => Some(KeyCode::Char(c)),
        Reach::Line | Reach::Elsewhere => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// The forcing function. A new subcommand appears in the palette on its own,
    /// because the names come from clap; what it cannot do on its own is decide whether
    /// the browser has an answer for it. This fails until somebody does.
    #[test]
    fn every_command_is_accounted_for() {
        let known: BTreeSet<&str> = REACH.iter().map(|(n, _)| *n).collect();
        let found: BTreeSet<String> = entries().into_iter().map(|e| e.path).collect();

        let missing: Vec<&String> = found
            .iter()
            .filter(|p| !known.contains(p.as_str()))
            .collect();
        assert!(
            missing.is_empty(),
            "these commands exist and the palette has not been told what to do with \
             them: {missing:?}. Add a row to REACH saying whether the browser runs it, \
             writes it, or leaves it to the command line."
        );

        let stale: Vec<&&str> = known
            .iter()
            .filter(|n| !found.contains(**n))
            .collect::<Vec<_>>();
        assert!(
            stale.is_empty(),
            "REACH names commands that no longer exist: {stale:?}"
        );
    }

    /// The descriptions are clap's, not a second copy of them.
    #[test]
    fn the_help_is_the_help_the_command_line_prints() {
        let all = entries();
        let scan = all.iter().find(|e| e.path == "scan").expect("scan exists");
        let from_clap = crate::Cli::command()
            .get_subcommands()
            .find(|c| c.get_name() == "scan")
            .and_then(|c| c.get_about().map(|a| a.to_string()))
            .unwrap();
        assert_eq!(scan.about, from_clap.lines().next().unwrap());
        assert!(
            all.iter().all(|e| !e.about.contains('\n')),
            "a palette row is a row"
        );
    }

    /// Nested commands are reachable by the name a reader would type.
    #[test]
    fn a_subcommand_of_a_subcommand_is_listed_under_both_words() {
        let all = entries();
        assert!(all.iter().any(|e| e.path == "tag add"), "{all:?}");
        assert!(all.iter().any(|e| e.path == "collection export"));
        assert!(all.iter().any(|e| e.path == "agent status"));
    }

    #[test]
    fn typing_narrows_the_list_by_subsequence() {
        let mut p = Palette::new();
        let all = p.matches().len();
        assert!(all > 40, "there are {all} commands");

        for c in "tgad".chars() {
            p.type_char(c);
        }
        let paths: Vec<&str> = p.matches().iter().map(|e| e.path.as_str()).collect();
        assert!(paths.contains(&"tag add"), "{paths:?}");

        // And backspacing widens it again.
        for _ in 0..4 {
            p.backspace();
        }
        assert_eq!(p.matches().len(), all);
    }

    /// The bug a flat subsequence list has, and the reason there is a ranking: for the
    /// query `info` a plain subsequence puts `scan` first, because its description
    /// happens to hold i-n-f-o in that order, and then Enter runs the wrong command.
    #[test]
    fn what_the_reader_typed_is_the_first_thing_on_the_list() {
        for (query, want) in [
            ("info", "info"),
            ("install", "install"),
            ("act", "activate"),
            ("scan", "scan"),
            ("license", "license"),
            ("specimen", "specimen"),
            ("tag add", "tag add"),
            ("glyphs", "glyphs"),
        ] {
            let mut p = Palette::new();
            for c in query.chars() {
                p.type_char(c);
            }
            assert_eq!(
                p.selected().map(|e| e.path.as_str()),
                Some(want),
                "{query:?} put {:?} first",
                p.matches()
                    .iter()
                    .map(|e| &e.path)
                    .take(3)
                    .collect::<Vec<_>>()
            );
        }
    }

    /// A narrowed list is a different list, so the cursor goes back to the top rather
    /// than staying on a row that has moved out from under it.
    #[test]
    fn narrowing_puts_the_cursor_back_on_the_first_match() {
        let mut p = Palette::new();
        p.move_cursor(9);
        assert_eq!(p.cursor(), 9);
        p.type_char('z');
        assert_eq!(p.cursor(), 0);
    }

    /// A query that matches nothing selects nothing, rather than the last thing that
    /// did match.
    #[test]
    fn a_query_matching_nothing_has_nothing_selected() {
        let mut p = Palette::new();
        for c in "zzzzz".chars() {
            p.type_char(c);
        }
        assert!(p.matches().is_empty());
        assert!(p.selected().is_none());
        p.move_cursor(1);
        assert_eq!(p.cursor(), 0, "and moving in an empty list stays put");
    }

    /// Every command that changes files on disk is asked about first.
    #[test]
    fn what_writes_to_the_disk_asks_before_it_does() {
        for name in ["install", "uninstall", "specimen"] {
            let e = entries().into_iter().find(|e| e.path == name).unwrap();
            assert!(
                matches!(e.reach, Reach::Ask(_)),
                "{name} runs without asking"
            );
            assert_eq!(e.hint(), "⏎ asks, then runs it");
        }
        // And the ones that only read do not.
        let activate = entries()
            .into_iter()
            .find(|e| e.path == "activate")
            .unwrap();
        assert!(matches!(activate.reach, Reach::Key(_)));
    }

    #[test]
    fn a_reach_that_presses_a_key_names_one() {
        assert_eq!(key_for(Reach::Key('R')), Some(KeyCode::Char('R')));
        assert_eq!(key_for(Reach::Ask('i')), Some(KeyCode::Char('i')));
        assert_eq!(key_for(Reach::Line), None);
        assert_eq!(key_for(Reach::Elsewhere), None);
    }
}
