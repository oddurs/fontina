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

//! The sandbox every fontina test was writing for itself.
//!
//! Twenty-four integration test files build a temporary directory, redirect the
//! environment at it, run the binary against an index inside it and delete it afterwards.
//! `fn fixtures()` is written nineteen times, `CARGO_BIN_EXE_fontina` is wired up
//! eighteen times, and fourteen files carry their own `impl Drop`.
//!
//! That would be worth removing for its own sake. What makes it worth removing now is
//! that the copies have drifted, and the drift is about correctness rather than style:
//! ten files redirect `HOME`, and five of those also redirect `LOCALAPPDATA`.
//! `windows::user_fonts_dir` reads `LOCALAPPDATA` and nothing else, so the other six do
//! not redirect the per-user font directory on Windows at all.
//!
//! To be exact about the risk, because overstating it would be its own kind of wrong:
//! none of those six installs a font on Windows today. The one install test among them
//! is `#[cfg(unix)]`, and it already asserts `fonts.starts_with(&s.root)` before it
//! touches anything — somebody thought about this. So nothing is broken right now.
//!
//! What is broken is the shape. A new test copies whichever neighbour it found, inherits
//! whichever variables that neighbour happened to know about, and the assertion that
//! saved `interruption.rs` is one nobody is obliged to write. Nobody decided the six; it
//! is what happens when a helper is copied — the copy is right on the day it is made and
//! stops being right when the original learns something.
//!
//! So: one sandbox, redirecting every variable any platform reads, in one place where a
//! fix reaches every test at once.
//!
//! # Using it
//!
//! ```ignore
//! let s = fontina_testkit::cli!("my-test");   // sandbox + the built binary
//! s.ok(&["scan", &s.fixtures().to_string_lossy()]);
//! let faces: serde_json::Value = s.json(&["list", "--json"]);
//! ```
//!
//! `Sandbox::new` on its own is for tests that need the directories and the environment
//! but not the binary — library tests, and anything in `fontina-platform`, where
//! `CARGO_BIN_EXE_fontina` does not exist.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

/// Build a [`Cli`] around a fresh sandbox and the `fontina` binary this test was built
/// against.
///
/// A macro rather than a function because `CARGO_BIN_EXE_fontina` is only set for test
/// targets of the crate that defines the binary, so it has to be expanded at the call
/// site rather than inside this one.
#[macro_export]
macro_rules! cli {
    ($name:expr) => {
        $crate::Cli::new($crate::Sandbox::new($name), env!("CARGO_BIN_EXE_fontina"))
    };
}

/// Every environment variable that steers where fontina reads or writes, on any platform.
///
/// All of them are set on every platform on purpose. Setting only the ones this build
/// happens to read is how six of the copies came to be hermetic on GNU/Linux and macOS
/// and not on Windows: nobody was wrong, they just could not see the platform they were
/// not on.
const REDIRECTED: &[(&str, &str)] = &[
    ("HOME", ""),
    ("XDG_CONFIG_HOME", ".config"),
    ("XDG_DATA_HOME", ".local/share"),
    ("XDG_CACHE_HOME", ".cache"),
    ("LOCALAPPDATA", "AppData/Local"),
    ("APPDATA", "AppData/Roaming"),
];

/// Variables cleared from every sandboxed command.
///
/// The sandbox is only hermetic if the shell it was started from cannot reach into it.
/// `colour.rs` had already worked this out for `NO_COLOR` and `CLICOLOR_FORCE` — a
/// developer who sets either would have watched the colour tests disagree with CI. The
/// same argument covers the rest: `FONTINA_DB` is the variable CLAUDE.md tells people to
/// export while developing, `FONTINA_CONFIG` points `config::path` at a file that can set
/// a default scan source or preview text, and `COLUMNS`/`LINES` decide what the table
/// code draws.
///
/// `FONTINA_CONFIG` is hygiene rather than a fix: pointing it at a config that sets
/// `scan.system` or `preview.text` breaks no test today, because the tests that would
/// care pass those explicitly. It is here because the next test to rely on a default
/// should inherit a clean one, not because anything is currently wrong.
///
/// A test that wants one of these sets it back with `with_env`, which runs afterwards.
const REMOVED: &[&str] = &[
    "NO_COLOR",
    "CLICOLOR",
    "CLICOLOR_FORCE",
    "FORCE_COLOR",
    "FONTINA_DB",
    "FONTINA_CONFIG",
    "COLUMNS",
    "LINES",
];

/// A temporary directory with the environment pointed at it, removed when it is dropped.
pub struct Sandbox {
    root: PathBuf,
    db: PathBuf,
}

