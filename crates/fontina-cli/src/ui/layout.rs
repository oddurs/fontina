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

//! How many panes the terminal can carry, and where they go.
//!
//! Kept apart from drawing so the decision can be tested at a width rather than at a
//! terminal. Every function here is a function of the width and the focus and nothing
//! else, which is also the reason the browser can be trusted to look the same on a
//! phone-sized `ssh` window as it does on a desktop.
//!
//! The breakpoints are not round numbers chosen for looking tidy. Each one is the
//! width at which the pane it protects stops being able to say what it knows.

use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Which pane the reader is in. The browser's `Focus` has a fourth state — the
/// controls — that lives *inside* the face pane; it maps to [`Pane::Detail`] here,
/// because a layout cares where something is drawn and not what its keys do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Facets,
    List,
    Detail,
}

/// How many panes fit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// Facets, families and the face, side by side. The browser as designed.
    Three,
    /// Families and the face. Facets are still a Tab away, drawn over the top.
    Two,
    /// One pane at a time, whichever has the focus.
    One,
}

/// Columns the face pane needs before it stops being able to say what it knows.
///
/// A `file` row is a ten-column label and then a path, and two of these columns are
/// the border, so this is a path of thirty-four characters on one line. It is not
/// generous — a deep path still wraps — but it is the width below which the wrapping
/// starts pushing the preview, which is the thing the browser exists to show, off the
/// bottom of the pane. Every breakpoint below is this number solved for the width.
const FACE: u16 = 46;

/// Columns the facet pane takes when it is beside the others. Wide enough for the
/// longest label the facets produce — `87.5% SemiCondensed` — a mark and a count.
const FACETS: u16 = 26;

/// Below this a third pane costs the face pane more than it is worth.
///
/// Three panes leave the face `WIDTH - FACETS - 36%`, so `FACE` columns needs
/// `0.64·WIDTH ≥ 72`. It is a wider breakpoint than a round number would have been,
/// and that is the finding rather than a compromise: three panes were never free, and
/// the old layout paid for the third one out of the pane a reader is actually looking
/// at. At eighty columns it left the face twenty-two.
pub const THREE: u16 = 112;

/// Below this two panes cannot both hold what they carry.
///
/// A family list wants its name, a count and three flag columns: about thirty columns
/// before names start losing their ends. `FACE` plus those thirty is this. Under it,
/// showing one pane properly beats truncating two.
pub const TWO: u16 = FACE + 30;

impl Shape {
    pub fn for_width(width: u16) -> Shape {
        if width >= THREE {
            Shape::Three
        } else if width >= TWO {
            Shape::Two
        } else {
            Shape::One
        }
    }

    /// Whether the face pane is a place the focus can rest.
    ///
    /// Beside the others it is a readout: Tab passing through it would stop on a pane
    /// where no key does anything, unless the face offers axes or features. Alone on
    /// the screen it is the only way to see the face at all, so it always takes focus
    /// — otherwise a 60-column terminal could browse a font library and never show
    /// anyone a font.
    pub fn detail_takes_focus(self, has_controls: bool) -> bool {
        self == Shape::One || has_controls
    }
}

/// Where each pane goes. `None` is a pane this width cannot carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Panes {
    pub facets: Option<Rect>,
    pub list: Option<Rect>,
    pub detail: Option<Rect>,
    /// True when the facet pane is drawn over the others rather than beside them, so
    /// the caller knows to clear beneath it and to say in the title that it is a
    /// visitor.
    pub overlay: bool,
}

/// Split the browser's body into panes.
pub fn split(area: Rect, focus: Pane) -> Panes {
    match Shape::for_width(area.width) {
        Shape::Three => {
            let c = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(FACETS),
                    Constraint::Percentage(36),
                    Constraint::Min(FACE),
                ])
                .split(area);
            Panes {
                facets: Some(c[0]),
                list: Some(c[1]),
                detail: Some(c[2]),
                overlay: false,
            }
        }
        Shape::Two => {
            let c = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(38), Constraint::Min(FACE)])
                .split(area);
            Panes {
                // Over the list exactly, and only when the reader asked: a narrower
                // overlay would leave a stripe of the list showing down its right
                // edge, which reads as a drawing bug rather than as one pane over
                // another. The face pane is never covered — it is what the facets are
                // being narrowed *for*.
                facets: (focus == Pane::Facets).then_some(c[0]),
                list: Some(c[0]),
                detail: Some(c[1]),
                overlay: true,
            }
        }
        Shape::One => Panes {
            facets: (focus == Pane::Facets).then_some(area),
            list: (focus == Pane::List).then_some(area),
            detail: (focus == Pane::Detail).then_some(area),
            overlay: false,
        },
    }
}

/// Every key the browser answers to, in the order a reader needs them.
///
/// Ordered by how often a session reaches for one, not by the alphabet and not by the
/// order they were written: search and movement before organising, organising before
/// the whole activation family, and `? help` last because it is the one that stands
/// for all the rest.
const KEYS: &[(&str, &str)] = &[
    ("/", "search"),
    ("⇥", "pane"),
    ("⏎", "open"),
    ("⌫", "back"),
    ("t", "tag"),
    ("c", "collection"),
    ("a/A", "activate"),
    ("d", "deactivate"),
    ("i", "install"),
    ("u", "uninstall"),
    ("e", "text"),
    ("+/-", "size"),
    ("P", "specimens"),
    ("s", "export"),
    ("R", "rescan"),
    ("q", "quit"),
];

