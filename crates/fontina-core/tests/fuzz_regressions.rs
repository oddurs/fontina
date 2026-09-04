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

//! Replay every input the fuzzer has ever found, on stable Rust.
//!
//! `cargo fuzz` needs nightly and a sanitizer runtime, so the fuzz targets are not part
//! of the CI matrix that guards every pull request; `.github/workflows/fuzz.yml` runs
//! them separately. This test is the part that runs everywhere, every time: it takes the
//! files in `fuzz/regressions/`, pushes each through both fuzz entry points, and demands
//! that they *return* — no panic, and inside a time bound, because a hostile font that
//! spins forever is as much a denial of service as one that crashes, and the
//! `catch_unwind` in `scan::parse_paths` catches only the crash.

use flate2::read::GzDecoder;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// Generous enough that a slow shared CI runner never trips it, tight enough that the
/// WOFF header loop of #33 — which never finished at all — does.
const BUDGET: Duration = Duration::from_secs(30);

fn regressions_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/regressions")
}

/// Every regression input, decompressed if it was stored that way.
fn inputs() -> Vec<(String, Vec<u8>)> {
    let dir = regressions_dir();
    let entries = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{} must exist and be readable: {e}", dir.display()));
    let mut out = Vec::new();
    for entry in entries {
        let path = entry.expect("readable directory entry").path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if !path.is_file() || name == "README.md" || name.starts_with('.') {
            continue;
        }
        let raw = std::fs::read(&path).expect("readable regression input");
        let bytes = if name.ends_with(".gz") {
            let mut d = GzDecoder::new(&raw[..]);
            let mut buf = Vec::new();
            d.read_to_end(&mut buf).expect("valid gzip");
            buf
        } else {
            raw
        };
        out.push((name, bytes));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// Run `f` on another thread so that an input which never terminates fails the test
/// instead of hanging the suite. Returns the wall time it took.
fn must_return(what: &str, name: &str, f: impl FnOnce() + Send + 'static) -> Duration {
    let started = Instant::now();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        f();
        let _ = tx.send(());
    });
    match rx.recv_timeout(BUDGET) {
        Ok(()) => started.elapsed(),
        // The sender is dropped without a send only when the closure unwound.
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            panic!("{what} panicked on fuzz/regressions/{name}")
        }
        Err(mpsc::RecvTimeoutError::Timeout) => {
            panic!("{what} did not return within {BUDGET:?} on fuzz/regressions/{name}")
        }
    }
}

/// Every input here, fixed or still open, has to be written down. `fuzz/regressions/open/`
/// holds the findings that are not fixed yet: they are kept out of the replay above and
/// out of the corpus, because an unfixed input would fail this job on every pull request
/// and would abort a fuzzing run before it fuzzed anything. A file nobody can explain is
/// a file nobody will ever dare delete, so the README is the assertion.
#[test]
fn every_regression_input_is_documented() {
    let dir = regressions_dir();
    let readme = std::fs::read_to_string(dir.join("README.md")).expect("regressions README");

    let mut open = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir.join("open")) {
        for entry in entries.filter_map(|e| e.ok()) {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name != "README.md" && !name.starts_with('.') {
                open.push(name);
            }
        }
    }
    open.sort();

    for (name, _) in inputs() {
        assert!(
            readme.contains(&name),
            "fuzz/regressions/{name} is not described in fuzz/regressions/README.md"
        );
    }
    for name in &open {
        assert!(
            readme.contains(name),
            "fuzz/regressions/open/{name} is not described in fuzz/regressions/README.md"
        );
        println!("OPEN finding, not fixed yet: fuzz/regressions/open/{name}");
    }
}

/// The import wrapper: container detection, WOFF/WOFF2 unwrapping, then parsing.
#[test]
fn every_regression_input_returns_from_load_bytes() {
    let inputs = inputs();
    assert!(!inputs.is_empty(), "no regression inputs found");
    for (name, bytes) in inputs {
        let label = name.clone();
        let took = must_return("load_bytes", &name, move || {
            // Either outcome is fine. Returning at all is the whole assertion.
            let _ = fontina_core::load_bytes(&bytes, &label);
        });
        println!("load_bytes  {name}: returned in {took:?}");
    }
}

/// The other fuzz entry point, so a crash found by the `sfnt` target — whose inputs are
/// not required to be a container `load_bytes` would even open — stays fixed too.
#[test]
fn every_regression_input_returns_from_parse_sfnt() {
    use fontina_core::model::{Container, FileInfo};

    let inputs = inputs();
    assert!(!inputs.is_empty(), "no regression inputs found");
    for (name, bytes) in inputs {
        let label = name.clone();
        let took = must_return("parse_sfnt", &name, move || {
            let file = FileInfo {
                path: label,
                size: bytes.len() as u64,
                mtime: 0,
                blake3: String::new(),
                container: Container::Ttf,
                face_count: 0,
            };
            let _ = fontina_core::parse::parse_sfnt(&bytes, &file);
        });
        println!("parse_sfnt  {name}: returned in {took:?}");
    }
}

/// `load_file` is `load_bytes` plus the directory entry, so the two must agree on every
/// fixture. This is what keeps the refactor that created `load_bytes` honest: the fuzz
/// entry point has to be the same import path the CLI runs, or fuzzing it proves nothing.
#[test]
fn load_bytes_matches_load_file_on_every_fixture() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures");
    let mut seen = 0;
    for entry in std::fs::read_dir(&fixtures).expect("fixtures directory") {
        let path = entry.expect("readable directory entry").path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !fontina_core::model::Container::extensions().contains(&ext) {
            continue;
        }
        let name = path.to_string_lossy().into_owned();
        let (from_file, faces_from_file) = fontina_core::load_file(&path).expect("fixture parses");
        let bytes = std::fs::read(&path).expect("readable fixture");
        let (mut from_bytes, mut faces_from_bytes) =
            fontina_core::load_bytes(&bytes, &name).expect("fixture parses");

        // The one field that is allowed to differ, and the only one: a slice has no
        // directory entry, so `load_bytes` reports mtime 0 and documents that it does.
        from_bytes.mtime = from_file.mtime;
        for f in &mut faces_from_bytes {
            f.file.mtime = from_file.mtime;
        }

        // Compare the serialised form: it is the published schema, so it covers every
        // field without needing `PartialEq` on the model types.
        assert_eq!(
            serde_json::to_value(&from_file).unwrap(),
            serde_json::to_value(&from_bytes).unwrap(),
            "{name}: FileInfo differs between load_file and load_bytes"
        );
        assert_eq!(
            serde_json::to_value(&faces_from_file).unwrap(),
            serde_json::to_value(&faces_from_bytes).unwrap(),
            "{name}: faces differ between load_file and load_bytes"
        );
        seen += 1;
    }
    assert!(seen >= 5, "expected the fixture set, found {seen} files");
}
