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

//! What the terminal is: how much colour it can show, and how wide it is.
//!
//! Resolved once in `main` and read from there by every printer, because both facts are
//! about the process rather than about any one command, and because a program that asks
//! the environment twice can answer itself differently.
//!
//! Two rules hold everything else up.
//!
//! **A pipe gets the bytes it has always got.** No escape ever reaches something that is
//! not a terminal, and the columns are not re-fitted either, because a script that cuts
//! a field out of `fontina list` is a reader too. Colour and fitting are what a person
//! at a terminal gets in addition, never instead.
//!
//! **Colour carries hierarchy and never meaning.** It is the same rule the browser's
//! palette follows, and it is why every distinction here survives `NO_COLOR`: a failing
//! check says `FAIL`, and the red is how you find it, not how you know.

use std::fmt;
use std::io::IsTerminal;

/// What the terminal can show, in the order the checks run.
///
/// The default is the top of the range, not the bottom: a palette assumes the best
/// until something asks the terminal, and [`Depth::detect`] is that asking. Keeping
/// the asking out of `Default` is what makes a palette a value rather than a reading
/// of the environment, and it is not a theoretical distinction: a preview test that
/// built its palette with `default()` passed under a developer's terminal and failed
/// in CI, which has no `TERM` at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Depth {
    /// Twenty-four bit colour: `COLORTERM` says `truecolor` or `24bit`.
    #[default]
    True,
    /// The 256-colour cube and its greyscale ramp.
    Ansi256,
    /// The original sixteen, and nothing to interpolate with.
    Ansi16,
    /// None at all, by the reader's instruction.
    None,
}

impl Depth {
    /// Read the environment the way every other well-behaved terminal program does.
    ///
    /// `NO_COLOR` wins over everything, including a `COLORTERM` that promises the
    /// world: it is a person saying what they want, and the informal standard is that
    /// its mere presence counts, whatever it is set to.
    pub fn detect() -> Depth {
        Self::from_env(
            std::env::var_os("NO_COLOR").is_some(),
            std::env::var("COLORTERM").ok().as_deref(),
            std::env::var("TERM").ok().as_deref(),
        )
    }

    /// The decision itself, separated from the environment so it can be tested.
    pub fn from_env(no_color: bool, colorterm: Option<&str>, term: Option<&str>) -> Depth {
        if no_color {
            return Depth::None;
        }
        if matches!(colorterm, Some(c) if c.contains("truecolor") || c.contains("24bit")) {
            return Depth::True;
        }
        match term {
            Some(t) if t.contains("256color") => Depth::Ansi256,
            // `dumb` is a terminal saying it cannot do this, which is the same
            // instruction `NO_COLOR` gives, arriving from the other direction.
            Some("dumb") | None => Depth::None,
            Some(_) => Depth::Ansi16,
        }
    }
}

/// When to colour.
///
/// There is no flag for this. `--color` is already a filter here — `fontina list
/// --color` is the fonts that carry their own colour — and a program with two flags of
/// one name, or two spellings of one word, is worse off than one that reads the two
/// environment variables every other tool reads. `NO_COLOR` says never and
/// `CLICOLOR_FORCE` says always; between them, a terminal is coloured and a pipe is not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum When {
    /// Colour a terminal and nothing else. What everybody wants and nobody asks for.
    #[default]
    Auto,
    /// Colour whatever this is. `CLICOLOR_FORCE`, for a pager that understands escapes
    /// and for the tests, which have no terminal and still have to see the colours.
    Always,
    /// Never. `NO_COLOR`, which is a person speaking and beats everything.
    Never,
}

impl When {
    /// The two variables, in the order they win.
    pub fn detect() -> When {
        Self::from_env(
            std::env::var_os("NO_COLOR").is_some(),
            std::env::var_os("CLICOLOR_FORCE").is_some_and(|v| v != "0"),
        )
    }

    fn from_env(no_color: bool, force: bool) -> When {
        match (no_color, force) {
            (true, _) => When::Never,
            (false, true) => When::Always,
            (false, false) => When::Auto,
        }
    }
}

/// What each colour is *for*. Named by the job rather than by the hue, so that the one
/// place that decides what "a warning" looks like is this file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// A column heading, a section title: the words that are not the answer.
    Head,
    /// Present, and not what you are reading. Labels, units, the directory part of a
    /// path, a count at the end of a table.
    Dim,
    /// The thing being pointed at: an id, a tag, the number a command was run to find.
    Accent,
    /// Something that worked, or is free.
    Good,
    /// Something to notice but not to act on.
    Warn,
    /// Something that failed, or that is not free.
    Bad,
}

/// The terminal, resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Term {
    depth: Depth,
    /// Columns to print into, or `None` when there is no terminal to ask and nothing
    /// said otherwise — in which case a table is exactly as wide as its content.
    width: Option<usize>,
}

impl Default for Term {
    fn default() -> Self {
        Term::plain()
    }
}

