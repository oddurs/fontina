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

//! Map embedded license text/URLs (name IDs 13 and 14) to SPDX identifiers.

/// Returns an SPDX identifier or expression when the license text or URL is recognised.
/// Returns `LicenseRef-Unknown` when text exists but matches nothing, `None` when no
/// license information is present at all.
pub fn spdx_from_names(description: Option<&str>, url: Option<&str>) -> Option<String> {
    let text = format!(
        "{} {}",
        description.unwrap_or_default(),
        url.unwrap_or_default()
    )
    .to_ascii_lowercase();
    if text.trim().is_empty() {
        return None;
    }
    let has = |needle: &str| text.contains(needle);

    let id = if has("scripts.sil.org/ofl")
        || has("openfontlicense.org")
        || has("sil open font license")
        || has("open font license")
        || has("sil ofl")
    {
        "OFL-1.1"
    } else if has("apache license") || has("apache.org/licenses/license-2.0") || has("apache-2.0") {
        "Apache-2.0"
    } else if has("ubuntu font licence")
        || has("ubuntu font license")
        || has("ubuntu.com/legal/font-licence")
    {
        "UFL-1.0"
    } else if has("gnu general public license") || has("gpl") {
        let v3 = has("version 3") || has("gplv3") || has("gpl-3");
        let exc = has("font exception") || has("font-exception") || has("as a special exception");
        match (v3, exc) {
            (true, true) => "GPL-3.0-only WITH Font-exception-2.0",
            (true, false) => "GPL-3.0-only",
            (false, true) => "GPL-2.0-only WITH Font-exception-2.0",
            (false, false) => "GPL-2.0-only",
        }
    } else if has("gnu lesser general public license") || has("lgpl") {
        "LGPL-2.1-only"
    } else if has("bitstream vera") {
        "Bitstream-Vera"
    } else if has("creativecommons.org/publicdomain/zero") || has("cc0") {
        "CC0-1.0"
    } else if has("creativecommons.org/licenses/by/") || has("cc by") || has("cc-by") {
        "CC-BY-4.0"
    } else if has("mit license") || has("opensource.org/licenses/mit") {
        "MIT"
    } else if has("public domain") {
        "LicenseRef-Public-Domain"
    } else if has("all rights reserved")
        || has("commercial")
        || has("end user license")
        || has("eula")
        || has("may not be")
        || has("not permitted")
    {
        "LicenseRef-Proprietary"
    } else {
        "LicenseRef-Unknown"
    };
    Some(id.to_string())
}

/// Extract OFL "Reserved Font Name" declarations from copyright/license text.
/// Handles `Reserved Font Name "Foo"`, `Reserved Font Names "Foo" and "Bar"`,
/// `Reserved Font Name Foo.` and comma lists.
pub fn reserved_font_names(texts: &[Option<&str>]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for text in texts.iter().flatten() {
        let lower = text.to_ascii_lowercase();
        let mut from = 0;
        while let Some(pos) = lower[from..].find("reserved font name") {
            let start = from + pos + "reserved font name".len();
            let rest = &text[start..];
            let rest = rest.trim_start_matches(['s', 'S']);
            let rest = rest.trim_start_matches(|c: char| c == ':' || c.is_whitespace());
            // Take up to a sentence end or a line break.
            let end = rest
                .find(['.', '\n', '\r', ';'])
                .map(|i| i.min(200))
                .unwrap_or(rest.len().min(200));
            let clause = &rest[..end];
            let quoted: Vec<String> = clause
                .split(['"', '\u{201C}', '\u{201D}', '\u{2018}', '\u{2019}', '\''])
                .enumerate()
                .filter(|(i, _)| i % 2 == 1)
                .map(|(_, s)| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            let names: Vec<String> = if quoted.is_empty() {
                clause
                    .split([',', '&'])
                    .flat_map(|part| part.split(" and "))
                    .map(|s| {
                        s.trim()
                            .trim_matches(|c: char| c == '\'' || c == '"')
                            .to_string()
                    })
                    .filter(|s| !s.is_empty() && s.len() < 60)
                    .collect()
            } else {
                quoted
            };
            for n in names {
                if !out.iter().any(|o| o.eq_ignore_ascii_case(&n)) {
                    out.push(n);
                }
            }
            from = start;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_reserved_font_names() {
        let t = "Copyright 2010 The Amiri Project Authors, with Reserved Font Name \"Amiri\".";
        assert_eq!(reserved_font_names(&[Some(t)]), vec!["Amiri"]);
        let t = "Copyright (c) 2011, Foo Bar (foo@example.com), with Reserved Font Names \"Foo\" and \"Foo Sans\".";
        assert_eq!(reserved_font_names(&[Some(t)]), vec!["Foo", "Foo Sans"]);
        let t = "with Reserved Font Name Ubuntu.";
        assert_eq!(reserved_font_names(&[Some(t)]), vec!["Ubuntu"]);
        assert!(reserved_font_names(&[Some("no names here"), None]).is_empty());
    }

    #[test]
    fn recognises_common_licenses() {
        assert_eq!(
            spdx_from_names(
                Some(
                    "This Font Software is licensed under the SIL Open Font License, Version 1.1."
                ),
                None
            )
            .as_deref(),
            Some("OFL-1.1")
        );
        assert_eq!(
            spdx_from_names(None, Some("http://www.apache.org/licenses/LICENSE-2.0")).as_deref(),
            Some("Apache-2.0")
        );
        assert_eq!(
            spdx_from_names(
                Some("GNU General Public License version 2 with font exception"),
                None
            )
            .as_deref(),
            Some("GPL-2.0-only WITH Font-exception-2.0")
        );
        assert_eq!(spdx_from_names(None, None), None);
        assert_eq!(
            spdx_from_names(Some("Some text"), None).as_deref(),
            Some("LicenseRef-Unknown")
        );
    }
}
