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

//! What the reader has set on one face: where each variable axis sits, and which
//! OpenType features are forced on.
//!
//! Kept apart from drawing and from key handling so the behaviour that is easy to get
//! wrong — clamping, stepping without accumulating float error, deciding when a set of
//! coordinates is a named instance — can be tested without a terminal.
//!
//! Every axis is held, including the ones a font marks hidden. A named instance's
//! coordinates cover the whole axis list, so dropping the hidden ones would leave two
//! vectors of different lengths that could never match, and instance snapping would
//! silently stop working on exactly the fonts that use hidden axes most.

use fontina_core::model::{FaceMetadata, InstanceInfo};
use fontina_core::typography;

/// One variable axis, and where the reader has put it.
pub struct Axis {
    pub tag: String,
    pub label: String,
    pub min: f32,
    pub max: f32,
    pub default: f32,
    pub step: f32,
    pub value: f32,
}

/// One feature the reader can force on.
pub struct Feature {
    pub tag: String,
    pub label: &'static str,
    pub on: bool,
}

/// A row of the controls pane: the shown axes first, then the features.
pub enum Row<'a> {
    Axis(&'a Axis),
    Feature(&'a Feature),
}

#[derive(Default)]
pub struct Controls {
    /// Every axis the font declares, in its own order, so coordinates line up with
    /// `InstanceInfo::coordinates`.
    axes: Vec<Axis>,
    /// Indices into `axes` that the reader can see and move.
    shown: Vec<usize>,
    features: Vec<Feature>,
    instances: Vec<InstanceInfo>,
    /// The instance last snapped to, cleared by any manual change. Remembering the index
    /// rather than re-deriving it from the coordinates keeps `n`/`p` walking in order
    /// even when a font declares an instance outside its own axis range.
    instance: Option<usize>,
    cursor: usize,
}

impl Controls {
    /// The controls a face offers.
    pub fn for_face(face: &FaceMetadata) -> Self {
        let declared = || face.variable.iter().flat_map(|v| &v.axes);
        let axes: Vec<Axis> = declared()
            .map(|a| Axis {
                tag: a.tag.clone(),
                label: a.name.clone().unwrap_or_else(|| a.tag.clone()),
                min: a.min,
                max: a.max,
                default: a.default,
                step: typography::axis_step(a),
                value: a.default,
            })
            .collect();
        let shown = declared()
            .enumerate()
            .filter(|(_, a)| !a.hidden)
            .map(|(i, _)| i)
            .collect();
        let features = typography::toggleable_features(&face.features)
            .into_iter()
            .map(|tag| Feature {
                tag: tag.to_string(),
                label: typography::feature_label(tag).unwrap_or(""),
                on: false,
            })
            .collect();
        let instances = face
            .variable
            .as_ref()
            .map(|v| v.instances.clone())
            .unwrap_or_default();
        Controls {
            axes,
            shown,
            features,
            instances,
            instance: None,
            cursor: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Rows a reader can move between: the shown axes, then the features.
    pub fn len(&self) -> usize {
        self.shown.len() + self.features.len()
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn rows(&self) -> impl Iterator<Item = Row<'_>> {
        self.shown
            .iter()
            .map(|i| Row::Axis(&self.axes[*i]))
            .chain(self.features.iter().map(Row::Feature))
    }

    /// Move the cursor, stopping at the ends rather than wrapping: a reader holding a
    /// key down should arrive somewhere and stay there.
    pub fn move_cursor(&mut self, delta: i32) {
        if self.is_empty() {
            return;
        }
        let last = self.len() - 1;
        self.cursor = (self.cursor as i32 + delta).clamp(0, last as i32) as usize;
    }

    /// Move the selected axis by `steps` of its own step size. Does nothing on a
    /// feature row. Returns whether anything changed.
    pub fn adjust(&mut self, steps: i32) -> bool {
        let Some(&i) = self.shown.get(self.cursor) else {
            return false;
        };
        let axis = &mut self.axes[i];
        let before = axis.value;
        // Step from the axis minimum rather than from the current value, so a hundred
        // presses do not drift the way repeated addition of 0.1 would.
        let offset = ((axis.value - axis.min) / axis.step).round() + steps as f32;
        axis.value = (axis.min + offset * axis.step).clamp(axis.min, axis.max);
        let moved = axis.value != before;
        if moved {
            self.instance = None;
        }
        moved
    }

    /// Flip the selected feature. Does nothing on an axis row.
    pub fn toggle(&mut self) -> bool {
        let Some(i) = self.cursor.checked_sub(self.shown.len()) else {
            return false;
        };
        match self.features.get_mut(i) {
            Some(f) => {
                f.on = !f.on;
                true
            }
            None => false,
        }
    }

    /// Every axis back to its default, every feature off.
    pub fn reset(&mut self) {
        for a in &mut self.axes {
            a.value = a.default;
        }
        for f in &mut self.features {
            f.on = false;
        }
        self.instance = None;
    }

    /// Jump to the next named instance in `dir`, wrapping. Named instances are the
    /// styles the designer actually drew, so they are worth reaching in one key.
    pub fn cycle_instance(&mut self, dir: i32) -> bool {
        if self.instances.is_empty() || self.axes.is_empty() {
            return false;
        }
        let n = self.instances.len() as i32;
        // Prefer the remembered index; fall back to whichever instance the coordinates
        // sit on, so snapping still works the first time after the controls are built.
        let current = self.instance.map(|i| i as i32).or_else(|| {
            let coords = self.coords();
            self.instances
                .iter()
                .position(|i| i.coordinates == coords)
                .map(|i| i as i32)
        });
        let next = match current {
            Some(i) => (i + dir).rem_euclid(n),
            // From a custom setting, `next` means the first instance and `previous` the
            // last, so one key always lands somewhere.
            None if dir >= 0 => 0,
            None => n - 1,
        };
        let target = &self.instances[next as usize];
        if target.coordinates.len() != self.axes.len() {
            return false;
        }
        for (axis, value) in self.axes.iter_mut().zip(&target.coordinates) {
            axis.value = value.clamp(axis.min, axis.max);
        }
        self.instance = Some(next as usize);
        true
    }

    /// Current coordinates, in axis order and covering every axis, so they can be
    /// compared with an instance's own.
    pub fn coords(&self) -> Vec<f32> {
        self.axes.iter().map(|a| a.value).collect()
    }

    /// The name of the instance the axes are sitting on, if they are on one.
    pub fn instance_name<'a>(&self, face: &'a FaceMetadata) -> Option<&'a str> {
        let v = face.variable.as_ref()?;
        typography::matching_instance(v, &self.coords())?
            .name
            .as_deref()
    }

    /// Whether the face has any axis at all, shown or hidden: the difference between
    /// "custom" being a meaningful label for the pane and being nonsense.
    pub fn is_variable(&self) -> bool {
        !self.axes.is_empty()
    }

    /// Axis settings for `RenderOptions`, covering hidden axes too: the renderer is told
    /// the whole position, not a diff.
    pub fn variations(&self) -> Vec<(String, f32)> {
        self.axes.iter().map(|a| (a.tag.clone(), a.value)).collect()
    }

    /// Features to force on. A feature the reader has not turned on is left out entirely
    /// rather than forced off, so the shaper's own defaults still apply.
    pub fn forced_features(&self) -> Vec<(String, bool)> {
        self.features
            .iter()
            .filter(|f| f.on)
            .map(|f| (f.tag.clone(), true))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fontina_core::model::AxisInfo;
    use std::path::PathBuf;

    fn face(name: &str) -> FaceMetadata {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures")
            .join(name);
        fontina_core::load_file(&path).unwrap().1.remove(0)
    }

    fn variable() -> FaceMetadata {
        face("BricolageGrotesque[opsz,wdth,wght].ttf")
    }

    /// No fixture declares a hidden axis, so one is added to a real variable face, with
    /// every instance extended to cover it exactly as a font's own would.
    fn with_hidden_axis() -> FaceMetadata {
        let mut f = variable();
        let v = f.variable.as_mut().unwrap();
        v.axes.push(AxisInfo {
            tag: "HIDE".into(),
            name: Some("Hidden".into()),
            min: 0.0,
            default: 5.0,
            max: 10.0,
            hidden: true,
        });
        for inst in &mut v.instances {
            inst.coordinates.push(5.0);
        }
        f
    }

    #[test]
    fn a_static_face_offers_features_but_no_axes() {
        let c = Controls::for_face(&face("Amiri-Regular.ttf"));
        assert!(!c.is_variable(), "Amiri is not variable");
        assert_eq!(c.len(), c.features.len());
    }

    #[test]
    fn a_face_with_nothing_to_set_offers_no_controls() {
        let mut f = face("Amiri-Regular.ttf");
        f.features.gsub.clear();
        f.variable = None;
        let mut c = Controls::for_face(&f);
        assert!(c.is_empty());
        // And nothing panics when a reader is somehow pointed at it.
        c.move_cursor(3);
        assert_eq!(c.cursor(), 0);
        assert!(!c.adjust(1));
        assert!(!c.toggle());
        assert!(!c.cycle_instance(1));
    }

    #[test]
    fn a_variable_face_offers_its_axes_at_their_defaults() {
        let f = variable();
        let c = Controls::for_face(&f);
        let tags: Vec<&str> = c.shown.iter().map(|i| c.axes[*i].tag.as_str()).collect();
        assert_eq!(tags, ["opsz", "wght", "wdth"]);
        assert_eq!(
            c.coords(),
            typography::default_coords(f.variable.as_ref().unwrap())
        );
    }

    #[test]
    fn a_hidden_axis_is_kept_but_not_shown() {
        let f = with_hidden_axis();
        let c = Controls::for_face(&f);
        assert_eq!(c.axes.len(), 4, "every axis is held");
        assert_eq!(c.shown.len(), 3, "the hidden one is not offered");
        assert_eq!(c.coords().len(), 4, "coordinates cover every axis");
        assert!(
            c.variations().iter().any(|(tag, _)| tag == "HIDE"),
            "the renderer is told the whole position"
        );
    }

    #[test]
    fn instances_still_work_when_an_axis_is_hidden() {
        let f = with_hidden_axis();
        let mut c = Controls::for_face(&f);
        assert!(
            c.cycle_instance(1),
            "cycling must not bail on a length check"
        );
        assert!(
            c.instance_name(&f).is_some(),
            "the pane must be able to name the instance it landed on"
        );
    }

    #[test]
    fn adjusting_clamps_and_does_not_drift() {
        let mut c = Controls::for_face(&variable());
        let i = c.shown[0];
        let (min, max, step) = (c.axes[i].min, c.axes[i].max, c.axes[i].step);

        for _ in 0..10_000 {
            c.adjust(-1);
        }
        assert_eq!(c.axes[i].value, min);
        assert!(!c.adjust(-1), "already at the minimum");

        // A hundred steps up lands exactly a hundred steps up, not a float smear.
        for _ in 0..100 {
            c.adjust(1);
        }
        assert_eq!(c.axes[i].value, (min + 100.0 * step).min(max));

        for _ in 0..10_000 {
            c.adjust(1);
        }
        assert_eq!(c.axes[i].value, max);
        assert!(!c.adjust(1), "already at the maximum");
    }

    #[test]
    fn the_cursor_walks_axes_then_features_and_stops_at_the_ends() {
        let mut c = Controls::for_face(&variable());
        assert!(!c.is_empty());
        c.move_cursor(-5);
        assert_eq!(c.cursor(), 0);
        c.move_cursor(10_000);
        assert_eq!(c.cursor(), c.len() - 1);
    }

    #[test]
    fn adjust_and_toggle_each_ignore_the_other_kind_of_row() {
        let mut c = Controls::for_face(&variable());
        assert!(
            !c.features.is_empty(),
            "this fixture has features to toggle"
        );
        c.cursor = 0;
        assert!(!c.toggle(), "Space does nothing on an axis");
        c.cursor = c.shown.len();
        assert!(!c.adjust(1), "the arrows do nothing on a feature");
        assert!(c.toggle());
        assert_eq!(c.forced_features().len(), 1);
        assert!(c.toggle());
        assert!(
            c.forced_features().is_empty(),
            "a feature switched off is left out, not forced off"
        );
    }

    #[test]
    fn cycling_walks_every_instance_in_order_and_wraps() {
        let f = variable();
        let mut c = Controls::for_face(&f);
        let n = f.variable.as_ref().unwrap().instances.len();
        assert!(n > 1, "this fixture has instances to walk");

        assert!(c.cycle_instance(1));
        let first = c.coords();
        for _ in 0..n {
            c.cycle_instance(1);
        }
        assert_eq!(c.coords(), first, "a full lap returns to the start");

        c.cycle_instance(-1);
        c.cycle_instance(1);
        assert_eq!(c.coords(), first, "backwards then forwards is a no-op");
    }

    #[test]
    fn reset_returns_every_axis_to_its_default() {
        let f = variable();
        let mut c = Controls::for_face(&f);
        c.cycle_instance(1);
        c.reset();
        assert_eq!(
            c.coords(),
            typography::default_coords(f.variable.as_ref().unwrap())
        );
    }

    #[test]
    fn a_hand_set_axis_is_not_called_by_an_instance_name() {
        let f = variable();
        let mut c = Controls::for_face(&f);
        c.cycle_instance(1);
        assert!(c.instance_name(&f).is_some());
        // One step off a named instance is a setting the reader chose, not that style.
        c.cursor = 0;
        assert!(c.adjust(-1));
        assert!(c.instance_name(&f).is_none());
        // The next cycle continues the walk rather than restarting it.
        assert!(c.cycle_instance(1));
        assert!(c.instance_name(&f).is_some());
    }
}
