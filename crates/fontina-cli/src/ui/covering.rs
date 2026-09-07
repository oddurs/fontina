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

//! Can anything I own set this sentence?
//!
//! Coverage in the browser was shown per face, as scripts and counts, which answers a
//! question about fonts. The question a designer actually has is about *text*: here is
//! a line, who can set it. `fontina covering` has answered that from the command line
//! since M2; this is the same query with somewhere to type.
//!
//! Two things make the answer worth reading rather than a yes or a no.
//!
//! **A near miss is an answer.** A face that sets every character but two is a face
//! somebody might use anyway, or subset, or pair — and "no" tells them none of that. So
//! a face missing a handful is offered with the handful named, by codepoint and by the
//! character itself.
//!
//! **A mixed-script string fails per script, not as a whole.** A line of English with
//! one Arabic word is two questions, and a face that answers the first and not the
//! second has said something useful. The report is per script for the same reason the
//! index stores coverage per script: "cannot set this" is a summary of several facts
//! and the facts are what a person acts on.

use fontina_core::model::FaceMetadata;
use fontina_core::unicode;

/// A codepoint from the text, and which script Unicode says it belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Want {
    pub cp: u32,
    pub script: &'static str,
}

/// What the reader typed, reduced to the distinct characters that need drawing.
///
/// Whitespace and control characters go: a font that lacks U+0020 still sets the line,
/// and asking about U+000A is asking about nothing. The order is the order they were
/// first seen, so a report reads in the direction the text does.
pub fn wanted(text: &str) -> Vec<Want> {
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    for ch in text.chars() {
        if ch.is_whitespace() || ch.is_control() {
            continue;
        }
        let cp = ch as u32;
        if !seen.insert(cp) {
            continue;
        }
        out.push(Want {
            cp,
            script: script_of(cp),
        });
    }
    out
}

/// The script one codepoint belongs to.
///
/// Asked of the core's own coverage builder, one codepoint at a time. The script table
/// lives there and this is its public door; a table of ranges copied into the browser
/// would be a second answer to a question that already has one, and the two would
/// disagree the first time Unicode added a block.
fn script_of(cp: u32) -> &'static str {
    static NAMES: std::sync::OnceLock<std::sync::Mutex<Vec<&'static str>>> =
        std::sync::OnceLock::new();
    let coverage = unicode::coverage_from_codepoints(vec![cp]);
    let name = coverage
        .scripts
        .first()
        .map(|s| s.script.clone())
        .unwrap_or_else(|| "Zzzz".into());
    // `ScriptCoverage` owns its name and this wants a `'static` one, so the handful of
    // distinct script names a text can hold are leaked once each and reused. A text has
    // at most a few dozen scripts in it and a session asks a few dozen times.
    let cache = NAMES.get_or_init(|| std::sync::Mutex::new(Vec::new()));
    let mut cache = cache.lock().expect("the script name cache");
    match cache.iter().find(|n| **n == name) {
        Some(found) => found,
        None => {
            let leaked: &'static str = Box::leak(name.into_boxed_str());
            cache.push(leaked);
            leaked
        }
    }
}

/// What one face can do with the text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verdict {
    /// Codepoints the face does not cover, in the order the text first wanted them.
    pub missing: Vec<Want>,
    /// Per script: how many of the text's codepoints in that script the face covers,
    /// out of how many the text wants. In the order the text first wanted them.
    pub per_script: Vec<(&'static str, u32, u32)>,
    pub wanted: usize,
}

impl Verdict {
    pub fn covers_everything(&self) -> bool {
        self.missing.is_empty()
    }

    /// The answer in one line, as wide as the pane it is going into.
    pub fn summary(&self, width: usize) -> String {
        if self.covers_everything() {
            return format!("sets all {} character(s)", self.wanted);
        }
        // The scripts it fails on first, because that is the shape of the failure: a
        // face that has the English and not the Arabic has said something a count of
        // missing codepoints does not.
        let failed: Vec<String> = self
            .per_script
            .iter()
            .filter(|(_, have, want)| have < want)
            .map(|(script, have, want)| format!("{script} {have}/{want}"))
            .collect();
        let named = self
            .missing
            .iter()
            .take(4)
            .map(|w| match char::from_u32(w.cp) {
                Some(c) if !c.is_control() => format!("U+{:04X} {c}", w.cp),
                _ => format!("U+{:04X}", w.cp),
            })
            .collect::<Vec<_>>()
            .join(", ");
        let more = self.missing.len().saturating_sub(4);
        let tail = if more > 0 {
            format!("{named} and {more} more")
        } else {
            named
        };
        let line = format!(
            "missing {} of {}: {}   [{}]",
            self.missing.len(),
            self.wanted,
            tail,
            failed.join(", ")
        );
        super::truncate(&line, width)
    }
}

