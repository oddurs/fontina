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

//! What each role looks like: one scheme, two renderers, one place to change it.
//!
//! The six roles had two definitions. `term::sgr` said `Role::Accent => "36"` for the
//! command line and `ui::Theme::accent` said `Color::Cyan` for the browser: the same
//! decision, written twice, in two files, in two vocabularies. Nothing made them agree
//! and nothing would have noticed if they stopped.
//!
//! So the decision lives here once, and both of them read it. That is also what makes
//! the scheme themable at all — a setting can only override one thing, and until now
//! there were two.
//!
//! # The sixteen, and why not more
//!
//! A role takes one of the original sixteen ANSI colours or none, at every depth,
//! including on a terminal that could show sixteen million. That is a deliberate limit
//! rather than a missing feature, and it is what makes theming work rather than what
//! stops it: the sixteen are the colours the reader has already themed their terminal
//! to look right, so `accent = "cyan"` means *their* cyan. A hex value chosen here
//! would be the one colour on the screen that ignores the theme they picked.
//!
//! This is also why there is no `background`. A role paints its own text and nothing
//! else; the ground is the reader's.
//!
//! # Colour carries hierarchy, never meaning
//!
//! The rule the rest of the program follows holds here too, and it is enforced rather
//! than hoped for: every role must remain distinguishable with no colour at all, so a
//! role that takes no colour has to take a modifier instead. [`Scheme::check`] is that
//! rule, and it is why a scheme that paints everything the same colour is refused
//! rather than accepted and quietly unreadable.

use std::fmt;
use std::str::FromStr;

/// What each colour is *for*. Named by the job rather than by the hue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

impl Role {
    /// Every role, in the order a person would want them listed.
    pub const ALL: [Role; 6] = [
        Role::Head,
        Role::Dim,
        Role::Accent,
        Role::Good,
        Role::Warn,
        Role::Bad,
    ];

    /// The `colours.<role>` key, as `fontina config` prints it.
    pub const fn setting_key(self) -> &'static str {
        match self {
            Role::Head => "colours.head",
            Role::Dim => "colours.dim",
            Role::Accent => "colours.accent",
            Role::Good => "colours.good",
            Role::Warn => "colours.warn",
            Role::Bad => "colours.bad",
        }
    }

    /// The name this role has in a configuration file.
    pub const fn key(self) -> &'static str {
        match self {
            Role::Head => "head",
            Role::Dim => "dim",
            Role::Accent => "accent",
            Role::Good => "good",
            Role::Warn => "warn",
            Role::Bad => "bad",
        }
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.key())
    }
}

/// One of the original sixteen, by name.
///
/// The names are the ones every other terminal program uses, and the bright half is
/// spelled `bright-<colour>` rather than as eight more words, because `bright-black`
/// says what it is and `gray` starts an argument about spelling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ansi {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
}

impl Ansi {
    /// The SGR foreground parameter: 30–37 for the eight, 90–97 for the bright eight.
    pub const fn sgr(self) -> u8 {
        match self {
            Ansi::Black => 30,
            Ansi::Red => 31,
            Ansi::Green => 32,
            Ansi::Yellow => 33,
            Ansi::Blue => 34,
            Ansi::Magenta => 35,
            Ansi::Cyan => 36,
            Ansi::White => 37,
            Ansi::BrightBlack => 90,
            Ansi::BrightRed => 91,
            Ansi::BrightGreen => 92,
            Ansi::BrightYellow => 93,
            Ansi::BrightBlue => 94,
            Ansi::BrightMagenta => 95,
            Ansi::BrightCyan => 96,
            Ansi::BrightWhite => 97,
        }
    }

