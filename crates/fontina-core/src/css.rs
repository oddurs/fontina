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

//! `@font-face` generation. One rule per face, CSS Fonts Level 4 descriptors.

use crate::model::FaceMetadata;
use crate::parse::unicode_range;

/// Write `s` as a CSS string, quotes included.
///
/// CSS Syntax Level 3 §4.3.7: a string ends at its quote, a backslash escapes the next
/// character, and a newline inside one is a parse error that ends the declaration. A
/// family name is whatever the font's `name` table says, so all three are reachable
/// without anyone doing anything strange: escaping only the quote leaves `Fo\` writing
/// `"Fo\"`, whose closing quote is escaped, and the rest of the stylesheet is then
/// inside a string.
///
/// A control character becomes a hex escape with the terminating space the spec calls
/// for, so the character after it cannot be read as part of the escape.
pub fn css_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' | '\\' => {
                out.push('\\');
                out.push(c);
            }
            c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                out.push_str(&format!("\\{:x} ", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// A `file://` URL for a path on this machine.
///
/// Percent-encoding is not decoration. A font may sit at any path its owner likes, and
/// on every system fontina supports that includes `"`, `#`, a space and a newline: `"`
/// ends the URL string, `#` truncates the path at a fragment, and `</style>` in a
/// directory name ends the style element of a specimen and puts the rest of the path
/// into the document as markup. Encoding every byte outside the unreserved set removes
/// the question rather than answering it character by character.
///
/// A Windows path gets the third slash it needs: `file://C:/…` names a host `C:`, while
/// `file:///C:/…` names the drive.
pub fn file_url(path: &str) -> String {
    let path = path.replace('\\', "/");
    let mut out = String::from("file://");
    if !path.starts_with('/') {
        out.push('/');
    }
    for b in path.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' | b':' => {
                out.push(*b as char);
            }
            b => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Render one `@font-face` rule. `src_url` overrides the file path when given (for a
/// custom protocol in the desktop app, or a relative web path).
pub fn font_face_rule(face: &FaceMetadata, src_url: Option<&str>) -> String {
    let css = &face.style.css;
    let url = src_url
        .map(String::from)
        .unwrap_or_else(|| file_url(&face.file.path));
    // Collections: the fragment selects the face per CSS Fonts 4 §4.3.
    let url = if face.file.face_count > 1 {
        format!("{url}#{}", face.index)
    } else {
        url
    };
    let src = format!(
        "url({}) format({})",
        css_string(&url),
        css_string(&css.format)
    );
    let mut out = String::new();
    out.push_str("@font-face {\n");
    out.push_str(&format!("  font-family: {};\n", css_string(&css.family)));
    out.push_str(&format!("  font-style: {};\n", css.style));
    out.push_str(&format!("  font-weight: {};\n", css.weight));
    out.push_str(&format!("  font-stretch: {};\n", css.stretch));
    out.push_str("  font-display: swap;\n");
    out.push_str(&format!("  src: {src};\n"));
    let range = unicode_range(face);
    if !range.is_empty() && face.coverage.ranges.len() <= 512 {
        out.push_str(&format!("  unicode-range: {range};\n"));
    }
    out.push_str("}\n");
    out
}
