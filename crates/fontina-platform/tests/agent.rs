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

//! The login agent, put through what a person does to it rather than what a script
//! does: installed twice, uninstalled twice, installed over a file fontina never wrote,
//! over a directory, over a symlink, and uninstalled after somebody edited the unit by
//! hand.
//!
//! Nothing here touches the real login session. Every test redirects the home directory
//! and the XDG directories at a temporary tree, the way `linux.rs` does, and
//! [`agent::plan`] reads them at call time, so the file the agent would install lands in
//! the sandbox. Windows is the exception — `directories` resolves the Startup folder
//! through `SHGetKnownFolderPath`, which no environment variable can redirect — so the
//! tests that write anything skip there and say so.
//!
//! Several of these assert behaviour that is wrong. They are written as
//! characterisation tests, named for the defect and commented with what the right
//! answer would be, so that the fix has a failing assertion waiting for it.

use fontina_platform::agent;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

/// The environment is process-global, and every test in this file rewrites it.
static ENV: Mutex<()> = Mutex::new(());

/// The variables that decide where the agent goes on the systems this can sandbox.
const REDIRECTED: [&str; 3] = ["HOME", "XDG_CONFIG_HOME", "XDG_DATA_HOME"];

/// A temporary home directory, held for as long as the test runs.
///
/// Restores the environment and removes the tree on drop, so a failing assertion does
/// not leak either into the next test.
struct Home {
    root: PathBuf,
    saved: Vec<(&'static str, Option<OsString>)>,
    _guard: MutexGuard<'static, ()>,
}

impl Drop for Home {
    fn drop(&mut self) {
        for (key, value) in &self.saved {
            // SAFETY: the tests in this file are serialised through `ENV`, and this runs
            // while that guard is still held.
            unsafe {
                match value {
                    Some(v) => std::env::set_var(key, v),
                    None => std::env::remove_var(key),
                }
            }
        }
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// A private home directory for one test, or `None` where the agent's location cannot
/// be redirected and a real login session would be written to.
///
/// The name carries the test's own name and this process's id: two tests sharing a
/// sandbox is a race, not a saving.
fn home(name: &str) -> Option<Home> {
    let guard = ENV.lock().unwrap_or_else(|e| e.into_inner());
    if cfg!(windows) {
        eprintln!("skipped {name}: the Windows Startup folder cannot be redirected");
        return None;
    }
    let root = std::env::temp_dir().join(format!("fontina-agent-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".config")).unwrap();
    std::fs::create_dir_all(root.join(".local/share")).unwrap();
    let saved = REDIRECTED
        .iter()
        .map(|k| (*k, std::env::var_os(k)))
        .collect();
    // SAFETY: serialised through `ENV`, and restored in `Home::drop`.
    unsafe {
        std::env::set_var("HOME", &root);
        std::env::set_var("XDG_CONFIG_HOME", root.join(".config"));
        std::env::set_var("XDG_DATA_HOME", root.join(".local/share"));
    }
    Some(Home {
        root,
        saved,
        _guard: guard,
    })
}

fn exe() -> PathBuf {
    PathBuf::from("/opt/fontina/bin/fontina")
}

fn args() -> Vec<String> {
    vec!["restore".to_string()]
}

/// Where the agent would go on this system, with the sandbox in force.
fn agent_path() -> PathBuf {
    agent::plan(&exe(), &args())
        .expect("a home directory exists")
        .path
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn symlink(target: &Path, link: &Path) {
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, link).unwrap();
    #[cfg(not(unix))]
    {
        let _ = (target, link);
        unreachable!("the tests that make symlinks do not run on this platform");
    }
}

// ----- the ordinary lifecycle -----

/// Install, install again, uninstall, uninstall again. Every step has to be safe to
/// repeat, because somebody who is not sure whether it worked will simply run it again.
#[test]
fn installing_and_uninstalling_twice_says_the_same_thing_each_time() {
    let Some(h) = home("twice") else { return };

    assert!(!agent::status().unwrap().installed, "nothing installed yet");

    let first = agent::install(&exe(), &args()).unwrap();
    assert!(first.path.starts_with(&h.root), "{}", first.path.display());
    assert_eq!(read(&first.path), first.contents);
    let status = agent::status().unwrap();
    assert!(status.installed);
    assert_eq!(status.path, first.path);

    let second = agent::install(&exe(), &args()).unwrap();
    assert_eq!(second, first, "installing twice is the same agent");
    assert_eq!(read(&first.path), first.contents);

    assert!(
        agent::uninstall().unwrap(),
        "the first uninstall removed it"
    );
    assert!(!first.path.exists());
    assert!(!agent::status().unwrap().installed);
    assert!(
        !agent::uninstall().unwrap(),
        "the second has nothing to remove, and says so rather than failing"
    );
}

/// A relative executable is refused before anything is written: systemd rejects a
/// relative `ExecStart` outright, so writing the file would leave an agent that fails at
/// every login while `install` reported success.
#[test]
fn a_relative_executable_is_refused_and_leaves_no_file() {
    let Some(_h) = home("relative") else { return };
    let path = agent_path();
    let err = agent::install(Path::new("fontina"), &args()).unwrap_err();
    assert!(err.to_string().contains("absolute"), "{err}");
    assert!(
        !path.exists(),
        "nothing may be written when the plan is refused"
    );
    assert!(!agent::status().unwrap().installed);
}

/// Two installs that disagree about the binary or the index: the second silently wins.
///
/// This is not obviously wrong — the label is shared, so re-pointing an existing agent
/// is the only sensible reading of a second `install` — but nothing tells the reader
/// that the agent they had has been replaced, or what it used to run. Recorded so that
/// a change of mind about it is a deliberate one.
#[test]
fn a_second_install_repoints_the_agent_without_saying_what_it_replaced() {
    let Some(_h) = home("repoint") else { return };
    let one = agent::install(Path::new("/usr/bin/fontina"), &args()).unwrap();
    let two = agent::install(
        Path::new("/usr/local/bin/fontina"),
        &["restore".into(), "--db".into(), "/srv/fonts.db".into()],
    )
    .unwrap();
    assert_eq!(one.path, two.path, "one label, one file");
    let on_disk = read(&two.path);
    assert_eq!(on_disk, two.contents);
    assert!(on_disk.contains("/usr/local/bin/fontina"));
    assert!(
        !on_disk.contains("\"/usr/bin/fontina\""),
        "the first agent is gone, and nothing said so: {on_disk}"
    );
}

// ----- files fontina did not write -----

/// DEFECT: `install` overwrites whatever is at the agent's path.
///
/// `fs::write` truncates. There is no check that fontina wrote the file it is about to
/// replace, so a file somebody else put there — a unit of their own that happens to
/// carry this name, a leftover from another tool — is destroyed without a word.
///
/// This is the same class as the defect where `install` adopted a font the user had
/// placed in their own font directory: `lib.rs` grew `copy_slot`, `is_copy_slot` and
/// `present_by_hand` to stop it for fonts, and `agent.rs` has no equivalent. What it
/// should do is refuse, the way `install` refuses an identical font already present by
/// hand, and name the file it will not touch.
#[test]
fn install_overwrites_a_file_fontina_did_not_write() {
    let Some(_h) = home("foreign-install") else {
        return;
    };
    let path = agent_path();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(
        &path,
        "# not fontina's. Somebody's own unit, hand written.\n",
    )
    .unwrap();

    let plan = agent::install(&exe(), &args()).expect("today it succeeds");
    assert_eq!(
        read(&path),
        plan.contents,
        "their file was replaced by ours"
    );
    assert!(
        !read(&path).contains("Somebody's own unit"),
        "and their content is gone"
    );
}

/// DEFECT: `uninstall` deletes whatever is at the agent's path.
///
/// It computes the path from [`agent::plan`], throws the contents away and calls
/// `remove_file`. A file fontina never wrote is deleted and reported as a successful
/// uninstall. It should leave a file it does not recognise alone and say why.
#[test]
fn uninstall_deletes_a_file_fontina_did_not_write() {
    let Some(_h) = home("foreign-uninstall") else {
        return;
    };
    let path = agent_path();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, "# not fontina's\n").unwrap();

    assert!(
        agent::uninstall().expect("today it succeeds"),
        "reported as a removal"
    );
    assert!(!path.exists(), "somebody else's file is gone");
}

/// DEFECT: uninstall cannot tell "mine, unchanged" from "mine, edited".
///
/// Somebody adds a line to the unit — an `Environment=`, a `Nice=`, an `ExecStartPre=`
/// that waits for their network share to mount — and `uninstall` throws it away exactly
/// as it throws away the file it wrote itself. The two calls are indistinguishable in
/// their result, which is the whole problem: there is nothing for a caller to warn on.
///
/// `install` already returns the bytes it wrote, so recognising them later is a matter
/// of comparing, or of recording a hash the way `blake3` is already used for font slot
/// names. What it should do is refuse, or at the very least report that what it removed
/// was not what it installed.
#[test]
fn uninstall_cannot_tell_a_hand_edited_unit_from_an_untouched_one() {
    let Some(_h) = home("edited") else { return };

    // Untouched.
    let plan = agent::install(&exe(), &args()).unwrap();
    let untouched = agent::uninstall().unwrap();

    // Edited by hand.
    agent::install(&exe(), &args()).unwrap();
    let edit = format!("{}# waits for /srv to mount\n", plan.contents);
    std::fs::write(&plan.path, &edit).unwrap();
    assert_ne!(read(&plan.path), plan.contents, "the edit is really there");
    let edited = agent::uninstall().unwrap();

    assert_eq!(
        untouched, edited,
        "the same answer for both, so nothing downstream can warn about the edit"
    );
    assert!(!plan.path.exists(), "and the edit was thrown away silently");
}

/// A directory where the agent's file goes is refused by both halves, and survives.
///
/// This one is right, and worth holding still: `fs::write` and `remove_file` both fail
/// on a directory, so the error is the filesystem's rather than fontina's, and nothing
/// inside is lost.
#[test]
fn a_directory_at_the_agents_path_is_refused_by_both_halves() {
    let Some(_h) = home("directory") else { return };
    let path = agent_path();
    std::fs::create_dir_all(&path).unwrap();
    std::fs::write(path.join("keep.txt"), "someone's file\n").unwrap();

    assert!(
        agent::install(&exe(), &args()).is_err(),
        "a directory is not overwritten"
    );
    assert!(agent::uninstall().is_err(), "and it is not removed either");
    assert!(path.is_dir());
    assert_eq!(read(&path.join("keep.txt")), "someone's file\n");

    // `status` reports it as installed, because `Path::exists` is true for a directory.
    // Harmless on its own, but it is the reason a reader is told the agent is there
    // while every attempt to remove it fails.
    assert!(agent::status().unwrap().installed);
}

/// DEFECT: `install` writes *through* a symlink at the agent's path.
///
/// `fs::write` follows symlinks, so a link at the agent's path — left by a dotfile
/// manager, or by a previous version of somebody's own setup — makes `install`
/// overwrite the file at the far end, wherever that is. The agent's own path is still a
/// symlink afterwards, so `uninstall` removes the link and leaves the clobbered file
/// behind for good.
///
/// It should refuse anything at that path that is not a regular file fontina wrote, and
/// where it does write, replace the link rather than follow it.
#[test]
fn install_writes_through_a_symlink_and_clobbers_the_file_at_the_far_end() {
    let Some(h) = home("symlink") else { return };
    let path = agent_path();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let victim = h.root.join("important.conf");
    std::fs::write(&victim, "somebody's configuration\n").unwrap();
    symlink(&victim, &path);

    let plan = agent::install(&exe(), &args()).expect("today it succeeds");
    assert_eq!(
        read(&victim),
        plan.contents,
        "the unit was written into a file outside the agent's own path: {}",
        victim.display()
    );
    assert!(
        !read(&victim).contains("somebody's configuration"),
        "their file is gone"
    );
    assert!(
        std::fs::symlink_metadata(&path)
            .unwrap()
            .file_type()
            .is_symlink(),
        "and the agent's path is still only a link to it"
    );

    assert!(agent::uninstall().unwrap());
    assert!(!path.exists(), "the link went");
    assert!(
        victim.exists(),
        "the file it clobbered stays behind, with nothing left pointing at it"
    );
}

/// DEFECT: a broken symlink is invisible to `status` and removable by `uninstall`.
///
/// `status` asks `Path::exists`, which follows the link and answers false; `uninstall`
/// calls `remove_file`, which does not follow and removes it. So fontina reports that no
/// agent is installed and then removes one. Worse, `install` over a dangling link
/// creates the file at the far end — a path of somebody else's choosing.
#[test]
fn status_and_uninstall_disagree_about_a_broken_symlink() {
    let Some(h) = home("dangling") else { return };
    let path = agent_path();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let missing = h.root.join("gone/service.conf");
    symlink(&missing, &path);

    assert!(
        !agent::status().unwrap().installed,
        "status follows the link and sees nothing"
    );
    assert!(
        agent::uninstall().unwrap(),
        "uninstall does not follow it, and removes what status said was not there"
    );

    // And the other way round: installing over a dangling link writes at the far end.
    symlink(&missing, &path);
    std::fs::create_dir_all(missing.parent().unwrap()).unwrap();
    let plan = agent::install(&exe(), &args()).expect("today it succeeds");
    assert_eq!(
        read(&missing),
        plan.contents,
        "the unit landed at {}, not at the agent's path",
        missing.display()
    );
}

// ----- what the system still has to be told -----

/// systemd's enablement symlink is deliberately left to the reader, and `status` stops
/// reporting it the moment the unit file goes.
///
/// Removing it would mean running `systemctl` on somebody's behalf, which this crate
/// does not do anywhere; `AgentPlan::deactivate_with` is the command to show instead.
/// The consequence is a link that outlives the unit it points at, and nothing in
/// `status` mentions it, so this records the shape of what a reader is left holding.
#[test]
fn the_systemd_enablement_link_outlives_the_unit_and_status_stops_mentioning_it() {
    if !cfg!(all(unix, not(target_os = "macos"))) {
        return; // only systemd keeps enablement outside the unit file
    }
    let Some(h) = home("enable-link") else { return };
    let plan = agent::install(&exe(), &args()).unwrap();
    assert!(plan.deactivate_with.is_some(), "there is a step to show");

    let wants = h
        .root
        .join(".config/systemd/user/graphical-session.target.wants");
    std::fs::create_dir_all(&wants).unwrap();
    let link = wants.join("dev.fontina.restore.service");
    symlink(&plan.path, &link);
    assert!(
        agent::status().unwrap().enabled,
        "the file plus the link is what 'enabled' means"
    );

    assert!(agent::uninstall().unwrap());
    assert!(
        std::fs::symlink_metadata(&link).is_ok(),
        "fontina does not run systemctl, so the link is still there"
    );
    let status = agent::status().unwrap();
    assert!(!status.installed);
    assert!(
        !status.enabled,
        "and 'enabled' goes false with the unit, saying nothing about the link left over"
    );
}

/// Whatever else happens, nothing is written outside the home directory the agent was
/// pointed at. This is the promise the module header makes: installing the agent can
/// never affect anyone else on the machine.
#[test]
fn everything_the_agent_writes_stays_inside_the_home_directory() {
    let Some(h) = home("contained") else { return };
    let plan = agent::install(&exe(), &args()).unwrap();
    let real = std::fs::canonicalize(&h.root).unwrap();
    let written = std::fs::canonicalize(&plan.path).unwrap();
    assert!(
        written.starts_with(&real),
        "{} is outside {}",
        written.display(),
        real.display()
    );
    agent::uninstall().unwrap();
}
