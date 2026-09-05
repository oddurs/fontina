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

//! What is left behind when a command does not finish, and what happens when the
//! filesystem says no.
//!
//! Both are ordinary. A scan of a font directory takes long enough that people close the
//! terminal, log out, or press Ctrl-C part way through, and a laptop suspends. A home
//! directory can be read-only: a locked-down desktop, a full disk, a network home that
//! has gone away. Neither is a rare condition to be reasoned about; both are Tuesday.
//!
//! The property in each case is the same. The index a person has spent time building
//! must still open, must still answer, and the command they run next must be able to
//! finish the job — and where the filesystem refuses, fontina says which path and why,
//! and exits 1 rather than panicking or reporting a success it did not achieve.

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

struct Session {
    root: PathBuf,
    fonts: PathBuf,
    db: PathBuf,
}

impl Drop for Session {
    fn drop(&mut self) {
        // A read-only directory has to be made writable again or the removal fails and
        // the temporary directory outlives the run.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            for dir in [self.root.join("ro"), self.root.join("ro/fonts")] {
                if dir.exists() {
                    let _ = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755));
                }
            }
        }
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// A sandbox with `copies` copies of each fixture, so a scan takes long enough to
/// interrupt.
fn session(name: &str, copies: usize) -> Session {
    let root =
        std::env::temp_dir().join(format!("fontina-interrupt-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let fonts = root.join("fonts");
    std::fs::create_dir_all(&fonts).unwrap();
    let sources: Vec<PathBuf> = std::fs::read_dir(fixtures())
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            matches!(
                p.extension().and_then(|e| e.to_str()),
                Some("ttf" | "otf" | "woff" | "woff2")
            )
        })
        .collect();
    assert!(
        !sources.is_empty(),
        "the fixtures directory has fonts in it"
    );
    for i in 0..copies {
        for src in &sources {
            let name = src.file_name().unwrap().to_string_lossy();
            std::fs::copy(src, fonts.join(format!("{i}-{name}"))).unwrap();
        }
    }
    Session {
        db: root.join("index.db"),
        fonts,
        root,
    }
}

impl Session {
    fn command(&self, args: &[&str]) -> Command {
        let mut c = Command::new(env!("CARGO_BIN_EXE_fontina"));
        c.args(["--db", &self.db.to_string_lossy()])
            .args(args)
            .env("HOME", &self.root)
            .env("XDG_CONFIG_HOME", self.root.join(".config"))
            .env("XDG_DATA_HOME", self.root.join(".local/share"));
        c
    }

    fn run(&self, args: &[&str]) -> std::process::Output {
        self.command(args).output().expect("fontina runs")
    }

    #[track_caller]
    fn ok(&self, args: &[&str]) -> String {
        let o = self.run(args);
        assert!(
            o.status.success(),
            "`fontina {}` failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&o.stderr)
        );
        String::from_utf8_lossy(&o.stdout).into_owned()
    }

    fn face_count(&self) -> usize {
        let listed: serde_json::Value =
            serde_json::from_str(&self.ok(&["list", "--json"])).unwrap();
        listed.as_array().expect("a list").len()
    }
}

