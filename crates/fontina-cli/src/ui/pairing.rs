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

//! Faces that might go with this one.
//!
//! Pairing is the question after choosing, and it is the one thing here that no field
//! in the index answers directly. So be clear about what this is: **it is not taste and
//! it does not pretend to be.** It is a ranking, shown with the numbers behind it, that
//! puts twenty plausible candidates in front of someone instead of four hundred faces.
//!
//! Nothing here calls a pairing good. Every row says what it measured and what it
//! found, in the words of the measurement — "weight +400", "x/em 0.55 vs 0.52", "3
//! scripts shared" — and the reader decides. A program that ranked type by taste and
//! then said so would be lying twice.
//!
//! ## What is measured, and what is not
//!
//! Four things, all of them already in the index: contrast in weight, contrast in
//! width, whether the spacing class differs, and how close the x-heights are at a
//! common size. Plus one gate: scripts in common, because two faces that cannot set the
//! same text are not a pair whatever else they are.
//!
//! The item that asked for this also asked for "a different outline class", meaning
//! serif against sans. **The index does not store that.** There is no PANOSE and no
//! `OS/2.sFamilyClass` in the model, and the acceptance criteria say nothing new is to
//! be parsed — so the honest thing is to leave it out and say so, rather than dress a
//! `glyf`-versus-`CFF` difference up as a typographic one. The nearest thing the index
//! does hold is the spacing class, which is a real pairing signal on its own: a
//! monospace against a proportional is a contrast anybody would recognise.

use fontina_core::model::FaceMetadata;

/// One candidate, measured against the target.
#[derive(Debug, Clone, PartialEq)]
pub struct Measured {
    pub id: i64,
    pub label: String,
    /// Difference in CSS weight, signed: positive means heavier than the target.
    pub weight_delta: f32,
    /// Difference in width as a percentage, signed.
    pub width_delta: f32,
    /// x-height as a fraction of the em, for the target and for this face. `None` where
    /// a face does not report one, which is a fact about the font rather than a zero.
    pub x_em: Option<f32>,
    pub target_x_em: Option<f32>,
    /// Whether one is monospaced and the other is not.
    pub spacing_differs: bool,
    /// Scripts both cover.
    pub shared_scripts: Vec<String>,
}

impl Measured {
    /// The measurements, in the words of the measurements.
    ///
    /// No adjective anywhere. "Weight +400" is a fact; "a strong contrast" is an
    /// opinion, and one the reader is better placed to have.
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();
        parts.push(if self.weight_delta == 0.0 {
            "weight same".to_string()
        } else {
            format!("weight {:+.0}", self.weight_delta)
        });
        if self.width_delta != 0.0 {
            parts.push(format!("width {:+.0}%", self.width_delta));
        }
        if self.spacing_differs {
            parts.push("spacing differs".into());
        }
        parts.push(match (self.target_x_em, self.x_em) {
            (Some(t), Some(c)) => format!("x/em {t:.2} vs {c:.2}"),
            _ => "x/em not reported".into(),
        });
        parts.push(match self.shared_scripts.len() {
            1 => format!("1 script shared ({})", self.shared_scripts[0]),
            n => format!("{n} scripts shared"),
        });
        parts.join(" · ")
    }

    /// How far apart the two x-heights are, as a fraction of the em. `None` when either
    /// face does not report one — an unknown is not a zero.
    pub fn x_height_gap(&self) -> Option<f32> {
        Some((self.target_x_em? - self.x_em?).abs())
    }

    /// The order, lower first.
    ///
    /// Not a quality. It is a sort key that puts contrast in weight or width, a
    /// difference in spacing, and a comparable x-height near the top, because those are
    /// the things pairing turns on — and it is only ever used to decide which twenty of
    /// four hundred faces to show first.
    fn rank(&self) -> f32 {
        // Contrast wanted, so distance from zero *helps*: a face 400 weights away is
        // ranked above one 20 away. Capped, because past a point more is not more.
        let weight = 1.0 - (self.weight_delta.abs() / 400.0).min(1.0);
        let width = 1.0 - (self.width_delta.abs() / 40.0).min(1.0);
        let spacing = if self.spacing_differs { 0.0 } else { 0.5 };
        // Closeness wanted here: two faces set together want to look the same size,
        // and x-height at a common size is most of what "the same size" means.
        let x = match self.x_height_gap() {
            Some(gap) => (gap / 0.1).min(1.0),
            // Unknown sorts in the middle rather than at either end: a face that does
            // not report an x-height is neither a good nor a bad match on it.
            None => 0.5,
        };
        weight * 2.0 + width + spacing + x * 2.0
    }
}

