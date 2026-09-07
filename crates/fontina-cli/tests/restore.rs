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

//! `fontina restore` against an index that has moved on, and the conflict gate in front
//! of `activate` and `install`.
//!
//! `restore` is what the login agent runs. Nobody is watching it, nobody will read a
//! prompt, and the index it walks was written days ago: fonts have been moved, deleted,
//! replaced by a directory of the same name, removed from the index by a prune. The
//! property that matters is that it gets to the end of the list and says what happened,
//! rather than stopping at the first record that surprises it. Nothing checked that.
//!
//! Everything here drives the binary, which is also what a login agent does — the unit
//! file's `ExecStart` is this program and these arguments. Each test gets its own
//! temporary home directory, passed to the child process rather than set in this one, so
//! the tests can run in parallel and the developer's own session is never the sandbox.
//!
//! Two backends cannot be sandboxed that far. `install` writes `HKCU` and calls
//! `AddFontResource` on Windows, and `activate` is a real CoreText or GDI registration
//! on macOS and Windows; on GNU/Linux both are a copy, a symlink and a fontconfig
//! snippet under `XDG_DATA_HOME`. Tests that would reach the running login session say
//! so and skip.

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// True where `install` is a copy or a symlink into a redirected home directory.
fn install_is_hermetic() -> bool {
    cfg!(unix)
}

/// True where `activate` is too.
fn activation_is_hermetic() -> bool {
    cfg!(all(unix, not(target_os = "macos")))
}

fn allowed(hermetic: bool, what: &str) -> bool {
    if hermetic {
        return true;
    }
    eprintln!("skipped {what}: it would reach the running login session on this system");
    false
}

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

