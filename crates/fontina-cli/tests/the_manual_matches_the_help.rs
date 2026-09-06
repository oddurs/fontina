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

//! The manual's chapter on the browser cannot fall behind the browser.
//!
//! It did, in a day. The chapter's key table was written from the help overlay on a
//! Thursday; by Friday the browser had grown marked selections, an undo, a command
//! palette and a pane policy, and the table mentioned none of them — while the *frame*
//! on the same page, which is read from a snapshot, showed all four. A manual that
//! contradicts the screenshot beside it is worse than no manual.
//!
//! Nothing about the site's own build catches this: `site.yml` runs on `site/**` and on
//! the snapshots, so a pull request that only changes the browser never builds the site
//! at all. `cargo test` does run, which is why this lives here.
//!
//! What it holds is the *subjects*, not the wording: every line of the help overlay
//! introduces one — Move, Filter, Select, Undo, Commands — and the chapter has to cover
//! each. Keys and their descriptions are the chapter's own business; a subject the
//! program has and the manual does not is a hole.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("{} must exist and be readable: {e}", path.display()))
}

/// The help overlay as it is drawn, out of the snapshot a test asserts on.
///
/// The snapshot rather than the source: it is what a reader sees, it is already pinned,
/// and it is the same file the web site reads for the picture beside this text.
fn help_overlay() -> String {
    read(&repo_root().join(
        "crates/fontina-cli/src/ui/snapshots/fontina__ui__tests__the_help_overlay_sits_over_the_browser.snap",
    ))
}

/// The overlay's own text, one line per row, with the browser behind it cut away.
///
/// The overlay is a box drawn over the browser, so every row of the frame carries the
/// pane behind it, the overlay's own left border, its text, and its right border. The
/// left border is at one column on every row — the last corner on the title row, since
/// the pane behind draws a corner of its own further left — so it is found once and
/// every row is cut there.
fn overlay_lines(overlay: &str) -> Vec<String> {
    // Columns, not bytes: a box-drawing character is three bytes and one column, and
    // there are a hundred of them to the left of the text on every row.
    let left = overlay
        .lines()
        .filter(|l| l.contains("fontina ui"))
        .find_map(|l| {
            l.chars()
                .collect::<Vec<_>>()
                .iter()
                .rposition(|c| *c == '╭')
        })
        .expect("the overlay's title row draws its top-left corner");

    let mut out = Vec::new();
    for line in overlay.lines() {
        let mut chars = line.chars().skip(left);
        if !matches!(chars.next(), Some('│' | '╭' | '╰')) {
            continue;
        }
        let inside: String = chars
            .take_while(|c| !matches!(c, '│' | '╯' | '╮'))
            .collect();
        let text = inside.trim().to_string();
        if !text.is_empty() {
            out.push(text);
        }
    }
    out
}

/// The overlay's section headings: MOVING, FINDING, DOING, LOOKING, THE PROGRAM.
///
/// Not something the manual is held to — a chapter writes sentences, and no sentence
/// says "DOING". They are here so that a layout change loud enough to lose them fails
/// this test rather than quietly emptying the set of keys below it.
fn sections(overlay: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for line in overlay_lines(overlay) {
        // `DOING — to the marks when there are any` is a heading with a clause after it.
        let head = line
            .split('—')
            .next()
            .unwrap_or_default()
            .trim()
            .to_string();
        if !head.is_empty()
            && head.chars().all(|c| c.is_ascii_uppercase() || c == ' ')
            && head.chars().any(|c| c.is_ascii_uppercase())
        {
            out.insert(head);
        }
    }
    out
}

/// The key each line of the overlay leads with.
///
/// Every line that is not a heading begins with the key it is about — `m  the glyph
/// map`, `U  undo the last change`, `:  every command` — which is a contract a test can
/// hold without reading prose. The few lines that describe a policy rather than a key
/// lead with a word, and are skipped by the same rule that finds the others.
fn keys(overlay: &str) -> BTreeSet<String> {
    let headings = sections(overlay);
    let mut out = BTreeSet::new();
    for line in overlay_lines(overlay) {
        if headings.iter().any(|h| line.starts_with(h.as_str())) {
            continue;
        }
        let Some(first) = line.split_whitespace().next() else {
            continue;
        };
        // `j/k` is two keys; `Ctrl-S` is one; `The` and `any` are prose.
        for part in first.split('/') {
            let named = part.starts_with("Ctrl-");
            if named || part.chars().count() == 1 {
                out.insert(part.to_string());
            }
        }
    }
    out
}

/// What the manual calls a key the overlay draws as a glyph.
///
/// The overlay has one line to spend and draws the key; the chapter writes a sentence
/// and names it. Neither is wrong, so the test knows both.
const DRAWN_AS: &[(&str, &str)] = &[
    ("⏎", "Enter"),
    ("␣", "Space"),
    ("⌫", "Backspace"),
    ("⇥", "Tab"),
];

#[test]
fn every_key_the_help_overlay_leads_with_is_in_the_manual() {
    let overlay = help_overlay();

    // The sections that were there when this was written, so that a *lost* one fails
    // rather than quietly shrinking the set the manual is held to.
    let sections = sections(&overlay);
    for expected in ["MOVING", "FINDING", "DOING", "LOOKING", "THE PROGRAM"] {
        assert!(
            sections.contains(expected),
            "the help overlay no longer has a {expected:?} section. If the browser \
             dropped it, drop it here; if it was renamed, rename it here. Sections \
             found: {sections:?}"
        );
    }

    let keys = keys(&overlay);
    assert!(
        keys.len() > 10,
        "only {} key(s) found in the help overlay; the parser has lost the layout:\n{overlay}",
        keys.len()
    );

    let chapter_path = repo_root().join("site/src/content/docs/terminal.md");
    // Two copies on purpose: named keys are matched case-insensitively, single keys are
    // not. `u` uninstalls and `U` undoes, and a manual that documents one is not
    // documenting the other.
    let chapter = read(&chapter_path);
    let chapter_lower = chapter.to_lowercase();

    let mut unlisted: Vec<String> = Vec::new();
    for key in &keys {
        let found = match DRAWN_AS.iter().find(|(glyph, _)| glyph == key) {
            // A glyph the chapter spells out: it writes the word, not the picture.
            Some((_, word)) => chapter_lower.contains(&word.to_lowercase()),
            // As the manual writes an ordinary key: in backticks.
            None if key.chars().count() == 1 => chapter.contains(&format!("`{key}`")),
            None => chapter_lower.contains(&key.to_lowercase()),
        };
        if !found {
            unlisted.push(key.clone());
        }
    }

    assert!(
        unlisted.is_empty(),
        "the help overlay leads a line with {unlisted:?} and {} never mentions it. The \
         chapter is what a person reads before they ever press `?`; a key the program \
         has and the manual does not is a hole. Add it there, or take the line out of \
         the overlay.",
        chapter_path.display()
    );
}
