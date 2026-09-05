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

//! Moving a font from one activation state to another, through the binary.
//!
//! A person tries a font for the session, keeps it for the user, decides to install it
//! properly, then changes their mind. Each of those is one command, and each command
//! used to take the new state without giving up the old one: the font stayed registered
//! where the previous command had put it, and the index row that would have named that
//! registration was overwritten by the new one. What was left could not be found, let
//! alone undone.
//!
//! The unit tests beside `leave_current_state` hold the ordering with a fake activator.
//! These hold what a person can see afterwards: no copy left in the font directory, no
//! link left in the one fontconfig reads.
//!
//! `install` is a copy or a symlink into a redirected home on GNU/Linux and macOS, and
//! `HKCU` plus `AddFontResource` on Windows. `activate` is a symlink and a fontconfig
//! snippet on GNU/Linux, and a real CoreText or GDI registration elsewhere. A test that
//! would reach the running login session says so and skips.

use std::path::PathBuf;
use std::process::{Command, Output};

fn install_is_hermetic() -> bool {
    cfg!(unix)
}

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

struct Session {
    root: PathBuf,
    db: PathBuf,
    id: String,
}

impl Drop for Session {
    fn drop(&mut self) {
        // Leave nothing registered behind, whatever the test ended on.
        let _ = self.run(&["deactivate", &self.id.clone()]);
        let _ = self.run(&["uninstall", &self.id.clone()]);
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// A sandboxed home with one fixture scanned into a fresh index.
///
/// The directory name carries the test's own name and this process's id: two tests
/// sharing one is two processes creating one index at the same instant.
fn session(name: &str) -> Session {
    let root =
        std::env::temp_dir().join(format!("fontina-transition-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let fonts = root.join("fonts");
    std::fs::create_dir_all(&fonts).unwrap();
    std::fs::create_dir_all(root.join(".config")).unwrap();
    std::fs::create_dir_all(root.join(".local/share")).unwrap();
    let fixture =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/Amiri-Regular.ttf");
    std::fs::copy(&fixture, fonts.join("Amiri-Regular.ttf")).unwrap();

    let mut s = Session {
        db: root.join("index.db"),
        root,
        id: String::new(),
    };
    let scanned = s.run(&["scan", &fonts.to_string_lossy()]);
    assert!(
        scanned.status.success(),
        "{}",
        String::from_utf8_lossy(&scanned.stderr)
    );
    let listed = s.run(&["list", "--json"]);
    let faces: serde_json::Value = serde_json::from_slice(&listed.stdout).expect("JSON");
    s.id = faces[0]["id"].to_string();
    s
}

impl Session {
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

    /// Where the copy went, if the record says one was made.
    fn installed_path(&self) -> Option<PathBuf> {
        let records: serde_json::Value =
            serde_json::from_str(&self.ok(&["activations", "--json"])).expect("JSON");
        records
            .as_array()?
            .first()?
            .get("installed_path")?
            .as_str()
            .map(PathBuf::from)
    }

    /// The links `activate` leaves for fontconfig to read. Empty means nothing is
    /// registered in place; this is the observable on GNU/Linux.
    fn active_links(&self) -> Vec<PathBuf> {
        let dir = self.root.join(".local/share/fonts/fontina-active");
        std::fs::read_dir(dir)
            .map(|d| d.flatten().map(|e| e.path()).collect())
            .unwrap_or_default()
    }
}

/// Installing a font that is already activated takes the in-place registration back.
///
/// Without it the font stays visible to every application with nothing naming it, which
/// is the state a person reaches by trying a font and then deciding to keep it.
#[test]
fn install_over_an_activation_leaves_no_registration_behind() {
    if !allowed(
        install_is_hermetic() && activation_is_hermetic(),
        "install_over_an_activation_leaves_no_registration_behind",
    ) {
        return;
    }
    let s = session("install-over-activate");
    let id = s.id.clone();

    s.ok(&["activate", "--user", &id]);
    assert_eq!(s.active_links().len(), 1, "the font is registered in place");

    s.ok(&["install", &id]);
    assert!(
        s.active_links().is_empty(),
        "installing took the in-place registration back: {:?}",
        s.active_links()
    );
    let copy = s.installed_path().expect("a copy was recorded");
    assert!(copy.exists());

    s.ok(&["uninstall", &id]);
    assert!(!copy.exists(), "the copy is gone");
    assert!(s.active_links().is_empty(), "and so is the registration");
}

/// Activating a font that is already installed takes the copy back.
#[test]
fn activate_over_an_install_leaves_no_copy_behind() {
    if !allowed(
        install_is_hermetic() && activation_is_hermetic(),
        "activate_over_an_install_leaves_no_copy_behind",
    ) {
        return;
    }
    let s = session("activate-over-install");
    let id = s.id.clone();

    s.ok(&["install", &id]);
    let copy = s.installed_path().expect("a copy was recorded");
    assert!(copy.exists());

    s.ok(&["activate", "--user", &id]);
    assert!(
        !copy.exists(),
        "activating removed the copy it replaced: {}",
        copy.display()
    );
    assert_eq!(s.active_links().len(), 1);
    assert!(
        s.installed_path().is_none(),
        "and the record no longer names a copy"
    );

    s.ok(&["deactivate", &id]);
    assert!(s.active_links().is_empty());
}

/// `deactivate` on an installed font says which command removes it.
///
/// It used to take the record away and leave the copy in the font directory: nothing was
/// deactivated, because what the operating system reads is the copy, and after the record
/// was cleared no command could find the copy either.
#[test]
fn deactivate_refuses_an_installed_font_and_names_uninstall() {
    if !allowed(
        install_is_hermetic(),
        "deactivate_refuses_an_installed_font_and_names_uninstall",
    ) {
        return;
    }
    let s = session("deactivate-installed");
    let id = s.id.clone();

    s.ok(&["install", &id]);
    let copy = s.installed_path().expect("a copy was recorded");

    let out = s.run(&["deactivate", &id]);
    assert_eq!(out.status.code(), Some(1), "it is an error");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("uninstall"), "it names the command: {err}");
    assert!(copy.exists(), "and it left the copy alone");
    assert!(
        s.installed_path().is_some(),
        "and left the record that names the copy"
    );

    // The command it named does the job.
    s.ok(&["uninstall", &id]);
    assert!(!copy.exists());
}
