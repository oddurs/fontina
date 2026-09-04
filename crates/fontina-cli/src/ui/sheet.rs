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

//! The same words, rendered more than once, down a scrolling sheet.
//!
//! A waterfall is one face at every size in the ladder; a comparison is several faces at
//! one size. They differ only in how the rows are filled, so they are one mode: the
//! scrolling, the labels and the drawing are shared, and a fix to either is a fix to
//! both.

use fontina_core::model::FaceMetadata;
use fontina_core::render::RenderOptions;
use fontina_core::typography;
use ratatui::text::Line;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Waterfall,
    Compare,
}

/// One rendering in the sheet: which face, how big, how it is set, and what to call it.
///
/// The metadata is held rather than the id. A sheet is drawn on every frame, and reading
/// a face back from the index per row per frame is a query and a full JSON parse each
/// time — the mistake #36 fixed for the details pane.
pub struct Row {
    pub face: FaceMetadata,
    pub label: String,
    pub size: f32,
    /// Axis positions and forced features, captured when the sheet was opened. The sheet
    /// is modal, so nothing can change them while it is up.
    pub variations: Vec<(String, f32)>,
    pub features: Vec<(String, bool)>,
}

/// A sheet laid out for one pane width and one sample text.
struct Built {
    width: u16,
    text: Option<String>,
    lines: Vec<Line<'static>>,
}

pub struct Sheet {
    kind: Kind,
    rows: Vec<Row>,
    /// First terminal line on show. The rows have wildly different heights — a 96 px
    /// row is nearly fifty terminal lines and a 10 px row is five — so scrolling counts
    /// lines rather than rows, or a single keypress would jump a screenful.
    scroll: usize,
    /// The rendered sheet, kept until the pane width or the sample text changes.
    /// Rebuilding is nine rasterisations for a waterfall and one per face for a
    /// comparison, which is fine once and ruinous on every frame.
    built: Option<Built>,
}

impl Sheet {
    /// One face at every size in the ladder, set the way the reader left it.
    ///
    /// The controls carry through here and nowhere else: an `opsz` axis walked down a
    /// size ladder is most of what a waterfall is for, and a waterfall is one face, so
    /// the settings mean something for every row.
    pub fn waterfall(
        face: FaceMetadata,
        variations: Vec<(String, f32)>,
        features: Vec<(String, bool)>,
    ) -> Self {
        Sheet {
            kind: Kind::Waterfall,
            rows: typography::WATERFALL_SIZES
                .iter()
                .map(|&size| Row {
                    face: face.clone(),
                    label: format!("{size:.0} px"),
                    size,
                    variations: variations.clone(),
                    features: features.clone(),
                })
                .collect(),
            scroll: 0,
            built: None,
        }
    }

    /// Several faces at one size, in the order the listing had them.
    ///
    /// Deliberately unset: the controls describe the one face the details pane was
    /// showing, and applying its `wght` to every other face in the family would be
    /// meaningless where the axis exists and a lie where it does not.
    pub fn compare(faces: Vec<FaceMetadata>, size: f32) -> Self {
        Sheet {
            kind: Kind::Compare,
            rows: faces
                .into_iter()
                .map(|face| Row {
                    label: format!("{} {}", face.names.family, face.names.subfamily),
                    face,
                    size,
                    variations: Vec::new(),
                    features: Vec::new(),
                })
                .collect(),
            scroll: 0,
            built: None,
        }
    }

    pub fn kind(&self) -> Kind {
        self.kind
    }

    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    pub fn scroll_row(&self) -> usize {
        self.scroll
    }

    /// Terminal lines the built sheet occupies, or none until it has been laid out.
    pub fn lines(&self) -> usize {
        self.built.as_ref().map(|b| b.lines.len()).unwrap_or(0)
    }

    /// Whether the sheet already holds a rendering for this width and sample text.
    pub fn is_built_for(&self, width: u16, text: Option<&str>) -> bool {
        self.built
            .as_ref()
            .is_some_and(|b| b.width == width && b.text.as_deref() == text)
    }

    /// Keep a laid-out sheet. Anything that changes what it should look like — a resize,
    /// new sample text, a different size — drops it and it is built again.
    pub fn set_built(&mut self, width: u16, text: Option<String>, lines: Vec<Line<'static>>) {
        self.built = Some(Built { width, text, lines });
    }

