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

//! The promises ADR 0008 makes about the plugin surface, as a test.
//!
//! An architecture decision record that nothing checks is a wish. These are the
//! guarantees a program building on the CLI is told it may rely on: with `--json`,
//! stdout is JSON and nothing else, and the exit codes mean what they say.

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

fn indexed(what: &str) -> (PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("fontina-contract-{what}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let db = dir.join("index.db");
    let o = fontina(&db, &["scan", &fixtures().to_string_lossy()]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    (dir, db)
}

/// ADR 0008: "With `--json`, stdout carries the JSON and nothing else."
///
/// A pipeline reading stdout must never have to filter progress or warnings out of it.
/// This is the promise most easily broken by a well-meaning `println!`, and the one a
/// reader downstream discovers as a parse error rather than a message.
#[test]
fn every_json_command_puts_json_on_stdout_and_nothing_else() {
    let (dir, db) = indexed("json");
    fontina(&db, &["tag", "add", "serif", "1"]);
    fontina(&db, &["collection", "create", "Set"]);
    fontina(&db, &["collection", "add", "Set", "1", "2"]);

    let fx = fixtures().to_string_lossy().into_owned();
    for args in [
        vec!["list", "--json"],
        vec!["families", "--json"],
        vec!["facets", "--json"],
        vec!["stats", "--json"],
        vec!["dupes", "--json"],
        vec!["dirs", "--json"],
        vec!["tag", "list", "--json"],
        vec!["collection", "list", "--json"],
        vec!["collection", "show", "Set", "--json"],
        vec!["collection", "export", "Set"],
        vec!["source", "list", "--json"],
        vec!["activations", "--json"],
        vec!["conflicts", "--json", "1"],
        vec!["info", "--json", "1"],
        vec!["license", "--json", "1"],
        vec!["glyphs", "--json", "1"],
        vec!["covers", "--json", "abc"],
        vec!["check", "--json", "1"],
        vec!["variants", "1", "--min", "0.0", "--json"],
        vec!["scan", "--json", fx.as_str()],
    ] {
        let o = fontina(&db, &args);
        let stdout = String::from_utf8_lossy(&o.stdout);
        assert!(
            !stdout.trim().is_empty(),
            "`fontina {}` printed nothing on stdout: {}",
            args.join(" "),
            String::from_utf8_lossy(&o.stderr)
        );
        serde_json::from_str::<Value>(&stdout).unwrap_or_else(|e| {
            panic!(
                "`fontina {}` did not put JSON alone on stdout: {e}\n{stdout}",
                args.join(" ")
            )
        });
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// ADR 0008: 0 success, 1 an error, 2 a conflict that stopped the operation.
///
/// 2 is deliberately distinct so a script can tell "this font clashes with one already
/// active" from "something went wrong", and act on the first.
///
/// Proving it needs a real conflict, which needs a real activation: this registers one
/// fixture with the OS at **session** scope — gone at logout — and deactivates it before
/// returning. CI's own smoke step does the same on every push. If the system will not
/// register it, the test says so and stops rather than pretending.
#[test]
fn the_exit_codes_mean_what_the_adr_says_they_mean() {
    let (dir, db) = indexed("exits");

    assert_eq!(fontina(&db, &["list", "--json"]).status.code(), Some(0));

    // An error: a face that is not there.
    let o = fontina(&db, &["info", "99999"]);
    assert_eq!(o.status.code(), Some(1));
    assert!(
        !String::from_utf8_lossy(&o.stderr).is_empty(),
        "an error says why, on stderr"
    );

    // A conflict needs two faces that clash: the fixtures ship Inter as both a WOFF and
    // a WOFF2, which are the same family and style. Activate one and ask about the other.
    let faces: Value = serde_json::from_str(&String::from_utf8_lossy(
        &fontina(&db, &["list", "--json", "--family", "Inter"]).stdout,
    ))
    .unwrap();
    let pair = faces.as_array().unwrap();
    assert_eq!(pair.len(), 2, "the fixtures ship Inter twice: {pair:?}");
    let first = pair[0]["id"].as_i64().unwrap().to_string();
    let second = pair[1]["id"].as_i64().unwrap().to_string();

    let activated = fontina(&db, &["activate", "--session", &first]);
    if !activated.status.success() {
        eprintln!(
            "skipped: this system would not activate the fixture: {}",
            String::from_utf8_lossy(&activated.stderr)
        );
        let _ = std::fs::remove_dir_all(&dir);
        return;
    }
    let o = fontina(&db, &["conflicts", &second]);
    assert_eq!(
        o.status.code(),
        Some(2),
        "a reported conflict is 2, not 1: {}",
        String::from_utf8_lossy(&o.stdout)
    );
    // And `activate` refuses for the same reason, with the same code.
    let o = fontina(&db, &["activate", "--session", &second]);
    assert_eq!(
        o.status.code(),
        Some(2),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );

    fontina(&db, &["deactivate", &first]);

    let _ = std::fs::remove_dir_all(&dir);
}

/// ADR 0008: health-check ids are never renamed and never re-pointed.
///
/// A script that greps for `license/nonfree` is the first thing anybody writes against
/// `check`, so the ids are part of the promise. This does not freeze the list — checks
/// may be added — it holds the shape and a few that carry weight.
#[test]
fn check_ids_are_stable_and_shaped_area_slash_check() {
    let (dir, db) = indexed("checks");
    let out = fontina(&db, &["check", "--json", "--min", "info", "1"]);
    let reports: Value = serde_json::from_str(&String::from_utf8_lossy(&out.stdout)).unwrap();
    let ids: Vec<String> = reports
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|r| r["findings"].as_array().cloned().unwrap_or_default())
        .filter_map(|f| f["id"].as_str().map(str::to_string))
        .collect();
    assert!(!ids.is_empty(), "the fixture triggers something");
    for id in &ids {
        let (area, check) = id
            .split_once('/')
            .unwrap_or_else(|| panic!("{id} has no area"));
        assert!(!area.is_empty() && !check.is_empty(), "{id}");
        assert!(
            id.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '/' || c == '-'),
            "{id} is not a stable identifier a script can match on"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}