    /// The name a configuration file uses.
    pub const fn name(self) -> &'static str {
        match self {
            Ansi::Black => "black",
            Ansi::Red => "red",
            Ansi::Green => "green",
            Ansi::Yellow => "yellow",
            Ansi::Blue => "blue",
            Ansi::Magenta => "magenta",
            Ansi::Cyan => "cyan",
            Ansi::White => "white",
            Ansi::BrightBlack => "bright-black",
            Ansi::BrightRed => "bright-red",
            Ansi::BrightGreen => "bright-green",
            Ansi::BrightYellow => "bright-yellow",
            Ansi::BrightBlue => "bright-blue",
            Ansi::BrightMagenta => "bright-magenta",
            Ansi::BrightCyan => "bright-cyan",
            Ansi::BrightWhite => "bright-white",
        }
    }

    /// Every colour, for an error message that lists what it would have accepted.
    pub const ALL: [Ansi; 16] = [
        Ansi::Black,
        Ansi::Red,
        Ansi::Green,
        Ansi::Yellow,
        Ansi::Blue,
        Ansi::Magenta,
        Ansi::Cyan,
        Ansi::White,
        Ansi::BrightBlack,
        Ansi::BrightRed,
        Ansi::BrightGreen,
        Ansi::BrightYellow,
        Ansi::BrightBlue,
        Ansi::BrightMagenta,
        Ansi::BrightCyan,
        Ansi::BrightWhite,
    ];
}

impl FromStr for Ansi {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ansi::ALL
            .into_iter()
            .find(|c| c.name() == s)
            .ok_or_else(|| ParseError::Colour(s.to_string()))
    }
}

/// How one role is painted: a colour, some modifiers, or both.
///
/// `bold` and `reverse` are here and `italic`, `underline` and `dim` are not. The first
/// two work everywhere; the others are ignored by enough terminals that offering them
/// would be offering a setting that silently does nothing on somebody's machine. `dim`
/// in particular is the one this program most wanted and most cannot have — see
/// `Role::Dim`, which uses bright black for exactly that reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Paint {
    pub colour: Option<Ansi>,
    pub bold: bool,
    pub reverse: bool,
}

impl Paint {
    /// A role painted in one colour and nothing else.
    pub const fn colour(c: Ansi) -> Paint {
        Paint {
            colour: Some(c),
            bold: false,
            reverse: false,
        }
    }

    /// A role that takes no colour: bold, and nothing else.
    pub const BOLD: Paint = Paint {
        colour: None,
        bold: true,
        reverse: false,
    };

    /// A role that takes nothing at all. Legal, and the reason [`Scheme::check`] exists.
    pub const PLAIN: Paint = Paint {
        colour: None,
        bold: false,
        reverse: false,
    };

    /// Whether this paints anything at all.
    pub const fn is_plain(self) -> bool {
        self.colour.is_none() && !self.bold && !self.reverse
    }

    /// The SGR parameters, `;`-joined, or the empty string for a paint that does
    /// nothing — which is what makes a painter print its text and no escape.
    pub fn sgr(self) -> String {
        let mut parts: Vec<String> = Vec::with_capacity(3);
        if self.bold {
            parts.push("1".to_string());
        }
        if self.reverse {
            parts.push("7".to_string());
        }
        if let Some(c) = self.colour {
            parts.push(c.sgr().to_string());
        }
        parts.join(";")
    }

    /// What this paint is called in a configuration file, which is what `fontina config`
    /// prints back. Round-trips: every value this produces parses to the same paint.
    pub fn name(self) -> String {
        let mut parts: Vec<&str> = Vec::with_capacity(3);
        if self.bold {
            parts.push("bold");
        }
        if self.reverse {
            parts.push("reverse");
        }
        if let Some(c) = self.colour {
            parts.push(c.name());
        }
        if parts.is_empty() {
            return "none".to_string();
        }
        parts.join(" ")
    }
}

impl FromStr for Paint {
    type Err = ParseError;

    /// `"cyan"`, `"bold"`, `"bold yellow"`, `"none"`.
    ///
    /// Order does not matter and repetition is not an error: a person writing
    /// `bold bold cyan` meant bold cyan. Two *colours* is an error, because
    /// `red green` has no reading that is not a mistake.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.is_empty() {
            return Err(ParseError::Empty);
        }
        if s == "none" {
            return Ok(Paint::PLAIN);
        }

        let mut paint = Paint::default();
        for word in s.split_whitespace() {
            match word {
                "bold" => paint.bold = true,
                "reverse" => paint.reverse = true,
                "none" => return Err(ParseError::NoneWithMore(s.to_string())),
                other => {
                    let colour: Ansi = other.parse()?;
                    if let Some(already) = paint.colour
                        && already != colour
                    {
                        return Err(ParseError::TwoColours(
                            already.name().to_string(),
                            colour.name().to_string(),
                        ));
                    }
                    paint.colour = Some(colour);
                }
            }
        }
        Ok(paint)
    }
}

