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
use std::ops::Range;

/// Which pane the reader is in. The browser's `Focus` has states — the controls, and
/// the Narrow-by panel — that live *inside* or *over* a pane rather than beside it;
/// they map to the pane they are drawn on, because a layout cares where something is
/// drawn and not what its keys do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    List,
    Detail,
}

/// How many panes fit.
///
/// Two of them, or one. There used to be a third — the facets, beside the list, at
/// 112 columns and up — and it was the pane nobody could name. It is a panel now,
/// opened on `f` and drawn over the others, which is what a control surface used in
/// bursts should cost: the width while you are using it, and nothing the rest of the
/// time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// Families and the face, side by side.
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

/// Columns the Narrow-by panel takes. Wide enough for the longest label the facets
/// produce — `87.5% SemiCondensed` — a mark, a family count and a face count in
/// brackets.
pub const PANEL: u16 = 38;

/// Below this two panes cannot both hold what they carry.
///
/// A family list wants its name, a count and three flag columns: about thirty columns
/// before names start losing their ends. `FACE` plus those thirty is this. Under it,
/// showing one pane properly beats truncating two.
pub const TWO: u16 = FACE + 30;

impl Shape {
    pub fn for_width(width: u16) -> Shape {
        if width >= TWO { Shape::Two } else { Shape::One }
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
    pub list: Option<Rect>,
    pub detail: Option<Rect>,
}

/// Split the browser's body into panes.
pub fn split(area: Rect, focus: Pane) -> Panes {
    match Shape::for_width(area.width) {
        Shape::Two => {
            let c = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(38), Constraint::Min(FACE)])
                .split(area);
            Panes {
                list: Some(c[0]),
                detail: Some(c[1]),
            }
        }
        Shape::One => Panes {
            list: (focus == Pane::List).then_some(area),
            detail: (focus == Pane::Detail).then_some(area),
        },
    }
}

/// Where the Narrow-by panel goes: a drawer down the left of the body.
///
/// Over the list rather than beside it, and never over the face, which is what the
/// narrowing is *for*. Down the left because that is where the facets lived for a
/// year and where a reader's eye already goes, and full height because a section with
/// its values open is the tallest thing the browser draws.
pub fn panel(area: Rect) -> Rect {
    match Shape::for_width(area.width) {
        // Nothing is beside it to protect, and a drawer with a stripe of the pane
        // underneath showing down its edge reads as a drawing bug rather than as one
        // thing over another.
        Shape::One => area,
        Shape::Two => {
            let list = split(area, Pane::List).list.unwrap_or(area);
            Rect::new(list.x, list.y, PANEL.min(list.width), list.height)
        }
    }
}

/// Rows of scrolloff: the cursor is kept this far from the edge of a pane while there
/// is list left to show, so a reader can see where they are going as well as where
/// they are.
const MARGIN: usize = 2;

/// The slice of a long list that a pane `height` rows tall should draw.
///
/// ratatui scrolls a `List` on its own, but only after being handed a widget for every
/// item in it. At the fixture scale that is six of them and free; on a real library it
/// is ten thousand `format!` calls per keystroke to fill thirty rows, and it is paid
/// again on the next keystroke because the frame is built from scratch. So the
/// scrolling moves here, and the pane is handed the rows it draws and nothing else.
///
/// `offset` is where the pane was looking on the last frame. It is an input rather than
/// something derived, because a list scrolled halfway down and then filtered should
/// stay where the reader left it rather than jumping to the cursor.
pub fn window(len: usize, selected: usize, height: usize, offset: usize) -> Range<usize> {
    if len == 0 || height == 0 {
        return 0..0;
    }
    // Never past the end: a pane showing the last screenful shows a full screenful.
    let last = len.saturating_sub(height);
    let mut start = offset.min(last);
    // Scrolloff, but never so much that it pushes the cursor off the other edge, which
    // is what happens on a pane shorter than twice the margin.
    let margin = MARGIN.min(height.saturating_sub(1) / 2);
    let selected = selected.min(len - 1);
    if selected < start + margin {
        start = selected.saturating_sub(margin);
    } else if selected + margin >= start + height {
        start = (selected + margin + 1).saturating_sub(height);
    }
    let start = start.min(last);
    start..(start + height).min(len)
}

