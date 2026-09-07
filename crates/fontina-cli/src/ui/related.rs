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

//! What else in the library covers nearly the same characters.
//!
//! A real library holds families that are nearly the same font: a patched build, a
//! re-encoding, a subset, an interpolation of one design. Nothing surfaced that, so a
//! person scrolled past six rows that were one typeface. One library of 149 faces
//! reports twenty families and holds about eight typefaces, because a patch spaced
//! three ways names itself three times.
//!
//! `Index::related` already answers it. What this adds is a view, and one rule about
//! how to show the answer: **the number is shown, never thresholded away.** A list that
//! silently drops everything under some cutoff is a list that has made the judgement
//! for the reader, and the judgement is the whole question — 0.62 between two faces
//! with different units per em is a coincidence, and 0.98 with every metric agreeing is
//! the same design twice. So the score is a column, the four metrics that tell those
//! two cases apart are beside it, and the floor the query used is in the title where it
//! can be argued with.

use fontina_core::{FaceMetadata, Related as Row};

/// The four numbers that decide whether identical coverage means identical design.
///
/// Held per face rather than as the one `metrics_agree` bool the query returns, because
/// "they agree" is a conclusion and the reader is entitled to the evidence. Two faces
/// at 1000 and 2048 units per em are not the same font however much they overlap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Metrics {
    pub upem: u16,
    pub ascender: i16,
    pub descender: i16,
    pub fixed_pitch: bool,
}

impl Metrics {
    pub fn of(face: &FaceMetadata) -> Metrics {
        Metrics {
            upem: face.metrics.units_per_em,
            ascender: face.metrics.ascender,
            descender: face.metrics.descender,
            fixed_pitch: face.metrics.is_fixed_pitch,
        }
    }

    /// The four numbers, in the space there is for them.
    pub fn summary(&self) -> String {
        format!(
            "upem {} asc {} desc {} {}",
            self.upem,
            self.ascender,
            self.descender,
            if self.fixed_pitch { "fixed" } else { "prop" }
        )
    }
}

/// One candidate, with everything it was ranked on.
pub struct Candidate {
    pub row: Row,
    pub metrics: Metrics,
}

/// The overlap below which a face is not worth listing.
///
/// Low on purpose. It is here so that asking about a Latin face does not return every
/// Latin face in the library, not to decide which answers count — a tenth of the union
/// in common is already more than two unrelated designs usually share, and everything
/// above it is shown with its number. The title says what the floor was, because a
/// reader who sees six results should be able to ask what the seventh was.
pub const FLOOR: f64 = 0.10;

/// The list, while it is open.
pub struct View {
    target: String,
    target_metrics: Metrics,
    candidates: Vec<Candidate>,
    cursor: usize,
}

impl View {
    pub fn new(target: &FaceMetadata, candidates: Vec<Candidate>) -> View {
        View {
            target: format!("{} {}", target.names.family, target.names.subfamily),
            target_metrics: Metrics::of(target),
            candidates,
            cursor: 0,
        }
    }

    pub fn candidates(&self) -> &[Candidate] {
        &self.candidates
    }

    pub fn is_empty(&self) -> bool {
        self.candidates.is_empty()
    }

    pub fn cursor(&self) -> usize {
        self.cursor.min(self.candidates.len().saturating_sub(1))
    }

    pub fn selected(&self) -> Option<&Candidate> {
        self.candidates.get(self.cursor())
    }

    pub fn move_cursor(&mut self, delta: i32) {
        if self.candidates.is_empty() {
            self.cursor = 0;
            return;
        }
        let last = self.candidates.len() as i32 - 1;
        self.cursor = (self.cursor.min(last as usize) as i32 + delta).clamp(0, last) as usize;
    }

    pub fn jump(&mut self, to: usize) {
        self.cursor = to.min(self.candidates.len().saturating_sub(1));
    }

    /// The face being asked about, and what the answer was measured against.
    pub fn title(&self) -> String {
        format!(
            "like {} — {} face(s) over {FLOOR:.2} overlap, {}",
            self.target,
            self.candidates.len(),
            self.target_metrics.summary()
        )
    }

