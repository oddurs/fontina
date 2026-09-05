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

/// What the terminal can show, in the order the checks run.
///
/// The default is the top of the range, not the bottom: a palette assumes the best
/// until something asks the terminal, and [`Depth::detect`] is that asking. Keeping
/// the asking out of `Default` is what makes a `Theme` a value rather than a reading
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

    /// Ink for a rasterised preview, for coverage `alpha`.
    ///
    /// `None` means this terminal has no colour to draw type with, and the caller must
    /// say it in characters instead. That is not a degraded mode so much as a different
    /// medium: see [`density`].
    pub fn ink(&self, alpha: u8) -> Option<Color> {
        // A neutral light grey blended over black, which reads on a dark theme and a
        // light one alike once the block glyph carries both halves.
        let v = 30 + (u16::from(alpha) * 200 / 255) as u8;
        Some(match self.depth {
            Depth::True => Color::Rgb(v, v, v),
            // 232..=255 is the greyscale ramp, twenty-four steps from near-black to
            // near-white — enough for type at this size, and the reason to reach for
            // the ramp rather than the six-level cube.
            Depth::Ansi256 => Color::Indexed(232 + (u16::from(v) * 23 / 255) as u8),
            Depth::Ansi16 => match v {
                0..=70 => Color::Black,
                71..=130 => Color::DarkGray,
                131..=200 => Color::Gray,
                _ => Color::White,
            },
            Depth::None => return None,
        })
    }
}

/// The block character for a pair of vertically stacked pixels, when colour is not
/// available to carry them.
///
/// A half-block preview normally puts two pixels in one cell by colouring the glyph's
/// halves separately. With no colour that cannot work — but the block glyphs already
/// encode which half is filled, so the same two pixels come through as shape instead.
/// The preview survives `NO_COLOR` rather than disappearing under it.
pub fn density(top: u8, bottom: u8) -> char {
    const ON: u8 = 96;
    match (top >= ON, bottom >= ON) {
        (true, true) => '█',
        (true, false) => '▀',
        (false, true) => '▄',
        (false, false) => ' ',
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The regression this guards: `Theme::default()` used to call `detect()`, so a
    /// palette built anywhere inherited the environment. Every test that renders got
    /// its colours from whatever terminal it ran under, which is how a preview test
    /// passed on a developer's machine and failed in CI, where there is no `TERM`.
    #[test]
    fn a_palette_is_a_value_and_never_a_reading_of_the_environment() {
        assert_eq!(Theme::default(), Theme::new(Depth::True));
        assert_eq!(Depth::default(), Depth::True);
    }

    #[test]
    fn no_color_beats_a_terminal_that_promises_everything() {
        assert_eq!(
            Depth::from_env(true, Some("truecolor"), Some("xterm-256color")),
            Depth::None,
            "a person saying no outranks a terminal saying yes"
        );
    }

    #[test]
    fn the_depth_is_read_from_the_environment_in_order() {
        assert_eq!(Depth::from_env(false, Some("truecolor"), None), Depth::True);
        assert_eq!(Depth::from_env(false, Some("24bit"), None), Depth::True);
        assert_eq!(
            Depth::from_env(false, None, Some("xterm-256color")),
            Depth::Ansi256
        );
        assert_eq!(Depth::from_env(false, None, Some("xterm")), Depth::Ansi16);
        // A terminal that says it cannot is the same instruction as a person saying not
        // to, arriving from the other side.
        assert_eq!(Depth::from_env(false, None, Some("dumb")), Depth::None);
        assert_eq!(Depth::from_env(false, None, None), Depth::None);
    }

    /// Ink has to stay ink at every depth: monotonic, so more coverage is never lighter,
    /// and never the same colour at both ends, or type becomes a solid block.
    #[test]
    fn ink_darkens_and_lightens_the_same_way_at_every_depth() {
        for depth in [Depth::True, Depth::Ansi256, Depth::Ansi16] {
            let theme = Theme::new(depth);
            let low = theme.ink(0).expect("a colour depth has ink");
            let high = theme.ink(255).expect("a colour depth has ink");
            assert_ne!(low, high, "{depth:?} collapses ink to one colour");
        }
    }

    #[test]
    fn no_colour_has_no_ink_and_says_so() {
        assert_eq!(Theme::new(Depth::None).ink(255), None);
    }

    /// The characters are the fallback's whole vocabulary, so each pair of pixels has to
    /// reach a different one.
    #[test]
    fn density_encodes_both_halves_of_a_cell() {
        assert_eq!(density(0, 0), ' ');
        assert_eq!(density(255, 0), '▀');
        assert_eq!(density(0, 255), '▄');
        assert_eq!(density(255, 255), '█');
    }

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
        ] {
            assert!(style.fg.is_none(), "a role set a colour under NO_COLOR");
            assert!(style.bg.is_none(), "a role set a background under NO_COLOR");
        }
    }
}