/// The key hints, cut to a width that can hold them.
///
/// The old line was one 170-column string handed to a paragraph, which meant that
/// under 170 columns the terminal cut it wherever it ran out — mid-word, with no sign
/// that anything had been dropped, and taking `? help` with it. That is the one hint
/// whose absence is expensive, because it is the hint that would have told you about
/// the others. So it is reserved first and the rest fill what is left, and a reader
/// who cannot see a key can always see how to find it.
pub fn keys(width: u16) -> String {
    let help = "? help";
    let width = width as usize;
    if width <= help.len() + 1 {
        return String::new();
    }
    let mut line = String::from(" ");
    for (key, label) in KEYS {
        let hint = format!("{key} {label}  ");
        // Two columns of gap before `? help` so it reads as its own thing.
        if line.chars().count() + hint.chars().count() + help.len() > width {
            break;
        }
        line.push_str(&hint);
    }
    line.push_str(help);
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(width: u16) -> Rect {
        Rect::new(0, 0, width, 40)
    }

    #[test]
    fn a_width_picks_a_shape_and_the_boundaries_belong_to_the_wider_one() {
        assert_eq!(Shape::for_width(200), Shape::Three);
        const { assert!(TWO < THREE, "the breakpoints are in order") };
        assert_eq!(Shape::for_width(THREE), Shape::Three);
        assert_eq!(Shape::for_width(THREE - 1), Shape::Two);
        assert_eq!(Shape::for_width(TWO), Shape::Two);
        assert_eq!(Shape::for_width(TWO - 1), Shape::One);
        assert_eq!(Shape::for_width(0), Shape::One);
    }

    /// The reason the breakpoints are where they are: at every width the face pane is
    /// drawn at, it has room for a ten-column label and a path of ordinary length.
    /// This is the assertion the item was filed over, so it is the one to keep.
    #[test]
    fn the_face_pane_can_always_hold_a_path() {
        for width in TWO - 16..=400u16 {
            let panes = split(area(width), Pane::Detail);
            let Some(detail) = panes.detail else {
                panic!("{width}: the face pane has to be reachable at every width")
            };
            assert!(
                detail.width >= FACE,
                "{width} columns ({:?}): the face pane got {} columns, which cannot \
                 hold a path",
                Shape::for_width(width),
                detail.width
            );
        }
    }

    #[test]
    fn three_panes_are_side_by_side_and_cover_the_width() {
        let panes = split(area(120), Pane::List);
        assert!(!panes.overlay);
        let (f, l, d) = (
            panes.facets.unwrap(),
            panes.list.unwrap(),
            panes.detail.unwrap(),
        );
        assert_eq!(f.x, 0);
        assert_eq!(f.x + f.width, l.x);
        assert_eq!(l.x + l.width, d.x);
        assert_eq!(d.x + d.width, 120);
    }

    #[test]
    fn two_panes_keep_the_facets_a_keystroke_away() {
        let resting = split(area(80), Pane::List);
        assert!(
            resting.facets.is_none(),
            "facets are a pane you open, not one that opens itself"
        );
        assert!(resting.list.is_some() && resting.detail.is_some());

        let asked = split(area(80), Pane::Facets);
        let over = asked.facets.expect("Tab has to be able to reach them");
        assert!(asked.overlay, "and they are drawn over, not beside");
        assert_eq!(
            Some(over),
            asked.list,
            "over the list exactly, so no stripe of it shows down the edge"
        );
        assert!(
            asked.detail.is_some(),
            "and never over the face, which is what the narrowing was for"
        );
    }

    #[test]
    fn one_pane_shows_exactly_the_one_with_the_focus() {
        for (focus, name) in [
            (Pane::Facets, "facets"),
            (Pane::List, "list"),
            (Pane::Detail, "detail"),
        ] {
            let p = split(area(60), focus);
            let shown: Vec<_> = [("facets", p.facets), ("list", p.list), ("detail", p.detail)]
                .into_iter()
                .filter_map(|(n, r)| r.map(|r| (n, r)))
                .collect();
            assert_eq!(shown.len(), 1, "{name}: one pane means one pane");
            assert_eq!(shown[0].0, name);
            assert_eq!(shown[0].1, area(60), "and it takes the whole screen");
        }
    }

    /// The face pane is a readout beside the others and the only view of a face when
    /// it is alone, so whether Tab stops on it depends on the shape as well as on
    /// whether the face has anything to adjust.
    #[test]
    fn the_face_pane_takes_focus_when_it_is_the_only_way_to_see_a_face() {
        assert!(!Shape::Three.detail_takes_focus(false));
        assert!(Shape::Three.detail_takes_focus(true));
        assert!(!Shape::Two.detail_takes_focus(false));
        assert!(Shape::One.detail_takes_focus(false));
        assert!(Shape::One.detail_takes_focus(true));
    }

    #[test]
    fn the_key_hints_fit_the_width_they_are_given() {
        for width in 0..=200u16 {
            let line = keys(width);
            assert!(
                line.chars().count() <= width as usize,
                "{width}: {line:?} is {} columns",
                line.chars().count()
            );
        }
    }

    /// The hint that stands for all the others is the one that survives.
    #[test]
    fn help_is_the_last_hint_to_go() {
        assert!(keys(200).ends_with("? help"));
        assert!(keys(60).ends_with("? help"));
        assert!(keys(20).ends_with("? help"));
        assert_eq!(
            keys(6),
            "",
            "and below its own width there is nothing to say"
        );
    }

    /// Wider is never worse: an extra column can add a hint but must never take one
    /// away, or the line would flicker as a window is dragged.
    #[test]
    fn the_hints_only_ever_grow_with_the_width() {
        let mut last = 0;
        for width in 0..=200u16 {
            let n = keys(width).chars().count();
            assert!(n >= last, "{width}: the line got shorter, {last} then {n}");
            last = n;
        }
    }
}
