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

//! The configuration file, through the binary a person runs.

use std::path::PathBuf;
use std::process::Command;

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

struct Sandbox {
    dir: PathBuf,
}

impl Sandbox {
    fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("fontina-config-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Sandbox { dir }
    }

    fn write(&self, toml: &str) -> PathBuf {
        let path = self.dir.join("config.toml");
        std::fs::write(&path, toml).unwrap();
        path
    }

    /// Run fontina with this sandbox's config file and nothing of the real environment's.
    fn run(&self, config: Option<&PathBuf>, args: &[&str]) -> std::process::Output {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_fontina"));
        cmd.env_remove("FONTINA_DB");
        match config {
            Some(p) => cmd.env("FONTINA_CONFIG", p),
            // A path that cannot exist, so the test never reads the developer's own file.
            None => cmd.env("FONTINA_CONFIG", self.dir.join("absent.toml")),
        };
        cmd.args(args).output().expect("fontina runs")
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn with_no_file_everything_is_a_default() {
    let sb = Sandbox::new("absent");
    let out = sb.run(None, &["config"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("no file yet"), "{text}");
    for line in text
        .lines()
        .filter(|l| l.contains("preview.") || l.contains("scan."))
    {
        assert!(line.ends_with("default"), "not a default: {line}");
    }
}

#[test]
fn the_example_can_be_saved_and_read_back() {
    let sb = Sandbox::new("example");
    let out = sb.run(None, &["config", "--example"]);
    assert!(out.status.success());
    let path = sb.write(&String::from_utf8_lossy(&out.stdout));
    let out = sb.run(Some(&path), &["config"]);
    assert!(
        out.status.success(),
        "the example fontina prints must be a file fontina reads: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn a_configured_default_reaches_the_command_and_a_flag_overrides_it() {
    let sb = Sandbox::new("preview");
    let db = sb.dir.join("index.db");
    let db = db.to_string_lossy().into_owned();
    let path = sb.write("[preview]\nsize = 12\nprotocol = \"blocks\"\ntext = \"AB\"\n");
    let scan = sb.run(
        Some(&path),
        &["--db", &db, "scan", &fixtures().to_string_lossy()],
    );
    assert!(
        scan.status.success(),
        "{}",
        String::from_utf8_lossy(&scan.stderr)
    );

    let at = |args: &[&str]| -> usize {
        let mut all = vec!["--db", &db, "preview", "family:Amiri"];
        all.extend_from_slice(args);
        let out = sb.run(Some(&path), &all);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        // Half blocks are two pixel rows to a line, so the drawing's height in lines is
        // a direct reading of the size that was used.
        String::from_utf8_lossy(&out.stdout).lines().count()
    };
    let configured = at(&[]);
    let bigger = at(&["--size", "48"]);
    assert!(
        bigger > configured,
        "the flag must override the file: {bigger} lines at 48 px, {configured} at the configured 12"
    );
}

#[test]
fn a_file_that_does_not_parse_is_an_error_naming_the_line() {
    let sb = Sandbox::new("broken");
    let path = sb.write("[preview]\nsize = \n");
    let out = sb.run(Some(&path), &["config"]);
    assert!(!out.status.success(), "a broken file is an error");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("line 2"), "the line is named: {err}");
    assert!(
        err.contains(&path.display().to_string()),
        "the file is named: {err}"
    );
}

#[test]
fn a_key_nobody_recognises_is_refused_rather_than_ignored() {
    let sb = Sandbox::new("typo");
    let path = sb.write("[preview]\nsiez = 12\n");
    let out = sb.run(Some(&path), &["config"]);
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("siez"), "{err}");
    assert!(err.contains("size"), "and what was probably meant: {err}");
}

#[test]
fn a_bare_scan_uses_the_sources_in_the_file() {
    let sb = Sandbox::new("sources");
    let db = sb.dir.join("index.db");
    let db = db.to_string_lossy().into_owned();
    let path = sb.write(&format!(
        "[scan]\nsources = [\"{}\"]\n",
        fixtures().display()
    ));
    let out = sb.run(Some(&path), &["--db", &db, "scan"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let list = sb.run(Some(&path), &["--db", &db, "list"]);
    let text = String::from_utf8_lossy(&list.stdout);
    assert!(
        text.contains("Amiri"),
        "the configured source was scanned: {text}"
    );

    // With no sources and no paths, it still says what to do rather than doing nothing.
    let sb2 = Sandbox::new("nosources");
    let out = sb2.run(None, &["scan"]);
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("scan.sources"),
        "the message offers the file: {err}"
    );
}
