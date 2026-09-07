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

//! Every command, given something wrong, is an error and not a panic.
//!
//! "Errors are values" is one of the rules in CLAUDE.md, and it is tested in the places
//! anyone thought to test it. This is the sweep: every command `fontina --help` reports,
//! each given a face id that does not exist, a path that does not exist, a file that is
//! not a font, an argument of the wrong shape, and an index that cannot be opened.
//!
//! What is asserted is deliberately weak and therefore universal. A command may succeed,
//! may refuse, may print nothing; what it may not do is die on a `panic!`, an `unwrap`
//! or an index out of bounds, and it may not fail silently. A backtrace is not an error
//! message: it tells the reader that fontina broke rather than what they did, and on a
//! font manager it usually means an `unwrap` on a font somebody was handed.
//!
//! Enumerated from `--help` rather than from a list, so a new command is swept the day
//! it appears rather than the day someone remembers this file.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::{Command, Output};

/// Commands this sweep may not run.
///
/// `ui` and `watch` do not return on their own — one is a full-screen program, the other
/// waits for the filesystem for as long as it is asked to. `restore` and the two `agent`
/// halves are the commands that touch the running login session, and a hostile argument
/// would not stop them: `restore` takes none. The rest of the activation commands are
/// swept, because a face id that does not exist is refused long before the operating
/// system is asked for anything.
const EXEMPT: &[(&str, &str)] = &[
    ("ui", "a full-screen program that does not return"),
    ("watch", "waits on the filesystem and does not return"),
    (
        "restore",
        "re-registers every recorded activation with the OS",
    ),
    ("agent install", "writes a login agent into the user's home"),
    (
        "agent uninstall",
        "removes a login agent from the user's home",
    ),
];

/// One set of arguments to hand every command, and what makes it hostile.
struct Hostile {
    what: &'static str,
    args: Vec<String>,
}

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

struct Session {
    root: PathBuf,
    db: PathBuf,
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn session(name: &str) -> Session {
    let root = std::env::temp_dir().join(format!("fontina-hostile-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let s = Session {
        db: root.join("index.db"),
        root,
    };
    // One real face, so a command that resolves an id has something to find and the
    // sweep is testing the command rather than an empty index.
    let out = s.run_with_db(
        &s.db.clone(),
        &["scan".into(), fixtures().to_string_lossy().into_owned()],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    s
}

impl Session {
    fn run_with_db(&self, db: &std::path::Path, args: &[String]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_fontina"))
            .args(["--db", &db.to_string_lossy()])
            .args(args)
            .env("HOME", &self.root)
            .env("XDG_CONFIG_HOME", self.root.join(".config"))
            .env("XDG_DATA_HOME", self.root.join(".local/share"))
            .env("LOCALAPPDATA", self.root.join("AppData/Local"))
            .env("COLUMNS", "80")
            // A backtrace would be printed to stderr and this looks for one; asking for
            // the full form makes a panic unmistakable rather than a one-line message.
            .env("RUST_BACKTRACE", "1")
            .output()
            .expect("fontina runs")
    }
}

/// Every command path whose own help says it takes arguments or options.
fn all_commands() -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    walk(&mut Vec::new(), &mut found);
    found
}

fn walk(path: &mut Vec<String>, found: &mut BTreeSet<String>) {
    let help = help_for(path);
    let children: Vec<String> = section(&help, "Commands:")
        .iter()
        .filter_map(|l| l.strip_prefix("  "))
        .filter(|l| !l.starts_with(' '))
        .filter_map(|l| l.split_whitespace().next())
        .filter(|n| *n != "help")
        .map(str::to_owned)
        .collect();
    // A command with subcommands is a group; the leaves are what run.
    if children.is_empty() && !path.is_empty() {
        found.insert(path.join(" "));
    }
    for name in children {
        path.push(name);
        walk(path, found);
        path.pop();
    }
}

fn help_for(path: &[String]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_fontina"))
        .args(path)
        .arg("--help")
        .output()
        .expect("fontina runs");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn section<'a>(help: &'a str, heading: &str) -> Vec<&'a str> {
    help.lines()
        .skip_while(|l| l.trim_end() != heading)
        .skip(1)
        .take_while(|l| l.trim().is_empty() || l.starts_with(' '))
        .collect()
}

/// The one thing no command may do, whatever it was given.
#[track_caller]
fn no_panic(label: &str, out: &Output) {
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        !err.contains("panicked at") && !err.contains("stack backtrace"),
        "{label} panicked instead of failing:\n{err}"
    );
    let code = out.status.code().unwrap_or_else(|| {
        panic!("{label} was killed by a signal rather than exiting:\n{err}");
    });
    assert!(
        (0..=2).contains(&code),
        "{label} exited {code}; the documented codes are 0, 1 and 2:\n{err}"
    );
    if code != 0 {
        // On stderr for a failure, on stdout for a verdict: `check` exits 1 when a font
        // fails its health checks and prints the findings as its ordinary output, which
        // is a report rather than an error. Either way something has to be said.
        let out_text = String::from_utf8_lossy(&out.stdout);
        assert!(
            !err.trim().is_empty() || !out_text.trim().is_empty(),
            "{label} exited {code} and said nothing at all"
        );
    }
}

