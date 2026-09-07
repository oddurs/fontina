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

//! Nothing that is not a terminal ever sees an escape.
//!
//! A test process has no terminal, which makes this the easy half to be sure of: every
//! command here runs into a pipe, and not one byte of any of them may be an escape. The
//! other half — that a terminal *does* get colours — is what `CLICOLOR_FORCE` is for,
//! and it is checked with the same commands so the two can be compared line for line.
//!
//! The rule is worth a test file of its own because breaking it is silent. A stray
//! escape in `--json` is a parse error somewhere else, tomorrow; a stray escape in a
//! table is a column that no longer lines up for anybody piping into `awk`.

use std::path::PathBuf;
use std::process::Command;

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

struct Session {
    db: PathBuf,
    root: PathBuf,
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn session(name: &str) -> Session {
    let root = std::env::temp_dir().join(format!("fontina-colour-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("a scratch directory");
    let s = Session {
        db: root.join("index.db"),
        root,
    };
    let out = s.run(&["scan", &fixtures().to_string_lossy()], &[]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    s
}

impl Session {
    fn run(&self, args: &[&str], env: &[(&str, &str)]) -> std::process::Output {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_fontina"));
        cmd.args(["--db", &self.db.to_string_lossy()])
            .args(args)
            .env("HOME", &self.root)
            .env("XDG_CONFIG_HOME", self.root.join(".config"))
            .env("XDG_DATA_HOME", self.root.join(".local/share"))
            .env_remove("NO_COLOR")
            .env_remove("CLICOLOR_FORCE");
        for (k, v) in env {
            cmd.env(k, v);
        }
        cmd.output().expect("fontina runs")
    }

    #[track_caller]
    fn output(&self, args: &[&str], env: &[(&str, &str)]) -> String {
        let o = self.run(args, env);
        let both = format!(
            "{}{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        );
        assert!(
            o.status.success() || args[0] == "check",
            "`fontina {}` failed: {both}",
            args.join(" ")
        );
        both
    }
}

/// One of everything that prints for a person to read. `{id}` is filled in with a face
/// the index really has: ids are handed out by the scan and start wherever it left off.
const READABLE: &[&[&str]] = &[
    &["list"],
    &["families"],
    &["facets"],
    &["stats"],
    &["dirs"],
    &["dupes"],
    &["activations"],
    &["config"],
    &["info", "{id}"],
    &["license"],
    &["glyphs", "{id}"],
    &["check", "{id}"],
    &["variants", "{id}"],
    &["covers", "Hamburgefonstiv"],
];

impl Session {
    /// A face id the index actually holds.
    fn a_face(&self) -> String {
        let json = self.output(&["list", "--json"], &[]);
        let at = json.find("\"id\": ").expect("a face in the index") + 6;
        json[at..]
            .split(|c: char| !c.is_ascii_digit())
            .next()
            .expect("an id")
            .to_string()
    }

    /// Every readable command, with `{id}` filled in.
    fn readable(&self) -> Vec<Vec<String>> {
        let id = self.a_face();
        READABLE
            .iter()
            .map(|args| {
                args.iter()
                    .map(|a| {
                        if *a == "{id}" {
                            id.clone()
                        } else {
                            a.to_string()
                        }
                    })
                    .collect()
            })
            .collect()
    }
}

/// `&[String]` as the `&[&str]` every runner here takes.
fn argv(args: &[String]) -> Vec<&str> {
    args.iter().map(String::as_str).collect()
}

/// The rule, in one assertion: a pipe gets no escapes, from any command, ever.
#[test]
fn nothing_piped_carries_an_escape() {
    let s = session("piped");
    for args in s.readable() {
        let args = argv(&args);
        let out = s.output(&args, &[]);
        assert!(
            !out.contains('\x1b'),
            "`fontina {}` wrote an escape into a pipe:\n{out}",
            args.join(" ")
        );
    }
}

/// And with `CLICOLOR_FORCE` every one of them is coloured, so the palette cannot rot
/// into a module nothing calls.
#[test]
fn every_readable_command_paints_something_when_asked() {
    let s = session("painted");
    for args in s.readable() {
        let args = argv(&args);
        // `dupes` and `activations` have nothing to say about the fixtures, and a
        // command with nothing to print has nothing to colour.
        if matches!(args[0], "dupes" | "activations") {
            continue;
        }
        let out = s.output(&args, &[("CLICOLOR_FORCE", "1")]);
        assert!(
            out.contains('\x1b'),
            "`fontina {}` printed nothing in colour:\n{out}",
            args.join(" ")
        );
    }
}

/// `NO_COLOR` is a person speaking, and it beats the variable that asks for colour.
#[test]
fn no_color_beats_the_variable_that_forces_it() {
    let s = session("no-color");
    let out = s.output(&["list"], &[("CLICOLOR_FORCE", "1"), ("NO_COLOR", "1")]);
    assert!(!out.contains('\x1b'), "{out}");
}

/// `--json` is a contract with another program, and another program does not want the
/// colours a terminal wanted.
#[test]
fn json_is_never_painted() {
    let s = session("json");
    let id = s.a_face();
    for args in [
        vec!["list", "--json"],
        vec!["facets", "--json"],
        vec!["stats", "--json"],
        vec!["info", &id, "--json"],
        vec!["check", &id, "--json"],
        vec!["license", "--json"],
    ] {
        let args = &args[..];
        let out = s.output(args, &[("CLICOLOR_FORCE", "1")]);
        assert!(
            !out.contains('\x1b'),
            "`fontina {}` painted machine-readable output:\n{out}",
            args.join(" ")
        );
        let body = out.trim_start();
        assert!(
            body.starts_with('{') || body.starts_with('['),
            "`fontina {}` did not print JSON:\n{out}",
            args.join(" ")
        );
    }
}

/// Colour is the only difference. Strip the escapes from a coloured run and it is the
/// piped run, byte for byte — which is what makes the colour hierarchy rather than
/// meaning, and what makes a reader and a script see the same table.
#[test]
fn painting_changes_nothing_but_the_paint() {
    let s = session("stripped");
    for args in s.readable() {
        let args = argv(&args);
        let plain = s.output(&args, &[]);
        let painted = s.output(&args, &[("CLICOLOR_FORCE", "1")]);
        assert_eq!(
            strip(&painted),
            plain,
            "`fontina {}` says something different in colour",
            args.join(" ")
        );
    }
}

/// Every SGR sequence removed. Deliberately not a general terminal parser: the only
/// escapes this program writes are `\x1b[<params>m`, and a test that accepted more
/// would stop noticing if that changed.
fn strip(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(at) = rest.find('\x1b') {
        out.push_str(&rest[..at]);
        let tail = &rest[at..];
        let end = tail
            .find('m')
            .unwrap_or_else(|| panic!("an escape that is not an SGR sequence: {tail:?}"));
        assert!(
            tail[..end].starts_with("\x1b[")
                && tail[2..end].chars().all(|c| c.is_ascii_digit() || c == ';'),
            "an escape that is not an SGR sequence: {:?}",
            &tail[..=end]
        );
        rest = &tail[end + 1..];
    }
    out.push_str(rest);
    out
}