/// A scan that is killed part way through leaves an index that opens and answers.
///
/// The scan writes in transactions and SQLite is in WAL mode, so an interrupted run
/// should leave the database at the last committed batch and nothing in between. That is
/// the claim; this is the test of it. What a person does next is run the command again,
/// so that is what happens here, and the second run has to reach the same total as an
/// uninterrupted one.
#[test]
fn a_killed_scan_leaves_an_index_that_still_works() {
    let s = session("killed-scan", 60);
    let expected = {
        // What an undisturbed scan finds, from a second sandbox with the same corpus.
        let clean = session("killed-scan-reference", 60);
        clean.ok(&["scan", &clean.fonts.to_string_lossy()]);
        clean.face_count()
    };
    assert!(expected > 100, "the corpus is big enough to interrupt");

    let mut child = s
        .command(&["scan", &s.fonts.to_string_lossy()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("fontina scan starts");

    // Kill it once it has written something, and before it has finished. Waiting for the
    // file is what makes this a test of an interrupted write rather than of an empty
    // directory; if the scan finishes first the corpus was too small.
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut killed_while_running = false;
    while Instant::now() < deadline {
        if s.db.metadata().map(|m| m.len()).unwrap_or(0) > 0 {
            killed_while_running = child.try_wait().expect("waiting on the child").is_none();
            let _ = child.kill();
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let _ = child.wait();
    assert!(
        killed_while_running,
        "the scan was already finished, so nothing was interrupted"
    );

    // The index opens, answers, and reports its own state without complaint.
    let stats: serde_json::Value = serde_json::from_str(&s.ok(&["stats", "--json"])).unwrap();
    assert!(stats["faces"].is_number(), "{stats}");
    let after_kill = s.face_count();

    // And the job can be finished by running the command again.
    s.ok(&["scan", &s.fonts.to_string_lossy()]);
    assert_eq!(
        s.face_count(),
        expected,
        "the second scan did not reach what an undisturbed scan finds (it had {after_kill} \
         after the kill)"
    );
}

/// Killing a scan does not leave a lock that the next command cannot get past.
///
/// A process that dies holding a SQLite write lock leaves the `-wal` and `-shm` files
/// behind. The next connection has to recover them rather than wait on a lock nobody
/// holds; a `busy_timeout` would otherwise turn this into a fifteen-second pause before
/// every command, for as long as the file exists.
#[test]
fn a_killed_scan_leaves_no_lock_behind() {
    let s = session("killed-lock", 40);
    let mut child = s
        .command(&["scan", &s.fonts.to_string_lossy()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("fontina scan starts");
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        if s.db.metadata().map(|m| m.len()).unwrap_or(0) > 0 {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let _ = child.kill();
    let _ = child.wait();

    let started = Instant::now();
    s.ok(&["list", "--json"]);
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "the next command waited {:?} on a lock nobody was holding",
        started.elapsed()
    );
}

/// A read-only font directory is an error that names the path, not a panic.
///
/// `install` copies a font into the per-user font directory. A locked-down desktop, a
/// full disk and a network home that has gone away all present as the same thing: the
/// write fails. What a person needs then is the path and the reason, and an exit code
/// that says the install did not happen — not a backtrace, and not a cheerful summary of
/// an install that is not there.
#[cfg(unix)]
#[test]
fn a_read_only_font_directory_is_an_error_that_says_which_path() {
    use std::os::unix::fs::PermissionsExt;

    let s = session("read-only", 1);
    s.ok(&["scan", &s.fonts.to_string_lossy()]);
    let id = {
        let listed: serde_json::Value = serde_json::from_str(&s.ok(&["list", "--json"])).unwrap();
        listed[0]["id"].to_string()
    };

    // The per-user font directory fontina itself names, made unwritable. Asking rather
    // than assuming: it is `$XDG_DATA_HOME/fonts` on GNU/Linux and `~/Library/Fonts` on
    // macOS, and the test is about the write failing, not about where it would go.
    let dirs: serde_json::Value = serde_json::from_str(&s.ok(&["dirs", "--json"])).unwrap();
    let fonts = dirs
        .as_array()
        .expect("directories")
        .iter()
        .find(|d| d["user_writable"].as_bool().unwrap_or(false))
        .map(|d| PathBuf::from(d["path"].as_str().expect("a path")))
        .expect("this system has a per-user font directory");
    assert!(
        fonts.starts_with(&s.root),
        "the sandbox did not redirect the font directory ({}), so this test would write \
         to the real one",
        fonts.display()
    );
    std::fs::create_dir_all(&fonts).unwrap();
    std::fs::set_permissions(&fonts, std::fs::Permissions::from_mode(0o555)).unwrap();

    let out = s.run(&["install", &id]);
    std::fs::set_permissions(&fonts, std::fs::Permissions::from_mode(0o755)).unwrap();

    assert_eq!(out.status.code(), Some(1), "a refused write is an error");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        !err.contains("panicked") && !err.contains("RUST_BACKTRACE"),
        "it is an error value, not a panic: {err}"
    );
    assert!(
        err.contains("fonts"),
        "the message names the directory it could not write to: {err}"
    );

    // And nothing was recorded, so no later command believes a copy exists.
    let records: serde_json::Value =
        serde_json::from_str(&s.ok(&["activations", "--json"])).unwrap();
    assert!(
        records.as_array().expect("records").is_empty(),
        "an install that did not happen was recorded anyway: {records}"
    );
}

/// An index in a directory that cannot be written is an error naming the path.
#[cfg(unix)]
#[test]
fn an_index_that_cannot_be_created_is_an_error_that_says_which_path() {
    use std::os::unix::fs::PermissionsExt;

    let s = session("read-only-db", 1);
    let ro = s.root.join("ro");
    std::fs::create_dir_all(&ro).unwrap();
    std::fs::set_permissions(&ro, std::fs::Permissions::from_mode(0o555)).unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_fontina"))
        .args(["--db", &ro.join("index.db").to_string_lossy()])
        .args(["scan", &s.fonts.to_string_lossy()])
        .env("HOME", &s.root)
        .output()
        .expect("fontina runs");
    std::fs::set_permissions(&ro, std::fs::Permissions::from_mode(0o755)).unwrap();

    assert_eq!(out.status.code(), Some(1));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        !err.contains("panicked"),
        "it is an error value, not a panic: {err}"
    );
    assert!(
        err.contains("index.db") || err.contains(&ro.display().to_string()),
        "the message names the file it could not create: {err}"
    );
}
