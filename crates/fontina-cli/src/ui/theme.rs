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

use ratatui::style::{Color, Modifier, Style};

/// What the terminal can show. Resolved for the whole process in [`crate::term`]; the
/// browser takes the same answer and turns it into ratatui styles.
pub use crate::term::Depth;

/// The palette, resolved once.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Theme {
    depth: Depth,
}

impl Theme {
    pub fn new(depth: Depth) -> Self {
        Theme { depth }
    }

    /// The pane you are in, the sheet that is open, the thing being pointed at.
    pub fn accent(&self) -> Style {
        match self.depth {
            Depth::None => Style::default().add_modifier(Modifier::BOLD),
            _ => Style::default().fg(Color::Cyan),
        }
    }

    /// The one thing on the screen the reader is being pointed *at*: a codepoint a
    /// search just found, sitting in a grid of several hundred others.
    ///
    /// Reversed rather than coloured when there is no colour, because the whole job of
    /// this style is to be the one cell an eye lands on, and a cell that merely goes
    /// bold in a grid of glyphs is not that.
    pub fn cursor(&self) -> Style {
        match self.depth {
            Depth::None => Style::default().add_modifier(Modifier::REVERSED),
            _ => Style::default().fg(Color::Black).bg(Color::Cyan),
        }
    }

    /// Present, and not what you are reading: labels, keys, the row of hints.
    ///
    /// Under `NO_COLOR` this is deliberately plain rather than dim. There is no
    /// modifier that means "quieter" the way a grey does — `DIM` is unsupported often
    /// enough to be a coin toss — and a label in italics is louder than one in nothing.
    pub fn dim(&self) -> Style {
        match self.depth {
            Depth::None => Style::default(),
            _ => Style::default().fg(Color::DarkGray),
        }
    }

    /// Something the reader should notice but need not act on.
    pub fn warn(&self) -> Style {
        match self.depth {
            Depth::None => Style::default().add_modifier(Modifier::BOLD),
            _ => Style::default().fg(Color::Yellow),
        }
    }

    /// Something that failed.
    pub fn bad(&self) -> Style {
        match self.depth {
            Depth::None => Style::default().add_modifier(Modifier::BOLD),
            _ => Style::default().fg(Color::Red),
        }
    }

    /// Something that worked: an activation, a free licence.
    pub fn good(&self) -> Style {
        match self.depth {
            Depth::None => Style::default(),
            _ => Style::default().fg(Color::Green),
        }
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
