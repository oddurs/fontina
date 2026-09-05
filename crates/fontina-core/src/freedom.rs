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

//! Does a font's license grant the four freedoms — to run it, study it, change it, and
//! redistribute it, changed or not?
//!
//! A font is a work of practical use, so the criterion is the same one that applies to
//! software: this module follows the FSF's list of free licenses, not any weaker
//! "available source" test. Where the FSF has not ruled on a license, the answer is
//! [`Freedom::Unknown`] and the reason says so. Guessing would be worse than admitting
//! it, because the user is deciding whether they may modify and pass on the font.
//!
//! The verdict is always derived from the SPDX identifier that [`crate::license`]
//! recognised; it is never stored in the index. Recomputing it on read means a font's
//! classification tracks this table rather than whatever the table said on the day the
//! index was built.
//!
//! Nothing here consults `OS/2.fsType`. Those bits are a technical restriction the font
//! file asserts about itself, not a grant or a refusal of permission, and honouring them
//! as if they were a license would be enforcing a restriction on the user's own
//! computer. `fontina` reports them (see [`crate::model::EmbeddingRights`]) and stops
//! there.

use serde::{Deserialize, Serialize};

/// Whether a license grants the four freedoms.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Default,
    Serialize,
    Deserialize,
    schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Freedom {
    /// The license grants all four freedoms.
    Free,
    /// The license withholds at least one of them.
    Nonfree,
    /// A license is stated, but it is not one that is known to be free.
    Unknown,
    /// The font carries no license information at all. No permission is stated, so in
    /// law none is granted, however the font was meant to be shared.
    #[default]
    Unstated,
}

impl Freedom {
    pub fn as_str(self) -> &'static str {
        match self {
            Freedom::Free => "free",
            Freedom::Nonfree => "nonfree",
            Freedom::Unknown => "unknown",
            Freedom::Unstated => "unstated",
        }
    }

    pub fn is_free(self) -> bool {
        self == Freedom::Free
    }

    /// Every value, in the order reports should list them.
    pub const ALL: [Freedom; 4] = [
        Freedom::Free,
        Freedom::Nonfree,
        Freedom::Unknown,
        Freedom::Unstated,
    ];
}

impl std::fmt::Display for Freedom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for Freedom {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "free" => Ok(Freedom::Free),
            "nonfree" | "non-free" => Ok(Freedom::Nonfree),
            "unknown" => Ok(Freedom::Unknown),
            "unstated" | "none" => Ok(Freedom::Unstated),
            other => Err(format!(
                "unknown freedom {other:?}; use free, nonfree, unknown or unstated"
            )),
        }
    }
}

/// A verdict and the one-line reason behind it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Verdict {
    pub freedom: Freedom,
    pub reason: &'static str,
}

/// SPDX identifiers for licenses the FSF lists as free. A `WITH <exception>` suffix is
/// stripped before lookup: a font exception only ever widens permission, so it cannot
/// turn a free license into a nonfree one. A trailing `+` is stripped too, so the
/// deprecated `GPL-2.0+` form lands on `GPL-2.0`, which is listed here alongside the
/// current identifiers because font metadata is full of the older spellings.
pub const FREE: &[&str] = &[
    "0BSD",
    "AGPL-3.0",
    "AGPL-3.0-only",
    "AGPL-3.0-or-later",
    "Apache-2.0",
    "Arphic-1999",
    "Artistic-2.0",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "BSL-1.0",
    "Bitstream-Vera",
    "CC-BY-4.0",
    "CC-BY-SA-4.0",
    "CC0-1.0",
    "GPL-2.0",
    "GPL-2.0-only",
    "GPL-2.0-or-later",
    "GPL-3.0-only",
    "GPL-3.0-or-later",
    "GPL-1.0",
    "GPL-3.0",
    "ISC",
    "LGPL-2.0",
    "LGPL-2.1",
    "LGPL-2.1-only",
    "LGPL-2.1-or-later",
    "LGPL-3.0",
    "LGPL-3.0-only",
    "LGPL-3.0-or-later",
    "LicenseRef-Public-Domain",
    "MIT",
    "MPL-2.0",
    "OFL-1.0",
    "OFL-1.1",
    "Unlicense",
    "X11",
    "Zlib",
];

/// Identifiers that stand for a refusal: the font may be used as supplied and no more.
pub const NONFREE: &[&str] = &["LicenseRef-Proprietary"];

/// Licenses that are stated plainly but that the FSF has not ruled on, with the reason
/// to print. These are [`Freedom::Unknown`], not free by assumption.
const UNRULED: &[(&str, &str)] = &[(
    "UFL-1.0",
    "the Ubuntu Font Licence is not on the FSF's list of free licenses; \
     its rename-on-modification clause has never been ruled on",
)];

/// Classify an SPDX identifier or expression. `None` means the font embedded no license
/// information at all.
pub fn assess(spdx: Option<&str>) -> Verdict {
    match spdx.map(str::trim).filter(|s| !s.is_empty()) {
        None => Verdict {
            freedom: Freedom::Unstated,
            reason: "no license is embedded, so no permission is stated and none is granted",
        },
        Some(expr) => expression(expr),
    }
}

/// The verdict alone, for callers that do not print a reason.
pub fn classify(spdx: Option<&str>) -> Freedom {
    assess(spdx).freedom
}