/// The per-script tally for a face that covers everything.
///
/// The same shape as a near miss's, so a row that sets the whole text and one that does
/// not are the same kind of answer rather than two.
pub fn judge_scripts(want: &[Want]) -> Vec<(&'static str, u32, u32)> {
    let mut out: Vec<(&'static str, u32, u32)> = Vec::new();
    for w in want {
        match out.iter_mut().find(|(s, _, _)| *s == w.script) {
            Some(entry) => {
                entry.1 += 1;
                entry.2 += 1;
            }
            None => out.push((w.script, 1, 1)),
        }
    }
    out
}

/// Judge one face against the text.
pub fn judge(face: &FaceMetadata, want: &[Want]) -> Verdict {
    let covers = |cp: u32| {
        face.coverage
            .ranges
            .iter()
            .any(|[lo, hi]| *lo <= cp && cp <= *hi)
    };
    let mut missing = Vec::new();
    let mut per_script: Vec<(&'static str, u32, u32)> = Vec::new();
    for w in want {
        let has = covers(w.cp);
        if !has {
            missing.push(*w);
        }
        match per_script.iter_mut().find(|(s, _, _)| *s == w.script) {
            Some(entry) => {
                entry.1 += u32::from(has);
                entry.2 += 1;
            }
            None => per_script.push((w.script, u32::from(has), 1)),
        }
    }
    Verdict {
        missing,
        per_script,
        wanted: want.len(),
    }
}

/// One row of the answer.
pub struct Row {
    pub id: i64,
    pub label: String,
    pub verdict: Verdict,
}

/// The answer, while it is on the screen.
pub struct View {
    text: String,
    rows: Vec<Row>,
    /// How many of the rows set the whole text. They sort first.
    complete: usize,
    /// Faces considered, and whether that was all of them.
    considered: usize,
    capped: bool,
    cursor: usize,
}

impl View {
    /// Sort so the faces that can set the text come first, then the near misses by how
    /// near they are. A list that puts a face missing forty characters above one
    /// missing two has buried the answer.
    pub fn new(text: &str, mut rows: Vec<Row>, considered: usize, capped: bool) -> View {
        rows.sort_by(|a, b| {
            a.verdict
                .missing
                .len()
                .cmp(&b.verdict.missing.len())
                .then_with(|| a.label.cmp(&b.label))
        });
        let complete = rows
            .iter()
            .filter(|r| r.verdict.covers_everything())
            .count();
        View {
            text: text.to_string(),
            rows,
            complete,
            considered,
            capped,
            cursor: 0,
        }
    }

    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn cursor(&self) -> usize {
        self.cursor.min(self.rows.len().saturating_sub(1))
    }

    pub fn selected(&self) -> Option<&Row> {
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

    /// How many can set it, out of how many were asked — and, when the answer was
    /// bounded, that it was. A number that might be a floor has to say so.
    pub fn title(&self) -> String {
        let capped = if self.capped { " (capped)" } else { "" };
        format!(
            "who can set {:?} — {} of {} face(s){capped}",
            super::truncate(&self.text, 32),
            self.complete,
            self.considered
        )
    }

    pub fn row_text(&self, row: &Row, width: usize) -> String {
        let mark = if row.verdict.covers_everything() {
            "✓ "
        } else {
            "  "
        };
        let name = format!("{mark}{}", row.label);
        let name_room = 30.min(width / 3);
        format!(
            "{:<name_room$}  {}",
            super::truncate(&name, name_room),
            row.verdict
                .summary(width.saturating_sub(name_room + 2).max(8))
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A real parsed face with its coverage replaced.
    ///
    /// A parsed fixture rather than a hand-built struct: `FaceMetadata` has no
    /// `Default`, and inventing one field by field would be inventing a font. Only the
    /// ranges matter here, and they are the one thing set explicitly.
    fn face_covering(ranges: &[[u32; 2]]) -> FaceMetadata {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/inter-latin-400-normal.woff2");
        let (_, faces) = fontina_core::load_file(&path).expect("a fixture parses");
        let mut face = faces.into_iter().next().expect("one face");
        face.coverage.ranges = ranges.to_vec();
        face
    }

    #[test]
    fn whitespace_and_controls_are_not_characters_a_font_has_to_have() {
        let want = wanted("ab \n\tab");
        assert_eq!(want.len(), 2, "{want:?}");
        assert_eq!(want[0].cp, 'a' as u32);
        assert_eq!(want[1].cp, 'b' as u32);
    }

    #[test]
    fn each_codepoint_carries_the_script_unicode_gives_it() {
        let want = wanted("aس1");
        assert_eq!(want[0].script, "Latn");
        assert_eq!(want[1].script, "Arab");
        // Digits are common to every script, which is what Zyyy means.
        assert_eq!(want[2].script, "Zyyy");
    }

    /// The whole point of a near miss: the characters are named, not counted.
    #[test]
    fn a_face_missing_a_handful_is_offered_with_the_handful_named() {
        let want = wanted("abc");
        let v = judge(&face_covering(&[['a' as u32, 'b' as u32]]), &want);
        assert!(!v.covers_everything());
        assert_eq!(v.missing.len(), 1);
        let line = v.summary(120);
        assert!(line.starts_with("missing 1 of 3: U+0063 c"), "{line}");
    }

    /// A mixed-script string fails per script, because a face that has the English and
    /// not the Arabic has said something a count of missing codepoints does not.
    #[test]
    fn a_mixed_script_string_reports_per_script() {
        let want = wanted("abس");
        let v = judge(&face_covering(&[['a' as u32, 'b' as u32]]), &want);
        assert_eq!(v.per_script, vec![("Latn", 2, 2), ("Arab", 0, 1)]);
        let line = v.summary(120);
        assert!(line.contains("[Arab 0/1]"), "{line}");
        assert!(
            !line.contains("Latn"),
            "a script it satisfies is not a failure: {line}"
        );
    }

    #[test]
    fn a_face_that_sets_everything_says_so_and_counts_nothing_missing() {
        let want = wanted("abc");
        let v = judge(&face_covering(&[['a' as u32, 'c' as u32]]), &want);
        assert!(v.covers_everything());
        assert_eq!(v.summary(120), "sets all 3 character(s)");
    }

    /// A long list of missing characters is cut, and says it was.
    #[test]
    fn many_missing_characters_are_named_up_to_a_point() {
        let want = wanted("abcdefgh");
        let v = judge(&face_covering(&[]), &want);
        let line = v.summary(200);
        assert!(line.contains("and 4 more"), "{line}");
    }

    fn row(label: &str, missing: usize) -> Row {
        let want = wanted("abcdefgh");
        let covered = 8 - missing;
        Row {
            id: 1,
            label: label.into(),
            verdict: judge(
                &face_covering(&[['a' as u32, 'a' as u32 + covered as u32 - 1]]),
                &want,
            ),
        }
    }

    /// A list that puts a face missing forty characters above one missing two has
    /// buried the answer.
    #[test]
    fn the_faces_that_can_set_it_come_first_then_the_nearest_misses() {
        let v = View::new(
            "abcdefgh",
            vec![row("far", 5), row("all", 0), row("near", 1)],
            3,
            false,
        );
        let order: Vec<&str> = v.rows().iter().map(|r| r.label.as_str()).collect();
        assert_eq!(order, ["all", "near", "far"]);
        assert!(
            v.title()
                .starts_with("who can set \"abcdefgh\" — 1 of 3 face(s)"),
            "{}",
            v.title()
        );
    }

    /// A number that might be a floor has to say it is one.
    #[test]
    fn a_bounded_answer_says_that_it_was_bounded() {
        let v = View::new("ab", vec![row("a", 0)], 200, true);
        assert!(v.title().ends_with("(capped)"), "{}", v.title());
        let v = View::new("ab", vec![row("a", 0)], 3, false);
        assert!(!v.title().contains("capped"), "{}", v.title());
    }
}
