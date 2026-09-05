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

//! The activation state machine: the real [`FontActivator`] for this system driven
//! against a real index, through every transition in both directions, twice over.
//!
//! Neither crate could test this alone. The core's own tests drive the index with no
//! activator, so they can record a state the filesystem never reached; the platform
//! tests drive the activator with no index, so they cannot see the state a person
//! actually queries. An activation is not finished until both agree, and what a person
//! does — activate, then install, then change their mind — walks the pair through
//! orderings neither half was written against. That is why `fontina-core` is a
//! dev-dependency here.
//!
//! Everything is sandboxed in a temporary home directory. Where a backend has to reach
//! the running operating system to do its job the test says so and skips, unless the
//! `platform-tests` feature says that is wanted:
//!
//! - install/uninstall is a copy or a symlink into the per-user font directory on both
//!   GNU/Linux and macOS, so a redirected `HOME` is enough; on Windows it also writes
//!   `HKCU` and calls `AddFontResource`, which is the login session itself.
//! - activate/deactivate is a symlink and a fontconfig snippet on GNU/Linux, but a real
//!   CoreText registration on macOS and a real GDI one on Windows.

use fontina_core::{ActivationState, FaceFilter, Index, ScanOptions};
use fontina_platform::{FontActivator, PlatformError, Scope};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

static ENV: Mutex<()> = Mutex::new(());

const REDIRECTED: [&str; 4] = ["HOME", "XDG_CONFIG_HOME", "XDG_DATA_HOME", "LOCALAPPDATA"];

/// True when `install`/`uninstall` stay inside a redirected home directory.
fn install_is_hermetic() -> bool {
    cfg!(unix)
}

/// True when `activate`/`deactivate` stay inside a redirected home directory.
fn activation_is_hermetic() -> bool {
    cfg!(all(unix, not(target_os = "macos")))
}

/// Whether a test may run: either it touches nothing outside the sandbox, or the reader
/// asked for the tests that do.
fn allowed(hermetic: bool, what: &str) -> bool {
    if hermetic || cfg!(feature = "platform-tests") {
        return true;
    }
    eprintln!("skipped {what}: it would reach the running login session on this system");
    false
}

