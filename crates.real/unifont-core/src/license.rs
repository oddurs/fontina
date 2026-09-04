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

#[cfg(test)]
mod tests {
    use super::*;

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