/// What is wrong with a value in a `[colours]` table.
///
/// Every one of them names what it got and what it would have taken. A configuration
/// file is written by hand, so the error is the whole of the documentation somebody
/// gets at the moment they need it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    Empty,
    Colour(String),
    TwoColours(String, String),
    NoneWithMore(String),
    /// Every role ended up invisible, which no terminal can distinguish.
    AllPlain,
    /// Two roles that have to differ do not.
    Indistinct(Role, Role),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Empty => f.write_str("an empty colour; write `none` to mean no colour"),
            ParseError::Colour(got) => write!(
                f,
                "`{got}` is not a colour. Take one of: {}, or `bold`, `reverse`, `none`",
                Ansi::ALL
                    .iter()
                    .map(|c| c.name())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            ParseError::TwoColours(a, b) => {
                write!(f, "two colours, `{a}` and `{b}`; a role takes one")
            }
            ParseError::NoneWithMore(got) => {
                write!(f, "`{got}`: `none` is the whole value or it is not in it")
            }
            ParseError::AllPlain => f.write_str(
                "every role would be invisible; colour carries hierarchy here, so at \
                 least one role has to be distinguishable",
            ),
            ParseError::Indistinct(a, b) => write!(
                f,
                "`{a}` and `{b}` look identical, so nothing on the screen would tell \
                 them apart. Colour carries hierarchy here and never meaning, which \
                 only works when the hierarchy is visible."
            ),
        }
    }
}

impl std::error::Error for ParseError {}

/// The six roles, resolved.
///
/// [`Scheme::default`] is what the program shipped with before there was a scheme, so a
/// reader with no configuration sees exactly what they saw before.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scheme {
    head: Paint,
    dim: Paint,
    accent: Paint,
    good: Paint,
    warn: Paint,
    bad: Paint,
}

impl Scheme {
    /// What the program shipped with, as a `const` so that `Term::plain()` can stay one.
    ///
    /// These are the SGR parameters that used to be literals in `term::sgr`, and a
    /// change to any of them is a change to what every reader who has configured
    /// nothing sees.
    pub const SHIPPED: Scheme = Scheme {
        // Bold rather than a colour: a heading is structure, and the reader's own
        // foreground at full weight reads as one on every theme there is.
        head: Paint::BOLD,
        // Bright black rather than the faint attribute, which enough terminals ignore
        // that it is a coin toss whether a label looks quieter or identical.
        dim: Paint::colour(Ansi::BrightBlack),
        accent: Paint::colour(Ansi::Cyan),
        good: Paint::colour(Ansi::Green),
        warn: Paint::colour(Ansi::Yellow),
        bad: Paint::colour(Ansi::Red),
    };
}

impl Default for Scheme {
    fn default() -> Self {
        Scheme::SHIPPED
    }
}

impl Scheme {
    /// How a role is painted.
    pub fn paint(&self, role: Role) -> Paint {
        match role {
            Role::Head => self.head,
            Role::Dim => self.dim,
            Role::Accent => self.accent,
            Role::Good => self.good,
            Role::Warn => self.warn,
            Role::Bad => self.bad,
        }
    }

    /// Overrides one role, returning the scheme so a chain of them reads as one.
    #[must_use]
    pub fn with(mut self, role: Role, paint: Paint) -> Self {
        match role {
            Role::Head => self.head = paint,
            Role::Dim => self.dim = paint,
            Role::Accent => self.accent = paint,
            Role::Good => self.good = paint,
            Role::Warn => self.warn = paint,
            Role::Bad => self.bad = paint,
        }
        self
    }

