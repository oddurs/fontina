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

//! fontina in a pipeline behaves like a Unix program.

#![cfg(unix)]

use std::path::PathBuf;
use std::process::Command;

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

/// `fontina list | head` must not complain.
///
/// Rust ignores `SIGPIPE` before `main`, so a write to a closed pipe comes back as an
/// error and `println!` panics on it. Reading the first line of a long listing is an
/// ordinary thing to do, and it printed "failed printing to stdout: Broken pipe" and
/// exited 101.
#[test]
fn closing_the_pipe_early_is_not_an_error() {
    let dir = std::env::temp_dir().join(format!("fontina-pipe-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let db = dir.join("index.db");

    let bin = env!("CARGO_BIN_EXE_fontina");
    let ok = Command::new(bin)
        .args(["--db", &db.to_string_lossy(), "scan"])
        .arg(fixtures())
        .output()
        .expect("scan runs");
    assert!(
        ok.status.success(),
        "{}",
        String::from_utf8_lossy(&ok.stderr)
    );

    // The output has to be bigger than a pipe buffer, or the writer finishes before the
    // reader goes away and nothing is ever written to a closed pipe. A specimen embeds
    // the font itself, so it is comfortably past 64 KB for one face.
    for args in [
        format!("{bin:?} --db {db:?} specimen 1 -o - | head -c 100"),
        format!("{bin:?} --db {db:?} css 1 | head -1"),
    ] {
        let out = Command::new("sh")
            .arg("-c")
            .arg(&args)
            .output()
            .expect("the pipeline runs");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            !stderr.contains("Broken pipe") && !stderr.contains("panicked"),
            "`{args}` complained: {stderr}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}