    /// The window of lines to draw, given how many rows the pane can show.
    pub fn window(&self, visible: usize) -> Vec<Line<'static>> {
        let Some(built) = &self.built else {
            return Vec::new();
        };
        built
            .lines
            .iter()
            .skip(self.scroll)
            .take(visible)
            .cloned()
            .collect()
    }

    /// Scroll by whole terminal lines, never past the last screenful.
    pub fn scroll_by(&mut self, delta: i32, visible: usize) {
        let last = self.lines().saturating_sub(visible.max(1));
        self.scroll = (self.scroll as i32 + delta).clamp(0, last as i32) as usize;
    }

    /// The words a row is set in.
    ///
    /// A comparison holds the string constant and varies the face — that is what makes
    /// it a comparison — so every row gets the same text, taken from the first face when
    /// the reader has not chosen any. A waterfall is one face, so it can fall back to
    /// that face's own embedded sample string.
    pub fn text_for(&self, row: &Row, chosen: Option<&str>) -> String {
        if let Some(text) = chosen {
            return text.to_string();
        }
        match self.kind {
            Kind::Compare => self
                .rows
                .first()
                .map(|first| typography::preview_text(&first.face).to_string())
                .unwrap_or_default(),
            Kind::Waterfall => row
                .face
                .names
                .sample_text
                .clone()
                .unwrap_or_else(|| typography::preview_text(&row.face).to_string()),
        }
    }

    /// How each row should be rendered, at the pane width being drawn.
    pub fn options(&self, row: &Row, text: String, width: u32) -> RenderOptions {
        RenderOptions {
            text,
            size: row.size,
            variations: row.variations.clone(),
            features: row.features.clone(),
            padding: 1,
            max_width: Some(width),
        }
    }

    /// Change the size every row is rendered at. Only a comparison has one size to
    /// change; a waterfall's sizes are the point of it.
    pub fn resize(&mut self, delta: f32) -> bool {
        if self.kind != Kind::Compare {
            return false;
        }
        let mut changed = false;
        for row in &mut self.rows {
            let next = (row.size + delta).clamp(8.0, 160.0);
            changed |= next != row.size;
            row.size = next;
        }
        if changed {
            self.scroll = 0;
            self.built = None;
        }
        changed
    }

    /// The size a comparison is set to, for its title.
    pub fn size(&self) -> f32 {
        self.rows.first().map(|r| r.size).unwrap_or(0.0)
    }

    pub fn title(&self) -> String {
        match self.kind {
            Kind::Waterfall => format!("waterfall — {} sizes", self.rows.len()),
            Kind::Compare => format!(
                "compare — {} face(s) at {:.0} px, +/- to resize",
                self.rows.len(),
                self.size()
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn face(name: &str) -> FaceMetadata {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures")
            .join(name);
        fontina_core::load_file(&path).unwrap().1.remove(0)
    }

    fn waterfall() -> Sheet {
        Sheet::waterfall(face("Amiri-Regular.ttf"), Vec::new(), Vec::new())
    }

    #[test]
    fn a_waterfall_is_one_face_at_every_size_in_the_ladder() {
        let s = waterfall();
        assert_eq!(s.kind(), Kind::Waterfall);
        assert_eq!(s.rows().len(), typography::WATERFALL_SIZES.len());
        let names: std::collections::BTreeSet<&str> = s
            .rows()
            .iter()
            .map(|r| r.face.names.family.as_str())
            .collect();
        assert_eq!(names.len(), 1, "one face throughout");
        let sizes: Vec<f32> = s.rows().iter().map(|r| r.size).collect();
        assert_eq!(sizes, typography::WATERFALL_SIZES);
        assert_eq!(s.rows()[0].label, "10 px");
    }

    #[test]
    fn a_waterfall_carries_the_settings_it_was_opened_with() {
        let variations = vec![("wght".to_string(), 700.0)];
        let features = vec![("smcp".to_string(), true)];
        let s = Sheet::waterfall(
            face("Amiri-Regular.ttf"),
            variations.clone(),
            features.clone(),
        );
        assert!(
            s.rows()
                .iter()
                .all(|r| r.variations == variations && r.features == features),
            "every size shows the face as the reader set it"
        );
        let opts = s.options(&s.rows()[0], "Ag".into(), 80);
        assert_eq!(opts.variations, variations);
        assert_eq!(opts.features, features);
    }

    #[test]
    fn a_comparison_is_every_face_at_one_size_and_carries_no_settings() {
        let faces = vec![
            face("Amiri-Regular.ttf"),
            face("SourceSerif4-Regular.otf"),
            face("BricolageGrotesque[opsz,wdth,wght].ttf"),
        ];
        let s = Sheet::compare(faces, 32.0);
        assert_eq!(s.kind(), Kind::Compare);
        assert_eq!(s.rows().len(), 3);
        assert!(
            s.rows().iter().all(|r| r.size == 32.0),
            "one size throughout"
        );
        assert!(
            s.rows()
                .iter()
                .all(|r| r.variations.is_empty() && r.features.is_empty()),
            "one face's axes mean nothing on another's row"
        );
        assert!(s.rows()[0].label.starts_with("Amiri"));
    }

    #[test]
    fn only_a_comparison_can_be_resized() {
        let mut w = waterfall();
        assert!(!w.resize(8.0), "a waterfall's sizes are the point of it");
        assert_eq!(w.rows()[0].size, typography::WATERFALL_SIZES[0]);

        let mut c = Sheet::compare(vec![face("Amiri-Regular.ttf")], 32.0);
        assert!(c.resize(8.0));
        assert_eq!(c.size(), 40.0);
        for _ in 0..100 {
            c.resize(8.0);
        }
        assert_eq!(c.size(), 160.0);
        assert!(!c.resize(8.0), "already at the largest size");
        for _ in 0..100 {
            c.resize(-8.0);
        }
        assert_eq!(c.size(), 8.0);
        assert!(!c.resize(-8.0), "already at the smallest size");
    }

    #[test]
    fn scrolling_stops_at_the_last_screenful() {
        let mut s = waterfall();
        s.set_built(80, None, vec![Line::from("x"); 200]);
        assert_eq!(s.lines(), 200);
        s.scroll_by(-10, 40);
        assert_eq!(s.scroll_row(), 0);
        s.scroll_by(10_000, 40);
        assert_eq!(s.scroll_row(), 160, "the last screenful, not past the end");
        assert_eq!(s.window(40).len(), 40);

        // A pane taller than the sheet cannot scroll at all.
        s.set_built(80, None, vec![Line::from("x"); 20]);
        s.scroll_by(10_000, 40);
        assert_eq!(s.scroll_row(), 0);
        assert_eq!(s.window(40).len(), 20, "the window never invents lines");
    }

    #[test]
    fn a_comparison_sets_every_row_in_the_same_words() {
        let faces = vec![
            face("Amiri-Regular.ttf"),
            face("SourceSerif4-Regular.otf"),
            face("BricolageGrotesque[opsz,wdth,wght].ttf"),
        ];
        let c = Sheet::compare(faces, 32.0);
        let texts: std::collections::BTreeSet<String> =
            c.rows().iter().map(|r| c.text_for(r, None)).collect();
        assert_eq!(
            texts.len(),
            1,
            "varying the words as well as the face compares nothing"
        );
        assert!(c.rows().iter().all(|r| c.text_for(r, Some("Ag")) == "Ag"));
    }

    #[test]
    fn a_waterfall_may_use_the_faces_own_sample_string() {
        let mut f = face("Amiri-Regular.ttf");
        f.names.sample_text = Some("A specimen".into());
        let w = Sheet::waterfall(f, Vec::new(), Vec::new());
        assert_eq!(w.text_for(&w.rows()[0], None), "A specimen");
        assert_eq!(w.text_for(&w.rows()[0], Some("Ag")), "Ag");
    }

    /// Anything that changes what the sheet should look like drops the rendering, and a
    /// sheet that is already laid out for this width and text is not rendered again.
    #[test]
    fn a_built_sheet_is_reused_until_something_changes() {
        let mut c = Sheet::compare(vec![face("Amiri-Regular.ttf")], 32.0);
        assert!(!c.is_built_for(80, None), "nothing rendered yet");
        c.set_built(80, None, vec![Line::from("x"); 50]);
        assert!(c.is_built_for(80, None));
        assert!(!c.is_built_for(100, None), "a resized pane is a new layout");
        assert!(
            !c.is_built_for(80, Some("Ag")),
            "new words are a new layout"
        );

        c.scroll_by(10, 10);
        assert!(c.scroll_row() > 0);
        c.resize(8.0);
        assert!(
            !c.is_built_for(80, None),
            "a resized sheet must be rendered again"
        );
        assert_eq!(c.scroll_row(), 0, "and starts from the top");
        assert_eq!(c.lines(), 0);
    }
}