    /// Whether this scheme can still be read.
    ///
    /// Two roles painted identically are two things the reader cannot tell apart, and
    /// the program's whole colour rule — hierarchy, never meaning — assumes they can.
    /// A file that sets `good` and `bad` to the same green is a file whose author has
    /// made a mistake, and saying so at startup is better than a table where a failing
    /// check is the same colour as a passing one.
    ///
    /// `head` is exempt from the pairwise test against nothing: it is deliberately
    /// plain-with-bold, and a scheme is allowed to make other roles plain too as long
    /// as they do not collide.
    ///
    /// # Errors
    ///
    /// Fails if every role is invisible, or if two roles are painted the same way.
    pub fn check(&self) -> Result<(), ParseError> {
        if Role::ALL.iter().all(|r| self.paint(*r).is_plain()) {
            return Err(ParseError::AllPlain);
        }
        for (i, a) in Role::ALL.iter().enumerate() {
            for b in &Role::ALL[i + 1..] {
                if self.paint(*a) == self.paint(*b) {
                    return Err(ParseError::Indistinct(*a, *b));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The scheme a reader with no configuration gets is the one the program had
    /// before it could be configured at all. These are the SGR parameters `term::sgr`
    /// returned as literals, and a change to any of them is a change to what every
    /// existing user sees.
    #[test]
    fn the_default_scheme_is_what_shipped() {
        let s = Scheme::default();
        assert_eq!(s.paint(Role::Head).sgr(), "1");
        assert_eq!(s.paint(Role::Dim).sgr(), "90");
        assert_eq!(s.paint(Role::Accent).sgr(), "36");
        assert_eq!(s.paint(Role::Good).sgr(), "32");
        assert_eq!(s.paint(Role::Warn).sgr(), "33");
        assert_eq!(s.paint(Role::Bad).sgr(), "31");
        s.check().expect("the shipped scheme is readable");
    }

    #[test]
    fn a_paint_round_trips_through_its_own_name() {
        let cases = [
            Paint::PLAIN,
            Paint::BOLD,
            Paint::colour(Ansi::Cyan),
            Paint {
                colour: Some(Ansi::BrightYellow),
                bold: true,
                reverse: false,
            },
            Paint {
                colour: Some(Ansi::Red),
                bold: true,
                reverse: true,
            },
        ];
        for paint in cases {
            let name = paint.name();
            assert_eq!(
                name.parse::<Paint>().expect(&name),
                paint,
                "`{name}` did not parse back to what printed it"
            );
        }
    }

    #[test]
    fn a_value_says_what_it_would_have_taken() {
        // The whole point of the message: a person editing TOML by hand gets the list.
        let err = "chartreuse".parse::<Paint>().unwrap_err();
        let said = err.to_string();
        assert!(said.contains("chartreuse"), "{said}");
        assert!(said.contains("bright-black"), "{said}");
        assert!(said.contains("none"), "{said}");

        assert_eq!(
            "red green".parse::<Paint>().unwrap_err(),
            ParseError::TwoColours("red".into(), "green".into())
        );
        assert_eq!("".parse::<Paint>().unwrap_err(), ParseError::Empty);
        assert!(matches!(
            "none cyan".parse::<Paint>().unwrap_err(),
            ParseError::NoneWithMore(_)
        ));
    }

    /// Order and repetition are a person's business, not the parser's.
    #[test]
    fn modifiers_may_arrive_in_any_order() {
        let want = Paint {
            colour: Some(Ansi::Cyan),
            bold: true,
            reverse: false,
        };
        for spelling in ["bold cyan", "cyan bold", "bold  cyan", "bold bold cyan"] {
            assert_eq!(
                spelling.parse::<Paint>().expect(spelling),
                want,
                "{spelling}"
            );
        }
    }

    /// A scheme nobody could read is refused, and the message says which two roles.
    #[test]
    fn two_roles_that_look_the_same_are_refused() {
        let same = Scheme::default().with(Role::Bad, Paint::colour(Ansi::Green));
        let err = same.check().unwrap_err();
        assert_eq!(err, ParseError::Indistinct(Role::Good, Role::Bad));
        let said = err.to_string();
        assert!(said.contains("good") && said.contains("bad"), "{said}");

        let blank = Role::ALL
            .iter()
            .fold(Scheme::default(), |s, r| s.with(*r, Paint::PLAIN));
        assert_eq!(blank.check().unwrap_err(), ParseError::AllPlain);
    }

    /// A scheme that only moves one role is still a scheme, and still readable.
    #[test]
    fn changing_one_role_leaves_the_rest_alone() {
        let s = Scheme::default().with(Role::Accent, Paint::colour(Ansi::Magenta));
        assert_eq!(s.paint(Role::Accent), Paint::colour(Ansi::Magenta));
        assert_eq!(s.paint(Role::Bad), Scheme::default().paint(Role::Bad));
        s.check().expect("one changed role is still readable");
    }
}