/// Every command, given every kind of wrong argument, errors rather than panicking.
#[test]
fn no_command_panics_on_a_hostile_argument() {
    let s = session("sweep");

    // A file that is not a font, and a file that is a truncated font: both are things a
    // person points fontina at by accident, and both reach the parser.
    let not_a_font = s.root.join("notes.txt");
    std::fs::write(&not_a_font, b"this is not a font\n").unwrap();
    let truncated = s.root.join("truncated.ttf");
    let real = std::fs::read(fixtures().join("Amiri-Regular.ttf")).unwrap();
    std::fs::write(&truncated, &real[..real.len() / 3]).unwrap();
    let empty = s.root.join("empty.ttf");
    std::fs::write(&empty, b"").unwrap();

    let hostiles = vec![
        Hostile {
            what: "no arguments at all",
            args: vec![],
        },
        Hostile {
            what: "a face id that does not exist",
            args: vec!["999999".into()],
        },
        Hostile {
            what: "a path that does not exist",
            args: vec!["/nonexistent/directory/font.ttf".into()],
        },
        Hostile {
            what: "a file that is not a font",
            args: vec![not_a_font.to_string_lossy().into_owned()],
        },
        Hostile {
            what: "a font truncated to a third of its length",
            args: vec![truncated.to_string_lossy().into_owned()],
        },
        Hostile {
            what: "an empty file",
            args: vec![empty.to_string_lossy().into_owned()],
        },
        Hostile {
            what: "an argument that is not valid UTF-8 in shape",
            args: vec!["--".into(), "-".into()],
        },
        Hostile {
            what: "a very long argument",
            args: vec!["x".repeat(4096)],
        },
    ];

    let exempt: BTreeSet<&str> = EXEMPT.iter().map(|(c, _)| *c).collect();
    let commands: Vec<String> = all_commands()
        .into_iter()
        .filter(|c| !exempt.contains(c.as_str()))
        .collect();
    assert!(
        commands.len() > 20,
        "walking `fontina --help` found only {} commands; the help format has probably \
         changed and this sweep is no longer reading it",
        commands.len()
    );

    for command in &commands {
        for hostile in &hostiles {
            let mut args: Vec<String> = command.split(' ').map(str::to_owned).collect();
            args.extend(hostile.args.iter().cloned());
            let label = format!("`fontina {}` ({})", args.join(" "), hostile.what);
            no_panic(&label, &s.run_with_db(&s.db, &args));
        }
    }

    // And the exempt list is not a place for commands that no longer exist.
    let all = all_commands();
    let stale: Vec<&&str> = EXEMPT
        .iter()
        .map(|(c, _)| c)
        .filter(|c| !all.contains(**c))
        .collect();
    assert!(
        stale.is_empty(),
        "EXEMPT names commands that are gone: {stale:?}"
    );
}

/// Every command, against an index it cannot use, errors rather than panicking.
///
/// Three ways an index goes wrong that a person meets: a directory where the file should
/// be (a mistyped `--db`), a file that is not a database (a mistyped `--db` that hits
/// something real), and a database from the future, whose `user_version` is past every
/// migration this build knows.
#[test]
fn no_command_panics_on_an_index_it_cannot_use() {
    let s = session("bad-index");

    let a_directory = s.root.join("dir.db");
    std::fs::create_dir_all(&a_directory).unwrap();

    let not_a_database = s.root.join("prose.db");
    std::fs::write(&not_a_database, b"SQLite format 3 is not what this is\n").unwrap();

    let from_the_future = s.root.join("future.db");
    std::fs::copy(&s.db, &from_the_future).unwrap();
    {
        let conn = rusqlite::Connection::open(&from_the_future).unwrap();
        conn.pragma_update(None, "user_version", 9999i64).unwrap();
    }

    // Promptly, too. Every command opens the index first, and the retry loop that gets a
    // fresh index into WAL mode used to spin its whole budget on a file that was never
    // going to become a database: eight seconds before `--db` with a typo in it said so.
    let started = std::time::Instant::now();
    s.run_with_db(&not_a_database, &["list".into()]);
    assert!(
        started.elapsed() < std::time::Duration::from_secs(3),
        "opening a file that is not a database took {:?} to fail",
        started.elapsed()
    );

    let exempt: BTreeSet<&str> = EXEMPT.iter().map(|(c, _)| *c).collect();
    for command in all_commands()
        .into_iter()
        .filter(|c| !exempt.contains(c.as_str()))
    {
        for (what, db) in [
            ("a directory", &a_directory),
            ("a file that is not a database", &not_a_database),
            ("an index from a later version", &from_the_future),
        ] {
            let args: Vec<String> = command.split(' ').map(str::to_owned).collect();
            let label = format!("`fontina {command}` with {what} as the index");
            no_panic(&label, &s.run_with_db(db, &args));
        }
    }
}
