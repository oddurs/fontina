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

//! One palette, resolved against what the terminal can actually show.
//!
//! Colour in the browser had accumulated a pane at a time: thirty-eight literals across
//! four files, truecolor in the preview with nothing behind it, and `NO_COLOR` honoured
//! nowhere. This is the whole palette, named by the job each colour does rather than by
//! what it looks like, and resolved once against the terminal's depth.
//!
//! Colour carries hierarchy. It never carries meaning on its own — the same rule the web
//! side follows — which is why every distinction here survives its own absence.
//!
//! # Where the colours come from
//!
//! Not from here. This file used to name them — `Color::Cyan` for the accent, and so on
//! — while `term::sgr` named the same six for the command line, in different words, with
//! nothing keeping the two in step. [`crate::scheme`] holds the decision now and this
//! turns it into ratatui styles. A reader who themes `accent` themes both.
//!
//! What stays here is the browser's own structure: a cursor is reversed and a title is
//! bold because of what they *are*, on any palette, and neither is in the scheme because
//! neither is a colour anybody should have to choose.

use crate::scheme::{Role, Scheme};
use ratatui::style::{Color, Modifier, Style};

/// What the terminal can show. Resolved for the whole process in [`crate::term`]; the
/// browser takes the same answer and turns it into ratatui styles.
pub use crate::term::Depth;

/// The palette, resolved once.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Theme {
    depth: Depth,
    scheme: Scheme,
}

impl Theme {
    /// The shipped scheme at this depth.
    ///
    /// Only the tests build one this way now — the browser goes through
    /// [`Theme::with_scheme`] so that it cannot end up painting a different scheme from
    /// the command line in the same process.
    #[cfg(test)]
    pub fn new(depth: Depth) -> Self {
        Theme {
            depth,
            scheme: Scheme::SHIPPED,
        }
    }

    /// This depth, painted by a reader's scheme.
    pub fn with_scheme(depth: Depth, scheme: Scheme) -> Self {
        Theme { depth, scheme }
    }

    /// One role, as a ratatui style.
    ///
    /// `Depth::None` outranks the scheme: somebody who set `NO_COLOR` said it about all
    /// of it. What survives is the modifiers, which is what keeps every role
    /// distinguishable when none of them has a colour.
    fn role(&self, role: Role) -> Style {
        let paint = self.scheme.paint(role);
        let mut style = Style::default();
        if paint.bold {
            style = style.add_modifier(Modifier::BOLD);
        }
        if paint.reverse {
            style = style.add_modifier(Modifier::REVERSED);
        }
        match (self.depth, paint.colour) {
            (Depth::None, _) | (_, None) => style,
            (_, Some(c)) => style.fg(ansi(c)),
        }
    }

    /// The pane you are in, the sheet that is open, the thing being pointed at.
    ///
    /// Bold when there is no colour, because a focused pane border that looks exactly
    /// like an unfocused one is the browser's single most important distinction gone.
    /// That is the rule stated in the module docs, applied: a role whose colour is
    /// taken away has to keep saying what it said.
    pub fn accent(&self) -> Style {
        match self.depth {
            Depth::None => Style::default().add_modifier(Modifier::BOLD),
            _ => self.role(Role::Accent),
        }
    }

    /// The one thing on the screen the reader is being pointed *at*: a codepoint a
    /// search just found, sitting in a grid of several hundred others.
    ///
    /// Reversed at every depth, and coloured by reversing the *foreground*: the cell
    /// takes the accent as its background and the terminal's own background as its
    /// text. Setting a literal black on cyan named a colour the reader had not chosen,
    /// and on a light theme it is the one cell on the screen that looks like a mistake.
    pub fn cursor(&self) -> Style {
        let base = Style::default().add_modifier(Modifier::REVERSED);
        match (self.depth, self.scheme.paint(Role::Accent).colour) {
            (Depth::None, _) | (_, None) => base,
            (_, Some(c)) => base.fg(ansi(c)),
        }
    }

