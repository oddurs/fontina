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

//! The tables line up whatever the font is called.
//!
//! A terminal is a grid of columns, and Rust's `{:<w$}` pads to a count of characters.
//! The two agree only for scripts where one character is one column, which is a
//! description of Latin and of nothing else. A Japanese family name takes two columns
//! per character, so every column to its right lands two places further along on that
//! row than on the row above, and a table of fonts with mixed names stops being a table
//! at the first one.
//!
//! There is no CJK fixture — the free ones are megabytes — so the font is made here, by
//! rewriting the name table of one that is already in `fixtures/`. `Amiri` in UTF-16BE
//! is ten bytes and so is any five-character Japanese name, so every string keeps its
//! length and every offset in the table stays where it was.

use std::path::{Path, PathBuf};
use std::process::Command;

/// A five-character Japanese name, ten columns wide and ten bytes in UTF-16BE.
const CJK: &str = "源ノ角ゴシ";

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

/// A copy of the Amiri fixture whose every `Amiri` name reads [`CJK`] instead.
fn cjk_font(to: &Path) {
    let mut bytes = std::fs::read(fixtures().join("Amiri-Regular.ttf")).expect("the fixture");
    let old: Vec<u8> = "Amiri".encode_utf16().flat_map(u16::to_be_bytes).collect();
    let new: Vec<u8> = CJK.encode_utf16().flat_map(u16::to_be_bytes).collect();
    assert_eq!(
        old.len(),
        new.len(),
        "the replacement has to be the same size"
    );

    let mut found = 0;
    let mut i = 0;
    while i + old.len() <= bytes.len() {
        if bytes[i..i + old.len()] == old[..] {
            bytes[i..i + old.len()].copy_from_slice(&new);
            found += 1;
            i += old.len();
        } else {
            i += 1;
        }
    }
    assert!(found > 0, "no UTF-16 name records to rewrite");
    std::fs::write(to, bytes).expect("writing the renamed font");
}

struct Session {
    root: PathBuf,
    fonts: PathBuf,
    db: PathBuf,
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// A sandbox with the Latin fixture and a Japanese-named copy of it, scanned.
fn session(name: &str) -> Session {
    let root = std::env::temp_dir().join(format!("fontina-table-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let fonts = root.join("fonts");
    std::fs::create_dir_all(&fonts).unwrap();
    std::fs::copy(
        fixtures().join("SourceSerif4-Regular.otf"),
        fonts.join("SourceSerif4-Regular.otf"),
    )
    .unwrap();
    cjk_font(&fonts.join("cjk.ttf"));

    let s = Session {
        db: root.join("index.db"),
        fonts,
        root,
    };
    let out = s.run(&["scan", &s.fonts.to_string_lossy()]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    s
}

impl Session {
    fn run(&self, args: &[&str]) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_fontina"))
            .args(["--db", &self.db.to_string_lossy()])
            .args(args)
            .env("HOME", &self.root)
            .env("XDG_CONFIG_HOME", self.root.join(".config"))
            .env("XDG_DATA_HOME", self.root.join(".local/share"))
            .output()
            .expect("fontina runs")
    }

    #[track_caller]
    fn ok(&self, args: &[&str]) -> String {
        let o = self.run(args);
        assert!(
            o.status.success(),
            "`fontina {}` failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&o.stderr)
        );
        String::from_utf8_lossy(&o.stdout).into_owned()
    }
}

/// Where `needle` starts, counted in terminal columns rather than in bytes.
fn column_of(line: &str, needle: &str) -> usize {
    let at = line
        .find(needle)
        .unwrap_or_else(|| panic!("{needle:?} is not in {line:?}"));
    fontina_core::unicode::columns(&line[..at])
}

/// Every row of `list` puts the path in the same column, Japanese name or not.
#[test]
fn the_face_table_lines_up_with_a_japanese_family_name() {
    let s = session("faces");
    let listed = s.ok(&["list"]);
    let lines: Vec<&str> = listed.lines().collect();
    let header = lines[0];
    assert!(header.contains("family"), "{listed}");
    let want = column_of(header, "path");

    let rows: Vec<&str> = lines[1..lines.len() - 1].to_vec();
    assert_eq!(rows.len(), 2, "two faces were scanned:\n{listed}");
    assert!(
        rows.iter().any(|r| r.contains(CJK)),
        "the renamed font is in the table:\n{listed}"
    );
    // The path is the last column, and it starts with the sandbox's own directory. A
    // separator would do on GNU/Linux and macOS and not on Windows, where there is no
    // `/` in a path at all.
    // Canonical, because that is what the index stores and what the row prints: on macOS
    // the temporary directory is reached through a symlink, and the uncanonical form is
    // a substring of the canonical one starting eight columns later.
    let prefix = std::fs::canonicalize(&s.fonts)
        .unwrap_or_else(|_| s.fonts.clone())
        .to_string_lossy()
        .into_owned();
    for row in &rows {
        assert_eq!(
            column_of(row, &prefix),
            want,
            "the path column moved on this row:\n{listed}"
        );
    }
}

/// And so does `families`, which measures the same names for its own column.
#[test]
fn the_family_table_lines_up_too() {
    let s = session("families");
    let listed = s.ok(&["families"]);
    let lines: Vec<&str> = listed.lines().collect();
    let want = column_of(lines[0], "faces");
    for row in &lines[1..] {
        if row.trim().is_empty() || row.contains("famil") {
            continue;
        }
        // The face count is the next column; it is right-aligned, so its own last
        // character is what lines up.
        let end = fontina_core::unicode::columns(row.split("  ").next().unwrap_or(row));
        assert!(
            end <= want,
            "the name column overran into the count on this row:\n{listed}"
        );
    }
    assert!(lines.iter().any(|l| l.contains(CJK)), "{listed}");
}

/// A tag is named by the person, so it is user input, and a person may type anything.
#[test]
fn the_tag_table_lines_up_with_a_japanese_tag() {
    let s = session("tags");
    let listed: serde_json::Value = serde_json::from_str(&s.ok(&["list", "--json"])).unwrap();
    let ids: Vec<String> = listed
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["id"].to_string())
        .collect();

    s.ok(&["tag", "add", "serif", &ids[0]]);
    s.ok(&["tag", "add", "日本語", &ids[1]]);

    let table = s.ok(&["tag", "list"]);
    let counts: Vec<usize> = table
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            // The count is the last field; where it ends is where the column ends.
            fontina_core::unicode::columns(l)
        })
        .collect();
    assert_eq!(counts.len(), 2, "{table}");
    assert_eq!(
        counts[0], counts[1],
        "the count column moved between an ASCII tag and a Japanese one:\n{table}"
    );
}