/// Measure one candidate against the target.
pub fn measure(target: &FaceMetadata, face: &FaceMetadata, id: i64) -> Measured {
    let x_em = |f: &FaceMetadata| {
        f.metrics
            .x_height
            .filter(|x| *x > 0)
            .map(|x| f32::from(x) / f32::from(f.metrics.units_per_em.max(1)))
    };
    let scripts_of = |f: &FaceMetadata| -> Vec<String> {
        f.coverage
            .scripts
            .iter()
            .map(|s| s.script.clone())
            .collect()
    };
    let mine = scripts_of(target);
    let shared_scripts: Vec<String> = scripts_of(face)
        .into_iter()
        // The pseudo-scripts are in every font that has any Latin at all, so counting
        // them as something in common would make every pair look compatible.
        .filter(|s| !matches!(s.as_str(), "Zyyy" | "Zinh" | "Zzzz"))
        .filter(|s| mine.iter().any(|m| m == s))
        .collect();
    Measured {
        id,
        label: format!("{} {}", face.names.family, face.names.subfamily),
        weight_delta: face.style.weight - target.style.weight,
        width_delta: face.style.width - target.style.width,
        x_em: x_em(face),
        target_x_em: x_em(target),
        spacing_differs: face.metrics.is_fixed_pitch != target.metrics.is_fixed_pitch,
        shared_scripts,
    }
}

/// The suggestions, while they are on the screen.
pub struct View {
    target: String,
    rows: Vec<Measured>,
    considered: usize,
    capped: bool,
    cursor: usize,
}

