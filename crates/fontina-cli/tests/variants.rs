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

//! `fontina variants` through the binary.
//!
//! The similarity itself is tested in `fontina-core`. What is left here is the command:
//! how a target is named, what `--min` refuses, and that the answer is reported rather
//! than thresholded into a verdict.

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

fn fontina(db: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_fontina"))
        .args(["--db", &db.to_string_lossy()])
        .args(args)
        .output()
        .expect("fontina runs")
}

fn ok(db: &Path, args: &[&str]) -> String {
    let o = fontina(db, args);
    assert!(
        o.status.success(),
        "fontina {args:?} failed: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    String::from_utf8(o.stdout).expect("stdout is UTF-8")
}

fn indexed(what: &str) -> (PathBuf, PathBuf) {
    // One directory per test: these run in parallel and would otherwise share, and fight
    // over, a single index.
    let dir = std::env::temp_dir().join(format!("fontina-variants-{what}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let db = dir.join("index.db");
    let o = fontina(&db, &["scan", &fixtures().to_string_lossy()]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    (dir, db)
}

/// The id of the Inter WOFF, whose twin is the WOFF2.
fn inter(db: &Path) -> (i64, i64) {
    let faces: Value =
        serde_json::from_str(&ok(db, &["list", "--json", "--family", "Inter"])).unwrap();
    let ids: Vec<i64> = faces
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["id"].as_i64().unwrap())
        .collect();
    assert_eq!(ids.len(), 2);
    (ids[0], ids[1])
}

#[test]
fn a_target_can_be_an_id_a_family_or_a_path() {
    let (dir, db) = indexed("target");
    let (first, second) = inter(&db);

    let by_id: Value = serde_json::from_str(&ok(
        &db,
        &["variants", &first.to_string(), "--min", "0.99", "--json"],
    ))
    .unwrap();
    assert_eq!(by_id.as_array().unwrap().len(), 1);
    assert_eq!(by_id[0]["face"]["id"], second);

    // A family that stands for several faces answers about the first, and says so on
    // stderr rather than silently picking one.
    let o = fontina(
        &db,
        &["variants", "family:Inter", "--min", "0.99", "--json"],
    );
    assert!(o.status.success());
    assert!(
        String::from_utf8_lossy(&o.stderr).contains("matches 2 faces"),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );

    // A path is a target like any other.
    let path = fixtures()
        .canonicalize()
        .unwrap()
        .join("inter-latin-400-normal.woff");
    let by_path: Value = serde_json::from_str(&ok(
        &db,
        &[
            "variants",
            &path.to_string_lossy(),
            "--min",
            "0.99",
            "--json",
        ],
    ))
    .unwrap();
    assert_eq!(by_path[0]["face"]["id"], second);

    let _ = std::fs::remove_dir_all(&dir);
}

/// `--min` is a similarity, and anything else is a mistake worth stopping for.
#[test]
fn min_outside_zero_to_one_is_refused() {
    let (dir, db) = indexed("min");
    let (first, _) = inter(&db);
    // `--min=-0.5` rather than `--min -0.5`: clap reads a leading `-` as another flag,
    // so the second form never reaches the check being tested here.
    for bad in ["--min=2", "--min=-0.5", "--min=100"] {
        let o = fontina(&db, &["variants", &first.to_string(), bad]);
        assert!(!o.status.success(), "{bad} was accepted");
        assert!(
            String::from_utf8_lossy(&o.stderr).contains("between 0.0 and 1.0"),
            "{bad}: {}",
            String::from_utf8_lossy(&o.stderr)
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// The score is printed beside the metrics, not turned into a verdict.
///
/// §12: high overlap with identical metrics is a variant of one typeface; high overlap
/// with different metrics is two fonts that happen to serve the same languages. The
/// reader draws that line, which they can only do if both numbers are on the page.
#[test]
fn the_score_and_the_metrics_are_both_reported() {
    let (dir, db) = indexed("report");
    let (first, _) = inter(&db);

    let text = ok(&db, &["variants", &first.to_string(), "--min", "0.0"]);
    assert!(
        text.contains("100.00%"),
        "the twin scores exactly 1.0: {text}"
    );
    assert!(text.contains("same"), "{text}");
    assert!(
        text.contains("differ"),
        "and something in the fixtures overlaps without matching: {text}"
    );
    assert!(
        text.contains("overlap") && text.contains("shared") && text.contains("metrics"),
        "the columns are named: {text}"
    );

    // Ordered most alike first.
    let json: Value = serde_json::from_str(&ok(
        &db,
        &["variants", &first.to_string(), "--min", "0.0", "--json"],
    ))
    .unwrap();
    let scores: Vec<f64> = json
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["overlap"].as_f64().unwrap())
        .collect();
    assert!(scores.len() > 2);
    assert!(
        scores.windows(2).all(|w| w[0] >= w[1]),
        "most alike first: {scores:?}"
    );

    // Nothing clears an impossible floor, and that is said in words rather than an
    // empty screen.
    let quiet = ok(&db, &["variants", &first.to_string(), "--min", "1.0"]);
    assert!(
        quiet.contains("100.00%") || quiet.contains("nothing overlaps"),
        "{quiet}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