    /// One row, as wide as the pane it is going into.
    ///
    /// The score first, because it is the column a reader scans; then the name, which
    /// is what they are looking for; then the evidence. The name is the part that gives
    /// way when the pane is narrow, because a truncated name is still recognisable and
    /// a truncated number is a different number.
    pub fn row_text(&self, c: &Candidate, width: usize) -> String {
        let head = format!("{:.2}  ", c.row.overlap);
        let tail = format!(
            "  {}/{} shared  {}  {}",
            c.row.shared,
            c.row.union,
            c.metrics.summary(),
            if c.row.metrics_agree {
                "same metrics"
            } else {
                "different metrics"
            }
        );
        let name = format!("{} {}", c.row.face.family, c.row.face.subfamily);
        let room = width
            .saturating_sub(head.chars().count() + tail.chars().count())
            .max(8);
        format!("{head}{:<room$}{tail}", super::truncate(&name, room))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fontina_core::{FaceSummary, Related as Row};

    fn metrics(upem: u16) -> Metrics {
        Metrics {
            upem,
            ascender: 900,
            descender: -200,
            fixed_pitch: false,
        }
    }

    fn candidate(family: &str, overlap: f64, agree: bool, upem: u16) -> Candidate {
        Candidate {
            row: Row {
                face: FaceSummary {
                    id: 1,
                    path: "/x.ttf".into(),
                    index: 0,
                    family: family.into(),
                    subfamily: "Regular".into(),
                    ..sample_summary()
                },
                overlap,
                shared: 1699,
                union: 1730,
                metrics_agree: agree,
            },
            metrics: metrics(upem),
        }
    }

    /// `FaceSummary` has more fields than this test cares about and no `Default`, so
    /// one is built once, from the JSON the index itself serialises.
    fn sample_summary() -> FaceSummary {
        serde_json::from_str(
            r#"{"id":1,"path":"/x.ttf","index":0,"family":"F","subfamily":"Regular",
                "weight":400.0,"width":100.0,"italic":false,"variable":false,
                "color":false,"monospace":false,"glyph_count":100,"scripts":[],
                "container":"ttf","tags":[]}"#,
        )
        .expect("a summary the index would produce")
    }

    fn view(candidates: Vec<Candidate>) -> View {
        View {
            target: "Amiri Regular".into(),
            target_metrics: metrics(1000),
            candidates,
            cursor: 0,
        }
    }

    /// The rule the whole view exists to keep: the number is shown, and so is the
    /// evidence that says what it means.
    #[test]
    fn a_row_carries_the_score_and_the_four_metrics_behind_it() {
        let v = view(vec![candidate("Inter", 0.98, true, 1000)]);
        let text = v.row_text(&v.candidates[0], 120);
        assert!(text.starts_with("0.98  "), "{text}");
        assert!(text.contains("Inter Regular"), "{text}");
        assert!(text.contains("1699/1730 shared"), "{text}");
        assert!(text.contains("upem 1000 asc 900 desc -200 prop"), "{text}");
        assert!(text.contains("same metrics"), "{text}");
    }

    /// The case the metrics are there to catch: the same coverage at a different
    /// units-per-em is not the same design, and the row has to say so.
    #[test]
    fn coverage_without_the_metrics_is_reported_as_different() {
        let v = view(vec![candidate("Coincidence", 0.98, false, 2048)]);
        let text = v.row_text(&v.candidates[0], 120);
        assert!(text.contains("upem 2048"), "{text}");
        assert!(text.contains("different metrics"), "{text}");
    }

    /// A narrow pane gives way on the name, never on the numbers: a truncated name is
    /// still recognisable and a truncated number is a different number.
    #[test]
    fn the_name_is_what_gives_way_when_the_pane_is_narrow() {
        let v = view(vec![candidate(
            "A Family With A Very Long Name Indeed",
            0.5,
            true,
            1000,
        )]);
        for width in [40usize, 60, 80, 120] {
            let text = v.row_text(&v.candidates[0], width);
            assert!(text.starts_with("0.50"), "{width}: {text}");
            assert!(text.contains("1699/1730 shared"), "{width}: {text}");
            assert!(text.contains("same metrics"), "{width}: {text}");
        }
    }

    /// The title says what the floor was, so a reader who sees six answers can ask
    /// what the seventh was.
    #[test]
    fn the_title_names_the_face_and_the_floor_it_was_measured_against() {
        let v = view(vec![candidate("Inter", 0.9, true, 1000)]);
        let t = v.title();
        assert!(t.contains("like Amiri Regular"), "{t}");
        assert!(t.contains("1 face(s) over 0.10 overlap"), "{t}");
        assert!(t.contains("upem 1000"), "{t}");
    }

    #[test]
    fn the_cursor_stays_inside_the_list_and_an_empty_one_has_no_selection() {
        let mut v = view(vec![
            candidate("A", 0.9, true, 1000),
            candidate("B", 0.5, true, 1000),
        ]);
        v.move_cursor(5);
        assert_eq!(v.cursor(), 1);
        v.move_cursor(-5);
        assert_eq!(v.cursor(), 0);
        v.jump(99);
        assert_eq!(v.cursor(), 1);

        let mut empty = view(Vec::new());
        assert!(empty.is_empty());
        assert!(empty.selected().is_none());
        empty.move_cursor(1);
        assert_eq!(empty.cursor(), 0, "moving in an empty list stays put");
    }
}