    /// The row a list has its cursor on.
    ///
    /// Reversed in the pane that has the focus, and merely bold in the others — a
    /// reader who has tabbed away still needs to see where they were, and two reversed
    /// bars on one screen is two answers to "where am I". Reversed rather than a
    /// background colour so that the bar is the reader's own foreground and background,
    /// whatever they chose: nothing here paints over a theme.
    pub fn selection(&self, focused: bool) -> Style {
        if focused {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default().add_modifier(Modifier::BOLD)
        }
    }

    /// The name of a pane, on its border.
    pub fn title(&self) -> Style {
        Style::default().add_modifier(Modifier::BOLD)
    }

    /// Present, and not what you are reading: labels, keys, the row of hints.
    ///
    /// Under `NO_COLOR` this is deliberately plain rather than dim. There is no
    /// modifier that means "quieter" the way a grey does — `DIM` is unsupported often
    /// enough to be a coin toss — and a label in italics is louder than one in nothing.
    pub fn dim(&self) -> Style {
        self.role(Role::Dim)
    }

    /// Something the reader should notice but need not act on.
    pub fn warn(&self) -> Style {
        match self.depth {
            // Bold with no colour: a warning that looks like every other row is not a
            // warning.
            Depth::None => Style::default().add_modifier(Modifier::BOLD),
            _ => self.role(Role::Warn),
        }
    }

    /// Something that failed.
    pub fn bad(&self) -> Style {
        match self.depth {
            // Bold with no colour: a warning that looks like every other row is not a
            // warning.
            Depth::None => Style::default().add_modifier(Modifier::BOLD),
            _ => self.role(Role::Bad),
        }
    }

    /// Something that worked: an activation, a free licence.
    pub fn good(&self) -> Style {
        self.role(Role::Good)
    }
}

/// One of the sixteen, as ratatui spells it.
///
/// `DarkGray` is ratatui's name for bright black, which is the one place the two
/// vocabularies disagree.
fn ansi(c: crate::scheme::Ansi) -> Color {
    use crate::scheme::Ansi;
    match c {
        Ansi::Black => Color::Black,
        Ansi::Red => Color::Red,
        Ansi::Green => Color::Green,
        Ansi::Yellow => Color::Yellow,
        Ansi::Blue => Color::Blue,
        Ansi::Magenta => Color::Magenta,
        Ansi::Cyan => Color::Cyan,
        Ansi::White => Color::Gray,
        Ansi::BrightBlack => Color::DarkGray,
        Ansi::BrightRed => Color::LightRed,
        Ansi::BrightGreen => Color::LightGreen,
        Ansi::BrightYellow => Color::LightYellow,
        Ansi::BrightBlue => Color::LightBlue,
        Ansi::BrightMagenta => Color::LightMagenta,
        Ansi::BrightCyan => Color::LightCyan,
        Ansi::BrightWhite => Color::White,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The regression this guards: `Theme::default()` used to call `detect()`, so a
    /// palette built anywhere inherited the environment. Every test that renders got
    /// its colours from whatever terminal it ran under, which is how a preview test
    /// passed on a developer's machine and failed in CI, where there is no `TERM`.
    /// Every role has to survive its own absence, because colour carries hierarchy here
    /// and never meaning on its own. Under `NO_COLOR` a role either takes a modifier or
    /// deliberately takes nothing — what it must not do is set a colour anyway.
    #[test]
    fn no_role_sets_a_colour_when_there_is_none() {
        let theme = Theme::new(Depth::None);
        for style in [
            theme.accent(),
            theme.dim(),
            theme.warn(),
            theme.bad(),
            theme.good(),
            theme.cursor(),
        ] {
            assert!(style.fg.is_none(), "a role set a colour under NO_COLOR");
            assert!(style.bg.is_none(), "a role set a background under NO_COLOR");
        }
    }
}