impl Sandbox {
    /// A sandbox named for the test using it.
    ///
    /// The name is in the path so a directory left behind by a crash says which test left
    /// it. The counter is there because the process id is not enough on its own: under
    /// `cargo test` every test in a binary shares one, and two tests calling this with
    /// the same name would share a directory and each other's index.
    pub fn new(name: &str) -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let unique = NEXT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "fontina-test-{name}-{}-{unique}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root)
            .unwrap_or_else(|e| panic!("{} must be creatable: {e}", root.display()));
        for (_, suffix) in REDIRECTED {
            if !suffix.is_empty() {
                let _ = std::fs::create_dir_all(root.join(suffix));
            }
        }
        let db = root.join("index.db");
        Sandbox { root, db }
    }

    /// The repository's fixture fonts.
    pub fn fixtures(&self) -> PathBuf {
        fixtures()
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn db(&self) -> &Path {
        &self.db
    }

    /// A path inside the sandbox, created if it is a directory this test wants.
    pub fn dir(&self, relative: &str) -> PathBuf {
        let p = self.root.join(relative);
        let _ = std::fs::create_dir_all(&p);
        p
    }

    /// Point a command at this sandbox: every redirected variable, and nothing inherited
    /// that would let it out.
    pub fn env(&self, cmd: &mut Command) -> &Self {
        for (var, suffix) in REDIRECTED {
            let value = if suffix.is_empty() {
                self.root.clone()
            } else {
                self.root.join(suffix)
            };
            cmd.env(var, value);
        }
        for var in REMOVED {
            cmd.env_remove(var);
        }
        self
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        // A test that made a directory read-only to prove fontina says so cannot delete
        // it again without this, and a sandbox that outlives its test is a sandbox the
        // next run inherits.
        #[cfg(unix)]
        restore_write_permission(&self.root);
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[cfg(unix)]
fn restore_write_permission(dir: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    if let Ok(meta) = std::fs::metadata(dir) {
        let mut perms = meta.permissions();
        perms.set_mode(perms.mode() | 0o700);
        let _ = std::fs::set_permissions(dir, perms);
    }
    for entry in entries.flatten() {
        if entry.file_type().is_ok_and(|t| t.is_dir()) {
            restore_write_permission(&entry.path());
        }
    }
}

/// The repository's fixture fonts.
///
/// Relative to this crate rather than to the caller's, so it is the same path from every
/// crate that uses it — which is the bug the nineteen copies could each have had and
/// happened not to, being all the same distance from the root.
pub fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

/// A sandbox and the `fontina` binary, for tests that drive the program rather than the
/// library. Build it with [`cli!`].
pub struct Cli {
    sandbox: Sandbox,
    binary: PathBuf,
    extra: Vec<(String, String)>,
}

impl Cli {
    #[doc(hidden)]
    pub fn new(sandbox: Sandbox, binary: &str) -> Self {
        Cli {
            sandbox,
            binary: PathBuf::from(binary),
            extra: Vec::new(),
        }
    }

    /// One more environment variable, for a test that needs the program to believe
    /// something about its surroundings — a terminal width, a colour setting, a locale.
    ///
    /// Set after the sandbox's own redirections, so a test can override one deliberately
    /// and the ordering is stated rather than discovered.
    #[must_use]
    pub fn with_env(mut self, key: &str, value: &str) -> Self {
        self.extra.push((key.to_owned(), value.to_owned()));
        self
    }

    pub fn sandbox(&self) -> &Sandbox {
        &self.sandbox
    }

    pub fn root(&self) -> &Path {
        self.sandbox.root()
    }

    pub fn db(&self) -> &Path {
        self.sandbox.db()
    }

    pub fn fixtures(&self) -> PathBuf {
        fixtures()
    }

    pub fn dir(&self, relative: &str) -> PathBuf {
        self.sandbox.dir(relative)
    }

    /// The command, for a test that needs to spawn it or change it further.
    pub fn cmd(&self, args: &[&str]) -> Command {
        let mut c = Command::new(&self.binary);
        c.args(["--db", &self.sandbox.db.to_string_lossy()])
            .args(args);
        self.sandbox.env(&mut c);
        c
    }

    /// Run it and hand back whatever happened, success or not.
    pub fn run(&self, args: &[&str]) -> Output {
        self.cmd(args).output().expect("fontina runs")
    }

    /// The same, with environment for this call only — for a test that runs one command
    /// several ways rather than one way several times.
    pub fn run_env(&self, args: &[&str], env: &[(&str, &str)]) -> Output {
        let mut c = self.cmd(args);
        for (k, v) in env {
            c.env(k, v);
        }
        c.output().expect("fontina runs")
    }

    /// Run it, require success, and return stdout.
    #[track_caller]
    pub fn ok(&self, args: &[&str]) -> String {
        let o = self.run(args);
        assert!(
            o.status.success(),
            "`fontina {}` failed ({}): {}",
            args.join(" "),
            o.status,
            String::from_utf8_lossy(&o.stderr)
        );
        String::from_utf8_lossy(&o.stdout).into_owned()
    }

    /// Run it, require failure, and return stderr — so a test asserting on a message
    /// cannot pass because the command unexpectedly succeeded and said nothing.
    #[track_caller]
    pub fn fails(&self, args: &[&str]) -> String {
        let o = self.run(args);
        assert!(
            !o.status.success(),
            "`fontina {}` was expected to fail and did not: {}",
            args.join(" "),
            String::from_utf8_lossy(&o.stdout)
        );
        String::from_utf8_lossy(&o.stderr).into_owned()
    }

    /// Run it, require success, and parse stdout as JSON.
    #[track_caller]
    pub fn json<T: serde::de::DeserializeOwned>(&self, args: &[&str]) -> T {
        let out = self.ok(args);
        serde_json::from_str(&out).unwrap_or_else(|e| {
            panic!(
                "`fontina {}` did not print JSON: {e}\n{out}",
                args.join(" ")
            )
        })
    }

    /// The index id of the first face parsed out of `file`, as a string ready to pass
    /// back on a command line.
    ///
    /// Written independently in `preview.rs` and `restore.rs` before it lived here, which
    /// is the shape of a helper that belongs to the harness rather than to a test.
    #[track_caller]
    pub fn id_of(&self, file: &str) -> String {
        let listed: serde_json::Value = self.json(&["list", "--json"]);
        for face in listed.as_array().expect("a list") {
            let path = face["path"].as_str().unwrap_or_default();
            if Path::new(path).file_name().and_then(|n| n.to_str()) == Some(file) {
                return face["id"].to_string();
            }
        }
        panic!("no face in the index came from {file}");
    }

    /// Index the repository's fixtures, which is the first line of most tests here.
    #[track_caller]
    pub fn scan_fixtures(&self) -> String {
        let path = fixtures().to_string_lossy().into_owned();
        self.ok(&["scan", &path])
    }
}