/// Below this a terminal is not a terminal, it is a misreport. Fitting a table into
/// nineteen columns produces something worse than not fitting it at all.
const NARROWEST: usize = 40;

impl Term {
    /// A terminal with no colour and no width: what a pipe is, and what every printer
    /// falls back to when nothing has resolved one.
    pub const fn plain() -> Term {
        Term {
            depth: Depth::None,
            width: None,
        }
    }

    /// A terminal with these two facts, for the tests.
    #[cfg(test)]
    pub const fn new(depth: Depth, width: Option<usize>) -> Term {
        Term { depth, width }
    }

    /// Ask the environment, once, for output going to `stdout`.
    pub fn detect() -> Term {
        Term::resolve(
            When::detect(),
            std::io::stdout().is_terminal(),
            Depth::detect(),
            || {
                ratatui::crossterm::terminal::size()
                    .ok()
                    .map(|(cols, _)| cols as usize)
            },
        )
    }

    /// The decision, with the environment handed in so it can be tested.
    ///
    /// A width is only ever taken from a terminal. `COLUMNS` overrides what the
    /// terminal reports — it is how a person says how wide to print — but it does not
    /// give a pipe a width it does not have, the same way `ls | cat` prints one column
    /// however wide the window is. Everything downstream of a pipe can wrap, and none
    /// of it can un-truncate.
    fn resolve(
        when: When,
        tty: bool,
        detected: Depth,
        size: impl FnOnce() -> Option<usize>,
    ) -> Term {
        let depth = match when {
            When::Never => Depth::None,
            // "Always" has to mean always, including with no `TERM` at all, or the
            // flag would be a suggestion. Sixteen colours is the floor every terminal
            // that has any has.
            When::Always => match detected {
                Depth::None => Depth::Ansi16,
                d => d,
            },
            When::Auto if tty => detected,
            When::Auto => Depth::None,
        };
        let width = tty
            .then(|| {
                std::env::var("COLUMNS")
                    .ok()
                    .and_then(|c| c.trim().parse::<usize>().ok())
                    .or_else(size)
            })
            .flatten()
            .filter(|w| *w >= NARROWEST);
        Term { depth, width }
    }

    /// Columns to print into, when anything knows.
    pub fn width(&self) -> Option<usize> {
        self.width
    }

    /// `text`, in the colours of `role` — or exactly `text`, when there are none.
    ///
    /// Borrowing rather than allocating: a table of five thousand faces paints eight
    /// cells a row, and the point of the column helpers is that the string being
    /// painted is already the right width.
    pub fn paint<'a>(&self, role: Role, text: &'a str) -> Painted<'a> {
        Painted {
            sgr: self.sgr(role),
            text,
        }
    }

    pub fn head<'a>(&self, text: &'a str) -> Painted<'a> {
        self.paint(Role::Head, text)
    }
    pub fn dim<'a>(&self, text: &'a str) -> Painted<'a> {
        self.paint(Role::Dim, text)
    }
    pub fn accent<'a>(&self, text: &'a str) -> Painted<'a> {
        self.paint(Role::Accent, text)
    }
    pub fn good<'a>(&self, text: &'a str) -> Painted<'a> {
        self.paint(Role::Good, text)
    }
    pub fn warn<'a>(&self, text: &'a str) -> Painted<'a> {
        self.paint(Role::Warn, text)
    }
    pub fn bad<'a>(&self, text: &'a str) -> Painted<'a> {
        self.paint(Role::Bad, text)
    }

    /// The SGR parameters for a role, or the empty string for a terminal with no
    /// colour — which is what makes [`Painted`] print its text and nothing else.
    ///
    /// Sixteen colours and no more, at every depth. The palette does not need the cube:
    /// six roles map onto six of the original eight, they are the six a reader has
    /// themed their terminal to look right, and a 24-bit grey chosen here would be the
    /// one colour on the screen that ignores what they chose.
    fn sgr(&self, role: Role) -> &'static str {
        if self.depth == Depth::None {
            return "";
        }
        match role {
            Role::Head => "1",
            // Bright black rather than the faint attribute, which enough terminals
            // ignore that it is a coin toss whether a label looks quieter or identical.
            Role::Dim => "90",
            Role::Accent => "36",
            Role::Good => "32",
            Role::Warn => "33",
            Role::Bad => "31",
        }
    }
}

/// A string and the escape that colours it. `Display` so it goes straight into a
/// `write!` beside the columns it was measured against.
pub struct Painted<'a> {
    sgr: &'static str,
    text: &'a str,
}

impl fmt::Display for Painted<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Nothing to colour is nothing to wrap: an empty cell used to cost a pair of
        // escapes around no characters at all, which is bytes down the wire and a
        // surprise for anything counting them.
        if self.sgr.is_empty() || self.text.is_empty() {
            return f.write_str(self.text);
        }
        write!(f, "\x1b[{}m{}\x1b[0m", self.sgr, self.text)
    }
}