/// A temporary home, a temporary font directory and an index in it.
struct Sandbox {
    root: PathBuf,
    fonts: PathBuf,
    index: Index,
    saved: Vec<(&'static str, Option<OsString>)>,
    _guard: MutexGuard<'static, ()>,
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        for (key, value) in &self.saved {
            // SAFETY: serialised through `ENV`, which is still held here.
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

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

/// A sandbox holding copies of `wanted` fixtures, scanned into a fresh index.
///
/// The name carries the test's own name and this process's id, so no two sandboxes —
/// and no two indexes — are ever the same path.
fn sandbox(name: &str, wanted: &[&str]) -> Sandbox {
    let guard = ENV.lock().unwrap_or_else(|e| e.into_inner());
    let root =
        std::env::temp_dir().join(format!("fontina-activation-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let fonts = root.join("fonts");
    std::fs::create_dir_all(&fonts).unwrap();
    std::fs::create_dir_all(root.join(".config")).unwrap();
    std::fs::create_dir_all(root.join(".local/share")).unwrap();
    for f in wanted {
        std::fs::copy(fixtures().join(f), fonts.join(f)).unwrap();
    }
    let saved = REDIRECTED
        .iter()
        .map(|k| (*k, std::env::var_os(k)))
        .collect();
    // SAFETY: serialised through `ENV`, restored in `Sandbox::drop`.
    unsafe {
        std::env::set_var("HOME", &root);
        std::env::set_var("XDG_CONFIG_HOME", root.join(".config"));
        std::env::set_var("XDG_DATA_HOME", root.join(".local/share"));
        std::env::set_var("LOCALAPPDATA", root.join("AppData/Local"));
    }
    let mut index = Index::open(&root.join("index.db")).unwrap();
    let report = fontina_core::scan::scan(
        &mut index,
        std::slice::from_ref(&fonts),
        &ScanOptions::default(),
    )
    .expect("the sandbox scans");
    assert!(report.failed.is_empty(), "{:?}", report.failed);
    Sandbox {
        root,
        fonts,
        index,
        saved,
        _guard: guard,
    }
}

/// The first face of one fixture, and the file it came from.
fn face_of(index: &Index, family: &str) -> (i64, PathBuf) {
    let f = index
        .list(&FaceFilter {
            family: Some(family.into()),
            ..Default::default()
        })
        .unwrap()
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("no face for {family}"));
    (f.id, PathBuf::from(f.path))
}

/// What the index says about one face, cross-checked three ways.
///
/// `activation`, `activations()` and the `--activation` filter are three different
/// queries over one row, and a person reads all three: `info` shows the first, the
/// `activations` command the second, `list --activation` the third. They must not be
/// able to disagree.
#[track_caller]
fn expect_state(index: &Index, id: i64, want: Option<ActivationState>) {
    let record = index.activation(id).unwrap();
    assert_eq!(
        record.as_ref().map(|r| r.state),
        want,
        "index.activation({id})"
    );

    let listed = index.activations().unwrap();
    let in_list = listed.iter().find(|r| r.face.id == id);
    assert_eq!(
        in_list.map(|r| r.state),
        want,
        "`activations` disagrees with `activation`"
    );

    let summary = index.summaries(&[id]).unwrap().pop().unwrap();
    assert_eq!(
        summary.activation, want,
        "the face summary disagrees with the record"
    );

    for state in [
        ActivationState::Session,
        ActivationState::User,
        ActivationState::Installed,
    ] {
        let filtered = index
            .list(&FaceFilter {
                activation: Some(state),
                ..Default::default()
            })
            .unwrap();
        let hit = filtered.iter().any(|f| f.id == id);
        assert_eq!(
            hit,
            want == Some(state),
            "`--activation {}` disagrees about face {id}",
            state.as_str()
        );
    }

    for (flag, expected) in [(true, want.is_some()), (false, want.is_none())] {
        let filtered = index
            .list(&FaceFilter {
                active: Some(flag),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(
            filtered.iter().any(|f| f.id == id),
            expected,
            "`--active {flag}` disagrees about face {id}"
        );
    }
}

/// Do what `fontina activate`/`install` does: register with the OS, then record it.
fn go_to(
    index: &mut Index,
    activator: &dyn FontActivator,
    file: &Path,
    id: i64,
    state: ActivationState,
) {
    let faces = index.file_faces(id).unwrap();
    match state {
        ActivationState::Installed => {
            let installed = activator.install(file).unwrap();
            index
                .set_activation(&faces, state, Some(&installed.to_string_lossy()))
                .unwrap();
        }
        ActivationState::Session | ActivationState::User => {
            let scope = if state == ActivationState::Session {
                Scope::Session
            } else {
                Scope::User
            };
            activator.activate(file, scope).unwrap();
            index.set_activation(&faces, state, None).unwrap();
        }
    }
}

/// Do what `fontina deactivate`/`uninstall` does.
fn go_to_none(index: &mut Index, activator: &dyn FontActivator, file: &Path, id: i64) {
    let faces = index.file_faces(id).unwrap();
    let record = index.activation(id).unwrap();
    match record.as_ref().and_then(|r| r.installed_path.as_deref()) {
        Some(p) => activator.uninstall(Path::new(p)).unwrap(),
        None => {
            activator.deactivate(file).unwrap();
        }
    }
    index.clear_activation(&faces).unwrap();
}

// ----- the state machine -----

/// None to session to user to installed and back to none, then the same the other way
/// round, then both again.
///
/// Repeating it is the point. A person who activates a font, installs it, changes their
/// mind and starts over exercises orderings a single pass never reaches: an install on
/// top of an activation, an activation on top of an install, and a second run over slots
/// and links the first run left behind.
#[test]
fn every_transition_in_both_directions_twice_over() {
    if !allowed(
        install_is_hermetic() && activation_is_hermetic(),
        "every_transition_in_both_directions_twice_over",
    ) {
        return;
    }
    let mut s = sandbox("transitions", &["Amiri-Regular.ttf"]);
    let (id, file) = face_of(&s.index, "Amiri");
    let activator = fontina_platform::activator();
    let faces = s.index.file_faces(id).unwrap();

    for pass in 1..=2 {
        for chain in [
            [
                ActivationState::Session,
                ActivationState::User,
                ActivationState::Installed,
            ],
            [
                ActivationState::Installed,
                ActivationState::User,
                ActivationState::Session,
            ],
        ] {
            expect_state(&s.index, id, None);
            assert!(
                s.index.activations().unwrap().is_empty(),
                "pass {pass} started with a clean index"
            );
            let mut copies: Vec<PathBuf> = Vec::new();

            for state in chain {
                go_to(&mut s.index, activator.as_ref(), &file, id, state);
                expect_state(&s.index, id, Some(state));

                // Every face in the file moves together, the way `files_for` groups them.
                for face in &faces {
                    expect_state(&s.index, *face, Some(state));
                }

                let record = s.index.activation(id).unwrap().unwrap();
                assert_eq!(
                    record.installed_path.is_some(),
                    state == ActivationState::Installed,
                    "only an install records where the copy went"
                );
                if let Some(p) = &record.installed_path {
                    assert!(Path::new(p).exists(), "{p} was recorded but is not there");
                    assert!(
                        Path::new(p).starts_with(&s.root),
                        "{p} is outside the sandbox"
                    );
                    copies.push(PathBuf::from(p));
                }
                assert_eq!(
                    s.index.stats().unwrap().activations,
                    faces.len() as i64,
                    "one row per face, however many times the state changed"
                );
            }

            go_to_none(&mut s.index, activator.as_ref(), &file, id);
            expect_state(&s.index, id, None);
            assert_eq!(s.index.stats().unwrap().activations, 0);

            if chain.last() == Some(&ActivationState::Installed) {
                let copy = copies.last().expect("an install records where it went");
                assert!(
                    !copy.exists(),
                    "pass {pass}: uninstall left {} behind",
                    copy.display()
                );
            }

            // Anything else the chain left behind is one of the two defects asserted on
            // their own below. Undo it properly — `uninstall` also takes back the
            // registry value on Windows — so the next pass starts from nothing and the
            // machine is left as it was found.
            for copy in copies.drain(..) {
                let _ = activator.uninstall(&copy);
                let _ = std::fs::remove_file(copy);
            }
            let _ = activator.deactivate(&file);
        }
    }
}

/// DEFECT: activating a font that is already installed orphans the installed copy.
///
/// `fontina install X` copies X into the per-user font directory and records where it
/// went. `fontina activate X` then registers X where it lies and calls `set_activation`
/// with `installed_path: None`, which the upsert writes straight over the recorded path.
/// The copy is still in the font directory, still visible to every application, and now
/// there is nothing in the index that knows about it — `fontina uninstall X` refuses,
/// because the record no longer says anything was installed.
///
/// The counterpart of the stranding below, from the other direction: a transition takes
/// the new state without giving up the old one. Here it also destroys the only record of
/// what would have to be given up.
#[test]
fn activating_a_font_that_is_already_installed_orphans_the_copy() {
    if !allowed(
        install_is_hermetic() && activation_is_hermetic(),
        "activating_a_font_that_is_already_installed_orphans_the_copy",
    ) {
        return;
    }
    let mut s = sandbox("orphan", &["Amiri-Regular.ttf"]);
    let (id, file) = face_of(&s.index, "Amiri");
    let activator = fontina_platform::activator();

    go_to(
        &mut s.index,
        activator.as_ref(),
        &file,
        id,
        ActivationState::Installed,
    );
    let copy = PathBuf::from(
        s.index
            .activation(id)
            .unwrap()
            .unwrap()
            .installed_path
            .expect("install records where the copy went"),
    );
    assert!(copy.exists());

    go_to(
        &mut s.index,
        activator.as_ref(),
        &file,
        id,
        ActivationState::User,
    );
    assert!(
        s.index
            .activation(id)
            .unwrap()
            .unwrap()
            .installed_path
            .is_none(),
        "the record of where the copy went was overwritten"
    );
    assert!(
        copy.exists(),
        "and the copy is still in the per-user font directory"
    );

    go_to_none(&mut s.index, activator.as_ref(), &file, id);
    expect_state(&s.index, id, None);
    assert!(
        copy.exists(),
        "so nothing fontina can be asked to do will ever remove {}",
        copy.display()
    );
    let _ = activator.uninstall(&copy);
    let _ = std::fs::remove_file(&copy);
    let _ = activator.deactivate(&file);
}

/// DEFECT: installing a font that is already activated leaves the activation registered,
/// and `uninstall` never takes it back.
///
/// `fontina activate --user X` registers X in place. `fontina install X` then copies it
/// into the per-user font directory and overwrites the index row with `installed`, but
/// nothing unregisters the in-place activation — and `fontina uninstall X` only removes
/// the copy, because `run_deactivate(uninstall: true)` calls `FontActivator::uninstall`
/// and never `deactivate`.
///
/// What is left is a font still visible to every application on the machine, with no row
/// in the index pointing at it and no fontina command that will find it: the state a
/// person reaches by trying a font, deciding to keep it, and then deciding not to. The
/// fix belongs in whichever half owns the transition, but the transition has to give the
/// old state up before it takes the new one.
#[test]
fn an_install_over_an_activation_strands_the_activation() {
    if !allowed(
        install_is_hermetic() && activation_is_hermetic(),
        "an_install_over_an_activation_strands_the_activation",
    ) {
        return;
    }
    let mut s = sandbox("stranded", &["Amiri-Regular.ttf"]);
    let (id, file) = face_of(&s.index, "Amiri");
    let activator = fontina_platform::activator();

    go_to(
        &mut s.index,
        activator.as_ref(),
        &file,
        id,
        ActivationState::User,
    );
    go_to(
        &mut s.index,
        activator.as_ref(),
        &file,
        id,
        ActivationState::Installed,
    );
    go_to_none(&mut s.index, activator.as_ref(), &file, id);

    expect_state(&s.index, id, None);
    assert!(
        s.index.activations().unwrap().is_empty(),
        "the index says nothing is active"
    );
    assert!(
        activator.deactivate(&file).unwrap(),
        "but the operating system still had a registration to take back"
    );
}

/// Two faces in different states at once, which is the ordinary case and the one a
/// single-face test cannot see: `activations` has to carry both, and each
/// `--activation` filter has to answer for its own state only.
#[test]
fn two_faces_in_different_states_do_not_bleed_into_each_others_filters() {
    if !allowed(
        install_is_hermetic(),
        "two_faces_in_different_states_do_not_bleed_into_each_others_filters",
    ) {
        return;
    }
    let mut s = sandbox(
        "two-states",
        &["Amiri-Regular.ttf", "SourceSerif4-Regular.otf"],
    );
    let (amiri, _) = face_of(&s.index, "Amiri");
    let (serif, serif_file) = face_of(&s.index, "Source Serif 4");
    let activator = fontina_platform::activator();

    // One recorded without touching the OS at all, one installed for real.
    s.index
        .set_activation(&[amiri], ActivationState::Session, None)
        .unwrap();
    go_to(
        &mut s.index,
        activator.as_ref(),
        &serif_file,
        serif,
        ActivationState::Installed,
    );

    expect_state(&s.index, amiri, Some(ActivationState::Session));
    expect_state(&s.index, serif, Some(ActivationState::Installed));
    assert_eq!(s.index.activations().unwrap().len(), 2);

    let session: Vec<i64> = s
        .index
        .list(&FaceFilter {
            activation: Some(ActivationState::Session),
            ..Default::default()
        })
        .unwrap()
        .into_iter()
        .map(|f| f.id)
        .collect();
    assert_eq!(session, [amiri]);

    go_to_none(&mut s.index, activator.as_ref(), &serif_file, serif);
    expect_state(&s.index, amiri, Some(ActivationState::Session));
    expect_state(&s.index, serif, None);
    assert_eq!(s.index.activations().unwrap().len(), 1);
}

/// Activating a face twice is one record, not two, and the second activation is the one
/// that counts. `restore` walks `activations()`, so a duplicate row would mean the same
/// font restored twice at every login.
#[test]
fn a_face_activated_twice_keeps_one_record() {
    let mut s = sandbox("twice", &["Amiri-Regular.ttf"]);
    let (id, _) = face_of(&s.index, "Amiri");
    let faces = s.index.file_faces(id).unwrap();

    s.index
        .set_activation(&faces, ActivationState::Session, None)
        .unwrap();
    let first = s.index.activation(id).unwrap().unwrap().activated_at;
    s.index
        .set_activation(&faces, ActivationState::Session, None)
        .unwrap();

    assert_eq!(s.index.activations().unwrap().len(), faces.len());
    let second = s.index.activation(id).unwrap().unwrap();
    assert_eq!(second.state, ActivationState::Session);
    assert!(
        second.activated_at >= first,
        "the later activation is the one recorded"
    );
    expect_state(&s.index, id, Some(ActivationState::Session));
}

/// A face that has left the index takes its activation with it, so `restore` never sees
/// a record it cannot resolve. Curation survives a rescan; a deletion does not.
#[test]
fn removing_a_face_removes_the_activation_restore_would_have_walked() {
    let mut s = sandbox(
        "removed",
        &["Amiri-Regular.ttf", "SourceSerif4-Regular.otf"],
    );
    let (amiri, amiri_file) = face_of(&s.index, "Amiri");
    let (serif, _) = face_of(&s.index, "Source Serif 4");
    s.index
        .set_activation(&[amiri], ActivationState::Session, None)
        .unwrap();
    s.index
        .set_activation(&[serif], ActivationState::User, None)
        .unwrap();
    assert_eq!(s.index.activations().unwrap().len(), 2);

    assert!(
        s.index.remove_file(&amiri_file.to_string_lossy()).unwrap(),
        "the file leaves the index"
    );
    let left = s.index.activations().unwrap();
    assert_eq!(left.len(), 1, "the orphaned activation went with it");
    assert_eq!(left[0].face.id, serif);
    assert!(s.index.activation(amiri).unwrap().is_none());
}

// ----- what `restore` walks into -----

/// The surprises an index that has moved on hands the activator, one call at a time.
///
/// `restore` runs unattended at login over records that were true when they were
/// written, so every one of these has to come back as an error value rather than a
/// panic — the crate's rule that errors are values, applied where it is load-bearing.
#[test]
fn a_font_that_has_moved_on_is_an_error_value_not_a_panic() {
    if !allowed(
        install_is_hermetic() && activation_is_hermetic(),
        "a_font_that_has_moved_on_is_an_error_value_not_a_panic",
    ) {
        return;
    }
    let s = sandbox("moved-on", &["Amiri-Regular.ttf"]);
    let activator = fontina_platform::activator();

    // The file is gone.
    let gone = s.fonts.join("gone.ttf");
    assert!(matches!(
        activator.activate(&gone, Scope::Session),
        Err(PlatformError::Io(..))
    ));
    assert!(matches!(
        activator.install(&gone),
        Err(PlatformError::Io(..))
    ));
    assert!(
        !activator.deactivate(&gone).unwrap(),
        "nothing to take back is not an error"
    );

    // The path is now a directory. `regular_file` canonicalises first, so this is the
    // one that would reach `fs::copy` if it were not caught.
    let dir = s.fonts.join("was-a-font.ttf");
    std::fs::create_dir_all(&dir).unwrap();
    assert!(matches!(
        activator.activate(&dir, Scope::Session),
        Err(PlatformError::NotAFile(..))
    ));
    assert!(matches!(
        activator.install(&dir),
        Err(PlatformError::NotAFile(..))
    ));
    assert!(dir.is_dir(), "and the directory is left alone");

    // An installed path that was never fontina's.
    let outsider = s.fonts.join("Amiri-Regular.ttf");
    assert!(matches!(
        activator.uninstall(&outsider),
        Err(PlatformError::NotManaged(..))
    ));
    assert!(outsider.exists(), "a font fontina did not install survives");
}

// ----- the rules that gate activation -----

/// The conflict rules, each one on its own, and the state each leaves for `--replace`.
///
/// `run_activate --replace` switches on the conflicting face's own activation state:
/// `installed` is uninstalled, any other activation is deactivated, and `None` — a face
/// that conflicts only because it sits in a system font directory — cannot be replaced
/// at all, because fontina does not touch system directories. So what a conflict *is*
/// decides what `--replace` is able to do about it, and this pins both.
#[test]
fn the_conflict_rules_and_what_replace_can_do_about_each() {
    let mut s = sandbox(
        "conflicts",
        &[
            "inter-latin-400-normal.woff",
            "inter-latin-400-normal.woff2",
        ],
    );
    let one = s
        .index
        .list(&FaceFilter {
            container: Some("woff".into()),
            ..Default::default()
        })
        .unwrap()[0]
        .id;
    let two = s
        .index
        .list(&FaceFilter {
            container: Some("woff2".into()),
            ..Default::default()
        })
        .unwrap()[0]
        .id;

    // A face never conflicts with itself, or with a sibling face in the same file. The
    // query excludes its own `file_id`, which is what stops `activate` from refusing to
    // activate something because it is already activated.
    s.index
        .set_activation(&[one], ActivationState::User, None)
        .unwrap();
    assert!(
        s.index.conflicts(one, &[]).unwrap().is_empty(),
        "a face is not its own conflict"
    );
    s.index.clear_activation(&[one]).unwrap();

    // Same PostScript name, and the other one is active: a conflict fontina can undo,
    // because it is fontina's own registration.
    for (state, expected) in [
        (
            ActivationState::Session,
            "same PostScript name, active (session)",
        ),
        (ActivationState::User, "same PostScript name, active (user)"),
        (
            ActivationState::Installed,
            "same PostScript name, active (installed)",
        ),
    ] {
        s.index.set_activation(&[two], state, None).unwrap();
        let c = s.index.conflicts(one, &[]).unwrap();
        assert_eq!(c.len(), 1, "{state:?}");
        assert_eq!(c[0].reason, expected);
        assert_eq!(
            c[0].face.activation,
            Some(state),
            "`--replace` reads this to decide between uninstall and deactivate"
        );
    }
    s.index.clear_activation(&[two]).unwrap();

    // Same family and style, different PostScript name. No pair of fixtures can produce
    // this — a font with the same family and style almost always carries the same
    // PostScript name too — so one is renamed in the index directly, which is also the
    // shape a font with no PostScript name at all takes.
    rename_postscript(&s.root.join("index.db"), two, "Inter-RegularAlias");
    s.index
        .set_activation(&[two], ActivationState::User, None)
        .unwrap();
    let c = s.index.conflicts(one, &[]).unwrap();
    assert_eq!(c.len(), 1);
    assert_eq!(
        c[0].reason, "same family and style, active (user)",
        "the second rule stands on its own"
    );
    s.index.clear_activation(&[two]).unwrap();

    // Present in a system font directory. Nothing fontina did, so `--replace` has
    // nothing to undo: the CLI warns that the OS decides which one wins.
    let root = std::fs::canonicalize(&s.fonts)
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let c = s.index.conflicts(one, &[root]).unwrap();
    assert_eq!(c.len(), 1);
    assert_eq!(
        c[0].reason,
        "same family and style, present in a system font directory"
    );
    assert!(
        c[0].face.activation.is_none(),
        "no activation to undo, so `--replace` can only warn"
    );

    // And a face that is not in the index at all is an error rather than an empty list,
    // so a stale id cannot read as "nothing in the way".
    assert!(s.index.conflicts(987_654, &[]).is_err());
}

/// Rewrite one face's PostScript name straight in the database.
///
/// Nothing in the core's public API can do this, and no fixture provides two fonts that
/// share a family and style without also sharing a PostScript name — so the second
/// conflict rule would otherwise never be reached by a test at all.
fn rename_postscript(db: &Path, face_id: i64, name: &str) {
    let conn = rusqlite::Connection::open(db).unwrap();
    let n = conn
        .execute(
            "UPDATE faces SET postscript_name = ?2 WHERE id = ?1",
            rusqlite::params![face_id, name],
        )
        .unwrap();
    assert_eq!(n, 1, "one face renamed");
}
