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

//! What `fontina css` and `fontina specimen` write when the name or the path is not
//! tidy.
//!
//! A family name is whatever a font's `name` table says, and a path is whatever the
//! person who keeps the font chose to call the directory. Neither is under fontina's
//! control, and both are pasted into a stylesheet a browser then parses. The rules a
//! browser applies are CSS Syntax Level 3 §4.3.7 for a string and RFC 8089 for a
//! `file://` URL, so those are the rules held here: not "unusual input does not crash",
//! but "the document a browser gets says what fontina meant".

use fontina_core::css::{css_string, file_url, font_face_rule};
use fontina_core::load_file;
use fontina_core::model::FaceMetadata;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name)
}

fn a_face() -> FaceMetadata {
    load_file(&fixture("Amiri-Regular.ttf")).unwrap().1[0].clone()
}

/// The declaration a rule ends with, with its `  key: ` prefix and `;` removed.
fn value_of<'a>(rule: &'a str, key: &str) -> &'a str {
    rule.lines()
        .find_map(|l| l.trim().strip_prefix(&format!("{key}: ")))
        .unwrap_or_else(|| panic!("no {key} in\n{rule}"))
        .strip_suffix(';')
        .unwrap_or_else(|| panic!("{key} does not end in a semicolon in\n{rule}"))
}

/// Read a CSS string back: the inverse of [`css_string`], by the rules a parser uses.
///
/// Written out rather than reusing anything from the crate, so the test agrees with a
/// browser rather than with the code it is testing. A round trip through this is the
/// evidence that what a browser reads is what fontina put in.
fn unescape(s: &str) -> String {
    let inner = s
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or_else(|| panic!("not a quoted CSS string: {s}"));
    let mut out = String::new();
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            assert!(c != '"', "an unescaped quote ends the string early: {s}");
            out.push(c);
            continue;
        }
        let next = chars.next().expect("a backslash escapes something");
        if next.is_ascii_hexdigit() {
            // A hex escape: up to six digits, ended by a space that is consumed.
            let mut hex = String::from(next);
            for c in chars.by_ref() {
                if c == ' ' {
                    break;
                }
                assert!(
                    c.is_ascii_hexdigit(),
                    "{c} is not part of a hex escape: {s}"
                );
                hex.push(c);
            }
            let n = u32::from_str_radix(&hex, 16).expect("hex digits");
            out.push(char::from_u32(n).expect("a character"));
        } else {
            out.push(next);
        }
    }
    out
}

/// How many `{` and `}` a parser sees, which is not how many the text contains.
fn braces_outside_strings(css: &str) -> (usize, usize) {
    let (mut open, mut close, mut in_string, mut escaped) = (0, 0, false, false);
    for c in css.chars() {
        match c {
            _ if escaped => escaped = false,
            '\\' if in_string => escaped = true,
            '"' => in_string = !in_string,
            '{' if !in_string => open += 1,
            '}' if !in_string => close += 1,
            _ => {}
        }
    }
    assert!(!in_string, "a string was left open:\n{css}");
    (open, close)
}

/// Every family name a font can carry survives into the stylesheet unchanged.
///
/// The one that mattered is a name ending in a backslash. Escaping only the quote wrote
/// `"Fo\"`, whose closing quote was itself escaped, so the string ran on into the next
/// declaration and the rest of the file was swallowed. A newline is the other: CSS calls
/// a raw newline inside a string a parse error, and the browser drops the declaration.
#[test]
fn a_family_name_survives_whatever_the_name_table_says() {
    for name in [
        "Inter",
        "Fo\\",
        "Say \"Hi\"",
        "back\\slash",
        "two\nlines",
        "carriage\rreturn",
        "tab\there",
        "\u{7f}delete",
        "Emoji 🎨 and عربى",
        "}; body { display: none } @font-face { font-family: \"x",
    ] {
        let mut face = a_face();
        face.style.css.family = name.to_string();
        let rule = font_face_rule(&face, None);
        assert_eq!(
            unescape(value_of(&rule, "font-family")),
            name,
            "the name a browser reads back is not the name in the font\n{rule}"
        );
        // One block, not two: nothing in the name opened a block or closed this one.
        // Counted outside strings, because a name is allowed to contain a brace and,
        // quoted properly, a parser never sees it as one.
        assert_eq!(braces_outside_strings(&rule), (1, 1), "{rule}");
        for key in ["font-style", "font-weight", "src"] {
            assert!(
                rule.contains(&format!("  {key}: ")),
                "{key} was swallowed by the family name\n{rule}"
            );
        }
    }
}

/// A path is percent-encoded, so no character in it can be read as syntax.
///
/// `#` truncating the path at a fragment is the quiet one: the rule stays valid, the
/// browser asks for a file that is not there, and the specimen renders in a fallback
/// font with nothing to say why.
#[test]
fn a_path_is_encoded_rather_than_pasted() {
    for path in [
        "/home/me/fonts/Amiri.ttf",
        "/home/me/my \"font\".ttf",
        "/home/me/rev#1/Amiri.ttf",
        "/home/me/</style><script>alert(1)</script>/Amiri.ttf",
        "/home/me/new\nline/Amiri.ttf",
        "/home/me/100%/Amiri.ttf",
        "/home/me/résumé/Amiri.ttf",
        "/home/me/back\\slash/Amiri.ttf",
    ] {
        let mut face = a_face();
        face.file.path = path.to_string();
        let rule = font_face_rule(&face, None);
        let src = value_of(&rule, "src");
        let url = src
            .strip_prefix("url(")
            .and_then(|s| s.split(')').next())
            .unwrap_or_else(|| panic!("no url() in {src}"));
        let url = unescape(url);
        assert!(
            url.starts_with("file:///"),
            "a local file URL has three slashes: {url}"
        );
        // Nothing in the URL can end a string, a URL, a style element or a declaration.
        for bad in ['"', '\'', '<', '>', ' ', '#', '\n', '\r', '\\', '('] {
            assert!(
                !url.contains(bad),
                "{bad:?} reached the stylesheet from {path}: {url}"
            );
        }
        assert_eq!(
            percent_decode(url.strip_prefix("file://").expect("a file URL")),
            path.replace('\\', "/"),
            "the path a browser resolves is not the path the font is at"
        );
    }
}

/// Undo percent-encoding, so the test reads a URL the way a browser resolves one.
fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            let hex = std::str::from_utf8(&b[i + 1..i + 3]).expect("ASCII");
            out.push(u8::from_str_radix(hex, 16).expect("two hex digits"));
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).expect("the encoding round-trips to UTF-8")
}

/// `css_string` and `file_url` on their own terms.
#[test]
fn the_helpers_do_what_they_say() {
    assert_eq!(css_string("plain"), "\"plain\"");
    assert_eq!(css_string("a\"b"), "\"a\\\"b\"");
    assert_eq!(css_string("a\\b"), "\"a\\\\b\"");
    assert_eq!(css_string("a\nb"), "\"a\\a b\"");
    // A hex escape is terminated by a space, so the next character cannot join it.
    assert_eq!(unescape(&css_string("a\nfe")), "a\nfe");

    assert_eq!(file_url("/a/b.ttf"), "file:///a/b.ttf");
    assert_eq!(
        file_url("C:\\Users\\me\\b.ttf"),
        "file:///C:/Users/me/b.ttf"
    );
    assert_eq!(file_url("/a b.ttf"), "file:///a%20b.ttf");
    assert_eq!(file_url("/a#b.ttf"), "file:///a%23b.ttf");
    assert_eq!(file_url("/é.ttf"), "file:///%C3%A9.ttf");
}