/// One test's home directory, index and font directory.
struct Session {
    root: PathBuf,
    fonts: PathBuf,
    db: PathBuf,
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// A sandbox holding copies of `wanted` fixtures, with them scanned into a fresh index.
///
/// The directory name carries this test's own name and this process's id. Two tests
/// sharing one is two processes creating one index at the same instant, which is a race
/// and not a saving.
fn session(name: &str, wanted: &[&str]) -> Session {
    let root = std::env::temp_dir().join(format!("fontina-restore-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let fonts = root.join("fonts");
    std::fs::create_dir_all(&fonts).unwrap();
    std::fs::create_dir_all(root.join(".config")).unwrap();
    std::fs::create_dir_all(root.join(".local/share")).unwrap();
    for f in wanted {
        std::fs::copy(fixtures().join(f), fonts.join(f)).unwrap();
    }
    let s = Session {
        db: root.join("index.db"),
        root,
        fonts,
    };
    let scanned = s.run(&["scan", &s.fonts.to_string_lossy()]);
    assert!(
        scanned.status.success(),
        "{}",
        String::from_utf8_lossy(&scanned.stderr)
    );
    s
}

impl Session {
    /// Run the binary the login agent would run, with the sandbox as its home.
    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_fontina"))
            .args(["--db", &self.db.to_string_lossy()])
            .args(args)
            .env("HOME", &self.root)
            .env("XDG_CONFIG_HOME", self.root.join(".config"))
            .env("XDG_DATA_HOME", self.root.join(".local/share"))
            .env("LOCALAPPDATA", self.root.join("AppData/Local"))
            .output()
            .expect("fontina runs")
    }

    fn json(&self, args: &[&str]) -> Value {
        let o = self.run(args);
        assert!(
            o.status.success(),
            "`fontina {}` failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&o.stderr)
        );
        serde_json::from_slice(&o.stdout).unwrap_or_else(|e| {
            panic!(
                "`fontina {}` did not print JSON: {e}\n{}",
                args.join(" "),
                String::from_utf8_lossy(&o.stdout)
            )
        })
    }

    /// The id of the first face parsed out of `file`.
    fn id_of(&self, file: &str) -> String {
        let listed = self.json(&["list", "--json"]);
        for face in listed.as_array().expect("a list") {
            let path = face["path"].as_str().unwrap_or_default();
            if Path::new(path).file_name().and_then(|n| n.to_str()) == Some(file) {
                return face["id"].to_string();
            }
        }
        panic!("no face from {file} in the index");
    }

    /// Where `install` put its copy of the font behind `id`.
    fn installed_path(&self, id: &str) -> PathBuf {
        let want: Value = id.parse::<i64>().expect("a numeric face id").into();
        let records = self.json(&["activations", "--json"]);
        for r in records.as_array().expect("records") {
            if r["face"]["id"] == want {
                return PathBuf::from(
                    r["installed_path"]
                        .as_str()
                        .expect("an installed record says where the copy went"),
                );
            }
        }
        panic!("face {id} has no activation record");
    }

    fn install(&self, file: &str) -> (String, PathBuf) {
        let id = self.id_of(file);
        let o = self.run(&["install", &id]);
        assert!(
            o.status.success(),
            "installing {file}: {}",
            String::from_utf8_lossy(&o.stderr)
        );
        let path = self.installed_path(&id);
        assert!(path.exists(), "{} was not written", path.display());
        (id, path)
    }
}

// ----- an index that has moved on -----

/// Four records, three of them broken in a different way, and one `restore`.
///
/// Every failure here is something that was true when the activation was recorded and
/// is not true now, which is the only kind of record a login agent ever meets. The
/// assertion that matters is the last one: `restore` reaches the end of the list, exits
/// 0, and hands back a report naming each font it could not put back. Stopping at the
/// first would mean every font after it in the list stayed missing until somebody
/// noticed by hand.
#[test]
fn restore_finishes_the_whole_list_and_reports_what_it_could_not_do() {
    if !allowed(
        install_is_hermetic(),
        "restore_finishes_the_whole_list_and_reports_what_it_could_not_do",
    ) {
        return;
    }
    let s = session(
        "moved-on",
        &[
            "Amiri-Regular.ttf",
            "SourceSerif4-Regular.otf",
            "Nabla[EDPT,EHLT].ttf",
            "BricolageGrotesque[opsz,wdth,wght].ttf",
        ],
    );

    // Untouched: there is nothing left to do for it.
    let (_intact, intact_copy) = s.install("Amiri-Regular.ttf");
    // The copy is gone — a font directory somebody tidied, or a home restored from a
    // backup that skipped it. The source is still there, so it can be put back.
    let (_wiped, wiped_copy) = s.install("SourceSerif4-Regular.otf");
    std::fs::remove_file(&wiped_copy).unwrap();
    // The source font is gone as well: nothing can be put back, and saying so is all
    // there is to do.
    let (_vanished, vanished_copy) = s.install("Nabla[EDPT,EHLT].ttf");
    std::fs::remove_file(&vanished_copy).unwrap();
    std::fs::remove_file(s.fonts.join("Nabla[EDPT,EHLT].ttf")).unwrap();
    // The source path is now a directory, which is the one that would reach `fs::copy`
    // if `regular_file` did not catch it first.
    let (_shadowed, shadowed_copy) = s.install("BricolageGrotesque[opsz,wdth,wght].ttf");
    std::fs::remove_file(&shadowed_copy).unwrap();
    let shadowed_source = s.fonts.join("BricolageGrotesque[opsz,wdth,wght].ttf");
    std::fs::remove_file(&shadowed_source).unwrap();
    std::fs::create_dir_all(&shadowed_source).unwrap();

    let out = s.run(&["restore", "--json"]);
    assert!(
        out.status.success(),
        "restore must finish and report, not fail: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let report: Value = serde_json::from_slice(&out.stdout).expect("a JSON report");

    assert_eq!(report["restored"], 2, "the intact one and the wiped one");
    assert_eq!(
        report["reinstalled"], 1,
        "only the wiped one had work to do"
    );
    let failed = report["failed"].as_array().expect("a list of failures");
    assert_eq!(failed.len(), 2, "{failed:?}");

    let named: Vec<&str> = failed.iter().map(|f| f[0].as_str().unwrap()).collect();
    assert!(
        named.iter().any(|p| p.contains("Nabla")),
        "the font that vanished has to be named: {named:?}"
    );
    assert!(
        named.iter().any(|p| p.contains("Bricolage")),
        "so does the one whose path is now a directory: {named:?}"
    );

    assert!(intact_copy.exists(), "the untouched install is still there");
    assert!(wiped_copy.exists(), "and the wiped one was put back");
    assert_eq!(
        s.installed_path(&s.id_of("SourceSerif4-Regular.otf")),
        wiped_copy,
        "the index was told where the new copy went"
    );

    // Running it again is the ordinary case — a second login — and says the same thing.
    let again: Value = serde_json::from_slice(&s.run(&["restore", "--json"]).stdout).unwrap();
    assert_eq!(again["restored"], 2);
    assert_eq!(
        again["reinstalled"], 0,
        "nothing to reinstall the second time"
    );
    assert_eq!(again["failed"].as_array().unwrap().len(), 2);
}

/// A face that left the index takes its activation with it, so `restore` never walks a
/// record it cannot resolve.
#[test]
fn restore_never_sees_an_activation_whose_face_was_pruned() {
    if !allowed(
        install_is_hermetic(),
        "restore_never_sees_an_activation_whose_face_was_pruned",
    ) {
        return;
    }
    let s = session("pruned", &["Amiri-Regular.ttf", "SourceSerif4-Regular.otf"]);
    s.install("Amiri-Regular.ttf");
    let (_kept, kept_copy) = s.install("SourceSerif4-Regular.otf");
    assert_eq!(
        s.json(&["activations", "--json"]).as_array().unwrap().len(),
        2
    );

    std::fs::remove_file(s.fonts.join("Amiri-Regular.ttf")).unwrap();
    let pruned = s.run(&["scan", "--prune", &s.fonts.to_string_lossy()]);
    assert!(
        pruned.status.success(),
        "{}",
        String::from_utf8_lossy(&pruned.stderr)
    );

    let records = s.json(&["activations", "--json"]);
    assert_eq!(
        records.as_array().unwrap().len(),
        1,
        "the pruned face's activation went with it"
    );

    let report: Value = serde_json::from_slice(&s.run(&["restore", "--json"]).stdout).unwrap();
    assert_eq!(report["restored"], 1);
    assert_eq!(report["failed"].as_array().unwrap().len(), 0);
    assert!(kept_copy.exists());
}

/// A session-scoped activation whose font has since been deleted.
///
/// This is the record a login agent is most likely to meet, because a session
/// activation is the one that has to be re-applied at all, and the longest-lived index
/// is the one most likely to name a font that has moved. It has to come back as one
/// named failure, not as the end of the run.
#[test]
fn a_session_activation_pointing_at_a_deleted_font_is_one_named_failure() {
    if !allowed(
        activation_is_hermetic(),
        "a_session_activation_pointing_at_a_deleted_font_is_one_named_failure",
    ) {
        return;
    }
    let s = session(
        "session-gone",
        &["Amiri-Regular.ttf", "SourceSerif4-Regular.otf"],
    );
    for f in ["Amiri-Regular.ttf", "SourceSerif4-Regular.otf"] {
        let id = s.id_of(f);
        let o = s.run(&["activate", "--session", &id]);
        assert!(
            o.status.success(),
            "activating {f}: {}",
            String::from_utf8_lossy(&o.stderr)
        );
    }
    std::fs::remove_file(s.fonts.join("Amiri-Regular.ttf")).unwrap();

    let out = s.run(&["restore", "--json"]);
    assert!(out.status.success(), "restore still finishes");
    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["restored"], 1, "the one that is still there");
    let failed = report["failed"].as_array().unwrap();
    assert_eq!(failed.len(), 1);
    assert!(failed[0][0].as_str().unwrap().contains("Amiri"));
    assert!(
        !failed[0][1].as_str().unwrap().is_empty(),
        "a failure has to carry a reason a person can act on"
    );

    for f in ["Amiri-Regular.ttf", "SourceSerif4-Regular.otf"] {
        let id = s.id_of(f);
        let _ = s.run(&["deactivate", &id]);
    }
}

// ----- an index that cannot be read -----

/// The index itself is the other thing that can have moved on, and a login agent has
/// nobody to ask. Whatever is wrong with it, `restore` has to stop with exit 1 and a
/// reason on stderr, and put nothing on stdout: ADR 0008 promises a reader that stdout
/// is JSON or empty, never a message.
#[test]
fn restore_fails_cleanly_when_the_index_cannot_be_opened() {
    let s = session("unreadable", &["Amiri-Regular.ttf"]);

    for (what, db) in [
        ("a directory", s.root.join("adirectory")),
        ("a file that is not a database", s.root.join("garbage.db")),
    ] {
        if what.starts_with("a directory") {
            std::fs::create_dir_all(&db).unwrap();
        } else {
            std::fs::write(&db, b"this is not a SQLite file, it is a note\n").unwrap();
        }
        let out = Command::new(env!("CARGO_BIN_EXE_fontina"))
            .args(["--db", &db.to_string_lossy(), "restore", "--json"])
            .env("HOME", &s.root)
            .env("XDG_CONFIG_HOME", s.root.join(".config"))
            .env("XDG_DATA_HOME", s.root.join(".local/share"))
            .output()
            .expect("fontina runs");
        assert_eq!(
            out.status.code(),
            Some(1),
            "{what}: expected exit 1, got {:?}\n{}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            out.stdout.is_empty(),
            "{what}: stdout must stay clean, got {:?}",
            String::from_utf8_lossy(&out.stdout)
        );
        assert!(
            !out.stderr.is_empty(),
            "{what}: a failure has to say something"
        );
    }
}

/// Another process is holding the index open with a write transaction — a `watch`
/// running in the reader's other terminal, or a scan that has not finished — and the
/// login agent starts anyway.
///
/// How long the wait is depends on where the lock is taken, so the assertions are the
/// ones that hold whatever happens: it terminates rather than waiting for ever, and
/// every record it walked is accounted for exactly once — restored or failed, never
/// dropped, and never counted as restored when the index was not told. A report that
/// silently lost a record would be worse than an error, because the reader would go
/// looking for a font the agent believes it put back.
#[test]
fn restore_does_not_hang_on_an_index_another_process_has_locked() {
    if !allowed(
        install_is_hermetic(),
        "restore_does_not_hang_on_an_index_another_process_has_locked",
    ) {
        return;
    }
    let s = session("locked", &["Amiri-Regular.ttf"]);
    let (_id, copy) = s.install("Amiri-Regular.ttf");
    std::fs::remove_file(&copy).unwrap(); // so restore has a write to do

    let mut holder = rusqlite::Connection::open(&s.db).unwrap();
    let tx = holder
        .transaction_with_behavior(rusqlite::TransactionBehavior::Exclusive)
        .expect("the other process takes the write lock");

    let started = std::time::Instant::now();
    let out = s.run(&["restore", "--json"]);
    let took = started.elapsed();
    drop(tx);

    assert!(
        took < std::time::Duration::from_secs(60),
        "restore waited {took:?}; the index's busy timeout is 15 seconds"
    );
    match out.status.code() {
        Some(0) => {
            let report: Value =
                serde_json::from_slice(&out.stdout).expect("exit 0 means a report on stdout");
            let restored = report["restored"].as_u64().unwrap();
            let failed = report["failed"].as_array().unwrap();
            assert_eq!(
                restored as usize + failed.len(),
                1,
                "the one record has to be accounted for exactly once: {report}"
            );
            for f in failed {
                assert!(
                    !f[1].as_str().unwrap_or_default().is_empty(),
                    "a failure carries a reason: {report}"
                );
            }
        }
        Some(1) => {
            assert!(
                out.stdout.is_empty(),
                "a failure puts nothing on stdout: {:?}",
                String::from_utf8_lossy(&out.stdout)
            );
            assert!(!out.stderr.is_empty(), "and says why");
        }
        other => panic!(
            "restore ended as {other:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        ),
    }
}

// ----- the gate in front of activation -----

/// A conflict stops `install` with exit 2 and changes nothing, and `--replace` undoes
/// the conflicting activation before taking its place.
///
/// Exit 2 is the promise a script reads to tell "this font clashes with one already
/// active" from "something went wrong". What `--replace` is able to do about a conflict
/// depends on what the conflicting face is: an `installed` one is uninstalled, and its
/// copy has to actually go, or the font it was meant to make way for is competing with
/// a file nothing points at any more.
#[test]
fn a_conflict_stops_install_at_exit_2_and_replace_takes_the_other_ones_place() {
    if !allowed(
        install_is_hermetic(),
        "a_conflict_stops_install_at_exit_2_and_replace_takes_the_other_ones_place",
    ) {
        return;
    }
    // The same font in two containers: one PostScript name, two files.
    let s = session(
        "conflict",
        &[
            "inter-latin-400-normal.woff",
            "inter-latin-400-normal.woff2",
        ],
    );
    let (_first_id, first_copy) = s.install("inter-latin-400-normal.woff2");
    let second_id = s.id_of("inter-latin-400-normal.woff");

    let refused = s.run(&["install", &second_id]);
    assert_eq!(
        refused.status.code(),
        Some(2),
        "a conflict is exit 2, not exit 1: {}",
        String::from_utf8_lossy(&refused.stderr)
    );
    assert!(
        String::from_utf8_lossy(&refused.stderr).contains("conflict"),
        "and it says what is in the way"
    );
    assert_eq!(
        s.json(&["activations", "--json"]).as_array().unwrap().len(),
        1,
        "the refused install changed nothing"
    );
    assert!(first_copy.exists());

    let replaced = s.run(&["install", "--replace", &second_id]);
    assert!(
        replaced.status.success(),
        "{}",
        String::from_utf8_lossy(&replaced.stderr)
    );
    let records = s.json(&["activations", "--json"]);
    let records = records.as_array().unwrap();
    assert_eq!(records.len(), 1, "one of them, not both: {records:?}");
    assert_eq!(records[0]["face"]["id"].to_string(), second_id);
    assert!(
        !first_copy.exists(),
        "the face that was replaced had its copy removed too"
    );
    assert_ne!(s.installed_path(&second_id), first_copy);
    let _ = s.run(&["uninstall", &second_id]);
}