/// A path longer than its column is printed whole, not cut short.
///
/// `fontina dirs` is how a script asks where an install goes, and `scripts/acceptance`
/// is one such script: it greps the install target out of this listing and then looks
/// for the font there. A temporary home on macOS is longer than sixty columns, so
/// truncating the column meant handing a caller a path that does not exist. The same
/// goes for a source, a tag and a collection: the name is the answer, not a label.
#[test]
fn a_path_or_a_name_is_never_cut_short_to_fit_its_column() {
    let s = session("whole-path");
    let deep = s
        .root
        .join("a-directory-with-a-deliberately-long-name/and-another-one-below-it/and-a-third");
    std::fs::create_dir_all(&deep).unwrap();
    let deep_str = deep.to_string_lossy().into_owned();
    assert!(
        fontina_core::unicode::columns(&deep_str) > 60,
        "the path has to be longer than the column: {deep_str}"
    );

    s.ok(&["source", "add", &deep_str]);
    let listed = s.ok(&["source", "list"]);
    assert!(
        listed.contains(&deep_str),
        "the source listing cut the path short:\n{listed}"
    );
    assert!(!listed.contains('…'), "{listed}");

    let long_tag = "a-tag-name-that-somebody-typed-and-is-longer-than-thirty-columns";
    let ids: serde_json::Value = serde_json::from_str(&s.ok(&["list", "--json"])).unwrap();
    let id = ids[0]["id"].to_string();
    s.ok(&["tag", "add", long_tag, &id]);
    let tags = s.ok(&["tag", "list"]);
    assert!(
        tags.contains(long_tag),
        "the tag listing cut the name short:\n{tags}"
    );

    let dirs = s.ok(&["dirs"]);
    assert!(
        !dirs.contains('…'),
        "a font directory was cut short:\n{dirs}"
    );
}

/// A name that would reverse the line, or move the cursor, prints as neither.
///
/// `fit` stands a replacement character in for anything with no column of its own. A
/// `name` table can hold U+202E RIGHT-TO-LEFT OVERRIDE, and a terminal that is handed one
/// reverses everything after it on the line: the id column would appear to the right of
/// the path. Nothing about a font entitles it to rearrange the program's output.
#[test]
fn a_family_name_cannot_rearrange_the_row() {
    assert_eq!(fontina_core::unicode::columns("源ノ角ゴシ"), 10);
    assert_eq!(fontina_core::unicode::columns("Amiri"), 5);
    // A combining mark has no column of its own and is shown as one replacement.
    assert_eq!(fontina_core::unicode::columns("A\u{0303}"), 2);
    let safe = fontina_core::unicode::fit("a\u{202e}b", 8);
    assert!(!safe.contains('\u{202e}'), "{safe:?}");
    assert_eq!(fontina_core::unicode::columns(&safe), 3);
    // Truncation counts columns and never splits a wide character.
    assert_eq!(fontina_core::unicode::fit("源ノ角ゴシ", 5), "源ノ…");
    assert_eq!(
        fontina_core::unicode::columns(&fontina_core::unicode::fit("源ノ角ゴシ", 5)),
        5
    );
    assert_eq!(fontina_core::unicode::fit("Amiri", 10), "Amiri");
}