/// Every key the browser answers to, in the order a reader needs them.
///
/// Ordered by how often a session reaches for one, not by the alphabet and not by the
/// order they were written: search and movement before organising, organising before
/// the whole activation family, and `? help` last because it is the one that stands
/// for all the rest.
const KEYS: &[(&str, &str)] = &[
    ("/", "search"),
    ("f", "narrow"),
    ("F", "flags"),
    ("x", "clear"),
    ("⇥", "pane"),
    ("⏎", "open"),
    ("⌫", "back"),
    ("␣", "select"),
    ("t", "tag"),
    ("c", "collection"),
    ("a/A", "activate"),
    ("d", "deactivate"),
    ("i", "install"),
    ("u", "uninstall"),
    ("r", "alike"),
    ("e", "who sets"),
    ("P", "pairing"),
    ("m", "glyphs"),
    ("s", "specimen"),
    ("U", "undo"),
    (":", "commands"),
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
    let mut cut = false;
    for (key, label) in KEYS {
        let hint = format!("{key} {label}  ");
        // Two columns of gap before `? help` so it reads as its own thing, and two more
        // for the `… ` that says the line was cut. Reserved whether or not it is
        // needed, so that adding the mark can never be what pushes `? help` off.
        if line.chars().count() + hint.chars().count() + help.len() + 2 > width {
            cut = true;
            break;
        }
        line.push_str(&hint);
    }
    // A line that stops has to say it stopped. It used to end wherever it ran out, and
    // a reader with a sixty-column window had no way to know there were nine more keys
    // — or that `?` would list them, which is the one hint that stands for the rest.
    if cut && line.chars().count() + 2 + help.len() <= width {
        line.push_str("… ");
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
    fn a_width_picks_a_shape_and_the_boundary_belongs_to_the_wider_one() {
        assert_eq!(Shape::for_width(200), Shape::Two);
        assert_eq!(Shape::for_width(TWO), Shape::Two);
        assert_eq!(Shape::for_width(TWO - 1), Shape::One);
        assert_eq!(Shape::for_width(0), Shape::One);
    }

    /// The width an ordinary terminal opens at. Both panes, side by side, and neither
    /// of them a stub: this is the assertion the redesign is answerable to.
    #[test]
    fn eighty_columns_still_shows_a_list_and_a_face() {
        let panes = split(area(80), Pane::List);
        let (list, detail) = (
            panes.list.expect("a list at 80 columns"),
            panes.detail.expect("and a face beside it"),
        );
        assert!(list.width >= 24, "the list got {} columns", list.width);
        assert!(
            detail.width >= FACE,
            "the face got {} columns",
            detail.width
        );
        assert_eq!(list.x + list.width, detail.x);
        assert_eq!(detail.x + detail.width, 80);
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
    fn two_panes_are_side_by_side_and_cover_the_width() {
        let panes = split(area(120), Pane::List);
        let (l, d) = (panes.list.unwrap(), panes.detail.unwrap());
        assert_eq!(l.x, 0);
        assert_eq!(l.x + l.width, d.x);
        assert_eq!(d.x + d.width, 120);
    }

    #[test]
    fn one_pane_shows_exactly_the_one_with_the_focus() {
        for (focus, name) in [(Pane::List, "list"), (Pane::Detail, "detail")] {
            let p = split(area(60), focus);
            let shown: Vec<_> = [("list", p.list), ("detail", p.detail)]
                .into_iter()
                .filter_map(|(n, r)| r.map(|r| (n, r)))
                .collect();
            assert_eq!(shown.len(), 1, "{name}: one pane means one pane");
            assert_eq!(shown[0].0, name);
            assert_eq!(shown[0].1, area(60), "and it takes the whole screen");
        }
    }

    /// The panel is a drawer over the list, never over the face — and on a terminal
    /// too narrow to hold both it takes what there is rather than drawing outside it.
    #[test]
    fn the_panel_is_a_drawer_that_never_covers_the_face() {
        for width in 20..=200u16 {
            let body = area(width);
            let p = panel(body);
            assert_eq!((p.x, p.y, p.height), (body.x, body.y, body.height));
            assert!(p.width <= width, "{width}: the panel drew outside the body");
            assert!(p.width > 0, "{width}: the panel has to be somewhere");
            let panes = split(body, Pane::List);
            if let Some(detail) = panes.detail {
                assert!(
                    p.x + p.width <= detail.x,
                    "{width}: the panel reached into the face pane"
                );
                assert_eq!(
                    p.width,
                    PANEL.min(panes.list.unwrap().width),
                    "{width}: it takes its width or the list's, whichever is less"
                );
            } else {
                assert_eq!(p, body, "one pane: the drawer is the screen");
            }
        }
        assert!(
            panel(area(200)).width == PANEL,
            "and never grows past its own width"
        );
    }

    /// The face pane is a readout beside the others and the only view of a face when
    /// it is alone, so whether Tab stops on it depends on the shape as well as on
    /// whether the face has anything to adjust.
    #[test]
    fn the_face_pane_takes_focus_when_it_is_the_only_way_to_see_a_face() {
        assert!(!Shape::Two.detail_takes_focus(false));
        assert!(Shape::Two.detail_takes_focus(true));
        assert!(Shape::One.detail_takes_focus(false));
        assert!(Shape::One.detail_takes_focus(true));
    }

    /// The property the whole change rests on: what a pane builds is bounded by what
    /// it can show, never by what it holds.
    #[test]
    fn a_window_is_the_size_of_the_pane_and_not_of_the_list() {
        for len in [0usize, 1, 5, 30, 1_000, 10_000] {
            for height in [0usize, 1, 3, 30] {
                for selected in [0, len / 2, len.saturating_sub(1)] {
                    let w = window(len, selected, height, 0);
                    assert!(
                        w.len() <= height,
                        "{len}/{height}/{selected}: built {} rows for {height}",
                        w.len()
                    );
                    assert!(w.end <= len, "{len}/{height}/{selected}: past the end");
                    if len >= height {
                        assert_eq!(w.len(), height, "and a full pane is filled");
                    }
                }
            }
        }
    }

    /// A cursor you cannot see is a cursor you cannot follow.
    #[test]
    fn the_selection_is_always_inside_the_window() {
        for selected in 0..200usize {
            // Walking down, carrying the offset the way a frame does.
            let mut offset = 0;
            for i in 0..=selected {
                let w = window(200, i, 20, offset);
                offset = w.start;
                assert!(
                    w.contains(&i),
                    "{i}: the cursor scrolled out of its own pane"
                );
            }
        }
    }

    /// Scrolloff, and the two ends where it has to give way: at the top and bottom of
    /// a list there is nothing to keep the cursor away from.
    #[test]
    fn the_cursor_keeps_its_distance_from_the_edge_except_at_the_ends() {
        let w = window(200, 100, 20, 0);
        assert!(w.contains(&100));
        assert!(
            100 - w.start >= MARGIN && w.end - 100 > MARGIN,
            "{w:?} put the cursor on the edge with list left on both sides"
        );

        assert_eq!(window(200, 0, 20, 0), 0..20, "the top is the top");
        assert_eq!(
            window(200, 199, 20, 180),
            180..200,
            "and the bottom the bottom"
        );

        // A pane too short for the margin still shows the cursor.
        for height in [1usize, 2, 3, 4] {
            let w = window(200, 100, height, 0);
            assert!(w.contains(&100), "{height} rows: {w:?} lost the cursor");
        }
    }

    /// A filter that shortens the list must not leave the pane looking past the end of
    /// it, and a list the reader scrolled must not jump back to the cursor.
    #[test]
    fn the_window_survives_the_list_changing_under_it() {
        assert_eq!(
            window(10, 0, 20, 500),
            0..10,
            "an offset past a shortened list comes back to the top"
        );
        let scrolled = window(1_000, 300, 20, 290);
        assert_eq!(scrolled.start, 290, "a pane the reader scrolled stays put");
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