/// `A OR B` takes the most permissive operand, since the user may pick either; `A AND B`
/// takes the most restrictive, since both bind at once.
fn expression(expr: &str) -> Verdict {
    let split = |sep: &str| -> Option<Vec<Verdict>> {
        let parts: Vec<&str> = expr.split(sep).collect();
        (parts.len() > 1).then(|| parts.iter().map(|p| expression(p.trim())).collect())
    };
    // Both `expect`s are unreachable: `split` returns `Some` only when it produced more
    // than one part, so the iterator is never empty.
    #[expect(clippy::expect_used, reason = "split() returns Some only when len > 1")]
    if let Some(parts) = split(" OR ") {
        return parts
            .into_iter()
            .min_by_key(|v| rank(v.freedom))
            .expect("split yields at least two parts");
    }
    #[expect(clippy::expect_used, reason = "split() returns Some only when len > 1")]
    if let Some(parts) = split(" AND ") {
        return parts
            .into_iter()
            .max_by_key(|v| rank(v.freedom))
            .expect("split yields at least two parts");
    }
    identifier(expr)
}

/// Lower is more permissive.
fn rank(f: Freedom) -> u8 {
    match f {
        Freedom::Free => 0,
        Freedom::Unknown => 1,
        Freedom::Unstated => 2,
        Freedom::Nonfree => 3,
    }
}

fn identifier(id: &str) -> Verdict {
    let base = id
        .split_once(" WITH ")
        .map(|(l, _)| l)
        .unwrap_or(id)
        .trim()
        .trim_end_matches('+');
    let eq = |candidate: &&str| candidate.eq_ignore_ascii_case(base);
    if FREE.iter().any(eq) {
        return Verdict {
            freedom: Freedom::Free,
            reason: "a free license: it grants the freedom to run, study, change and \
                     redistribute the font",
        };
    }
    if NONFREE.iter().any(eq) {
        return Verdict {
            freedom: Freedom::Nonfree,
            reason: "the license withholds the freedom to change or redistribute the font",
        };
    }
    if let Some((_, why)) = UNRULED.iter().find(|(k, _)| k.eq_ignore_ascii_case(base)) {
        return Verdict {
            freedom: Freedom::Unknown,
            reason: why,
        };
    }
    if base.eq_ignore_ascii_case("LicenseRef-Unknown") {
        return Verdict {
            freedom: Freedom::Unknown,
            reason: "license text is embedded but was not recognised; read it yourself",
        };
    }
    Verdict {
        freedom: Freedom::Unknown,
        reason: "not a license known to be free; check the FSF's license list",
    }
}

/// A SQL fragment matching `column` against every identifier in `list`, case-insensitively
/// and ignoring any `WITH <exception>` suffix. Used to filter the index by freedom without
/// storing a derived column that could drift from [`FREE`] and [`NONFREE`].
pub fn sql_in(column: &str, list: &[&str]) -> String {
    let ids = list
        .iter()
        .map(|id| format!("'{}'", id.to_ascii_lowercase()))
        .collect::<Vec<_>>()
        .join(", ");
    // `rtrim(x, '+')` and the ` WITH ` cut mirror `identifier()` above.
    format!(
        "rtrim(lower(CASE WHEN instr({column}, ' WITH ') > 0 \
         THEN substr({column}, 1, instr({column}, ' WITH ') - 1) ELSE {column} END), '+') IN ({ids})"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_licenses_are_free() {
        for id in ["OFL-1.1", "Apache-2.0", "GPL-3.0-only", "MIT", "CC0-1.0"] {
            assert_eq!(classify(Some(id)), Freedom::Free, "{id}");
        }
    }

    #[test]
    fn a_font_exception_does_not_change_the_verdict() {
        assert_eq!(
            classify(Some("GPL-2.0-only WITH Font-exception-2.0")),
            Freedom::Free
        );
        assert_eq!(classify(Some("GPL-2.0+")), Freedom::Free);
        assert_eq!(classify(Some("ofl-1.1")), Freedom::Free);
    }

    #[test]
    fn proprietary_is_nonfree() {
        let v = assess(Some("LicenseRef-Proprietary"));
        assert_eq!(v.freedom, Freedom::Nonfree);
        assert!(v.reason.contains("withholds"));
    }

    #[test]
    fn unrecognised_and_unruled_licenses_are_unknown() {
        assert_eq!(classify(Some("LicenseRef-Unknown")), Freedom::Unknown);
        assert_eq!(classify(Some("Whatever-1.0")), Freedom::Unknown);
        let ufl = assess(Some("UFL-1.0"));
        assert_eq!(ufl.freedom, Freedom::Unknown);
        assert!(ufl.reason.contains("FSF"));
    }

    #[test]
    fn no_license_is_unstated() {
        assert_eq!(classify(None), Freedom::Unstated);
        assert_eq!(classify(Some("   ")), Freedom::Unstated);
    }

    #[test]
    fn expressions_take_the_permissive_or_and_the_restrictive_and() {
        assert_eq!(
            classify(Some("MIT OR LicenseRef-Proprietary")),
            Freedom::Free
        );
        assert_eq!(
            classify(Some("MIT AND LicenseRef-Proprietary")),
            Freedom::Nonfree
        );
        assert_eq!(classify(Some("MIT AND Apache-2.0")), Freedom::Free);
    }

    #[test]
    fn every_state_round_trips_through_its_name() {
        for f in Freedom::ALL {
            assert_eq!(f.as_str().parse::<Freedom>().unwrap(), f);
        }
    }
}