impl View {
    /// Rank, and drop what cannot be a pair at all.
    ///
    /// A face sharing no script with the target is not a candidate however its numbers
    /// look: the two cannot set the same sentence, so nothing else about them matters.
    pub fn new(
        target: &FaceMetadata,
        mut rows: Vec<Measured>,
        considered: usize,
        capped: bool,
    ) -> View {
        rows.retain(|r| !r.shared_scripts.is_empty());
        rows.sort_by(|a, b| {
            a.rank()
                .partial_cmp(&b.rank())
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.label.cmp(&b.label))
        });
        View {
            target: format!("{} {}", target.names.family, target.names.subfamily),
            rows,
            considered,
            capped,
            cursor: 0,
        }
    }

    pub fn rows(&self) -> &[Measured] {
        &self.rows
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn cursor(&self) -> usize {
        self.cursor.min(self.rows.len().saturating_sub(1))
    }

    pub fn selected(&self) -> Option<&Measured> {
        self.rows.get(self.cursor())
    }

    pub fn move_cursor(&mut self, delta: i32) {
        if self.rows.is_empty() {
            self.cursor = 0;
            return;
        }
        let last = self.rows.len() as i32 - 1;
        self.cursor = (self.cursor.min(last as usize) as i32 + delta).clamp(0, last) as usize;
    }

    pub fn jump(&mut self, to: usize) {
        self.cursor = to.min(self.rows.len().saturating_sub(1));
    }

    /// What was ranked, against what, and out of how many.
    ///
    /// "Ranked", never "best": the word in the title has to be the same kind of claim
    /// the rows make.
    pub fn title(&self) -> String {
        let capped = if self.capped { " (capped)" } else { "" };
        format!(
            "ranked against {} — {} of {} face(s) share a script{capped}",
            self.target,
            self.rows.len(),
            self.considered
        )
    }

    pub fn row_text(&self, row: &Measured, width: usize) -> String {
        let name_room = 28.min(width / 3);
        format!(
            "{:<name_room$}  {}",
            super::truncate(&row.label, name_room),
            super::truncate(&row.summary(), width.saturating_sub(name_room + 2).max(8))
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> FaceMetadata {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures")
            .join(name);
        let (_, faces) = fontina_core::load_file(&path).expect("a fixture parses");
        faces.into_iter().next().expect("one face")
    }

    fn amiri() -> FaceMetadata {
        fixture("Amiri-Regular.ttf")
    }

    fn inter() -> FaceMetadata {
        fixture("inter-latin-400-normal.woff2")
    }

    fn serif() -> FaceMetadata {
        fixture("SourceSerif4-Regular.otf")
    }

    /// Every suggestion shows the measurements it was ranked on, and none of them is
    /// an adjective.
    #[test]
    fn a_row_says_what_was_measured_and_never_whether_it_is_good() {
        let m = measure(&serif(), &inter(), 1);
        let line = m.summary();
        assert!(line.contains("weight "), "{line}");
        assert!(line.contains("x/em "), "{line}");
        assert!(line.contains("script"), "{line}");
        for word in ["good", "best", "great", "poor", "bad", "ideal", "perfect"] {
            assert!(
                !line.contains(word),
                "{word:?} is taste, not a measurement: {line}"
            );
        }
    }

    /// A weight difference is signed, because "heavier" and "lighter" are different
    /// answers and the sign is the whole of the difference.
    #[test]
    fn the_weight_difference_is_signed() {
        let mut heavy = inter();
        heavy.style.weight = 700.0;
        let m = measure(&serif(), &heavy, 1);
        assert!(m.summary().contains("weight +300"), "{}", m.summary());

        let mut light = inter();
        light.style.weight = 200.0;
        let m = measure(&serif(), &light, 1);
        assert!(m.summary().contains("weight -200"), "{}", m.summary());
    }

    /// An x-height nobody reported is not a zero, and must not be treated as one.
    #[test]
    fn an_unreported_x_height_is_unknown_rather_than_zero() {
        let mut blank = inter();
        blank.metrics.x_height = None;
        let m = measure(&serif(), &blank, 1);
        assert_eq!(m.x_em, None);
        assert_eq!(m.x_height_gap(), None);
        assert!(m.summary().contains("x/em not reported"), "{}", m.summary());
    }

    /// The gate: two faces that cannot set the same text are not a pair, whatever else
    /// is true of them.
    #[test]
    fn a_face_sharing_no_script_is_not_a_candidate() {
        let target = amiri();
        let latin = measure(&target, &inter(), 1);
        assert!(
            latin.shared_scripts.iter().all(|s| s != "Arab"),
            "Inter has no Arabic"
        );
        let view = View::new(&target, vec![latin], 1, false);
        // Amiri covers Latin as well, so Inter does share one; the rule under test is
        // that a row with nothing shared is dropped.
        let nothing = Measured {
            shared_scripts: Vec::new(),
            ..measure(&target, &inter(), 2)
        };
        let view2 = View::new(&target, vec![nothing], 1, false);
        assert!(
            view2.is_empty(),
            "a face with nothing in common was offered"
        );
        assert!(view2.title().contains("0 of 1 face(s) share a script"));
        let _ = view;
    }

    /// The pseudo-scripts are in every font with any Latin in it, so counting them
    /// would make every pair look compatible.
    #[test]
    fn the_pseudo_scripts_do_not_count_as_something_in_common() {
        let m = measure(&serif(), &inter(), 1);
        for pseudo in ["Zyyy", "Zinh", "Zzzz"] {
            assert!(
                !m.shared_scripts.iter().any(|s| s == pseudo),
                "{pseudo} counted as a shared script"
            );
        }
    }

    /// Contrast in weight ranks above sameness, which is the one thing the ordering is
    /// for: twenty plausible candidates instead of four hundred faces.
    #[test]
    fn contrast_ranks_above_sameness() {
        let target = serif();
        let same = {
            let mut f = inter();
            f.style.weight = target.style.weight;
            measure(&target, &f, 1)
        };
        let contrasting = {
            let mut f = inter();
            f.style.weight = target.style.weight + 400.0;
            measure(&target, &f, 2)
        };
        assert!(
            contrasting.rank() < same.rank(),
            "sameness outranked contrast: {} vs {}",
            contrasting.rank(),
            same.rank()
        );

        let view = View::new(&target, vec![same, contrasting], 2, false);
        assert_eq!(view.rows().first().map(|r| r.id), Some(2));
    }

    /// The title is a claim about what was measured, not about what is good.
    #[test]
    fn the_title_says_ranked_and_never_best() {
        let target = serif();
        let view = View::new(&target, vec![measure(&target, &inter(), 1)], 5, true);
        let t = view.title();
        assert!(t.starts_with("ranked against Source Serif"), "{t}");
        assert!(t.contains("of 5 face(s) share a script"), "{t}");
        assert!(t.ends_with("(capped)"), "{t}");
        assert!(!t.contains("best"), "{t}");
    }
}