/// The terminal this process is printing to.
///
/// A global because it is one: both facts are about the process, every printer needs
/// them, and threading a parameter through fifteen printers and their forty call sites
/// would say something about the plumbing rather than about the program. Unset until
/// `main` resolves it, and [`Term::plain`] until then, so anything that prints on the
/// way to that — an argument error — prints plainly rather than not at all.
static TERM: std::sync::OnceLock<Term> = std::sync::OnceLock::new();

pub fn set(term: Term) {
    let _ = TERM.set(term);
}

pub fn term() -> Term {
    *TERM.get().unwrap_or(&Term::plain())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The regression this guards: `Depth::default()` used to call `detect()`, so a
    /// palette built anywhere inherited the environment. Every test that rendered got
    /// its colours from whatever terminal it ran under, which is how a preview test
    /// passed on a developer's machine and failed in CI, where there is no `TERM`.
    #[test]
    fn a_depth_is_a_value_and_never_a_reading_of_the_environment() {
        assert_eq!(Depth::default(), Depth::True);
    }

    #[test]
    fn the_depth_is_read_from_the_environment_in_order() {
        assert_eq!(
            Depth::from_env(true, Some("truecolor"), Some("xterm-256color")),
            Depth::None,
            "NO_COLOR is a person speaking and it wins"
        );
        assert_eq!(
            Depth::from_env(false, Some("truecolor"), Some("xterm")),
            Depth::True
        );
        assert_eq!(
            Depth::from_env(false, None, Some("xterm-256color")),
            Depth::Ansi256
        );
        assert_eq!(Depth::from_env(false, None, Some("xterm")), Depth::Ansi16);
        assert_eq!(Depth::from_env(false, None, Some("dumb")), Depth::None);
        assert_eq!(Depth::from_env(false, None, None), Depth::None);
    }

    /// The rule the whole module rests on: nothing that is not a terminal ever sees an
    /// escape, and nothing a terminal sees is different in kind from what a pipe gets.
    #[test]
    fn a_pipe_is_never_coloured_and_is_never_fitted() {
        let piped = Term::resolve(When::Auto, false, Depth::True, || Some(120));
        assert_eq!(piped.width(), None, "a pipe has no width to fit to");
        assert_eq!(piped.dim("path").to_string(), "path");
    }

    #[test]
    fn a_terminal_is_coloured_to_the_depth_it_reports() {
        let tty = Term::resolve(When::Auto, true, Depth::Ansi256, || Some(120));
        assert_eq!(tty.width(), Some(120));
        assert_eq!(tty.bad("no").to_string(), "\x1b[31mno\x1b[0m");
    }

    /// `NO_COLOR` has to beat a terminal that can, and `CLICOLOR_FORCE` a terminal that
    /// cannot — otherwise they are suggestions rather than instructions.
    #[test]
    fn the_environment_beats_the_terminal_in_both_directions() {
        assert_eq!(When::from_env(true, true), When::Never, "NO_COLOR wins");
        assert_eq!(When::from_env(false, true), When::Always);
        assert_eq!(When::from_env(false, false), When::Auto);
        assert_eq!(
            Term::resolve(When::Never, true, Depth::True, || Some(80))
                .bad("no")
                .to_string(),
            "no"
        );
        let forced = Term::resolve(When::Always, false, Depth::None, || None);
        assert_ne!(
            forced.bad("no").to_string(),
            "no",
            "--color always has to work where there is no TERM at all, which is where \
             the tests run"
        );
    }

    /// An empty cell is not worth an escape.
    #[test]
    fn nothing_is_painted_as_nothing() {
        assert_eq!(Term::new(Depth::True, None).dim("").to_string(), "");
    }

    /// Every role is a different colour, and every one of them disappears together.
    #[test]
    fn the_roles_are_distinct_and_they_all_go_at_once() {
        let roles = [
            Role::Head,
            Role::Dim,
            Role::Accent,
            Role::Good,
            Role::Warn,
            Role::Bad,
        ];
        let on = Term::new(Depth::Ansi16, None);
        let mut seen: Vec<&str> = roles.iter().map(|r| on.sgr(*r)).collect();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), roles.len(), "two roles share an escape");

        let off = Term::new(Depth::None, None);
        for role in roles {
            assert_eq!(off.paint(role, "x").to_string(), "x");
        }
    }

    /// A width nobody could use is no width at all: a table fitted into nineteen
    /// columns is worse than a table that overflows a terminal somebody has made tiny
    /// on purpose.
    #[test]
    fn a_terminal_too_narrow_to_fit_is_not_fitted() {
        assert_eq!(
            Term::resolve(When::Auto, true, Depth::True, || Some(12)).width(),
            None
        );
        assert_eq!(
            Term::resolve(When::Auto, true, Depth::True, || Some(NARROWEST)).width(),
            Some(NARROWEST)
        );
    }
}
