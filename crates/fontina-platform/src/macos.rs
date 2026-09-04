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

//! macOS: CoreText's font manager registers files in place with a scope. `user` scope
//! persists across logins without copying; `session` ends at logout. Persistent install
//! copies into `~/Library/Fonts`, which fontd watches.

use super::{
    FontActivator, PlatformError, Result, Scope, copy_slot, io, is_copy_slot, is_under,
    present_by_hand, regular_file,
};
use core_foundation::base::TCFType;
use core_foundation::error::{CFError, CFErrorRef};
use core_foundation::url::{CFURL, CFURLRef};
use std::path::{Path, PathBuf};

pub struct CoreText;

// CTFontManagerScope
const SCOPE_USER: u32 = 2;
const SCOPE_SESSION: u32 = 3;
// CTFontManagerError
const ERR_ALREADY_REGISTERED: isize = 105;
const ERR_NOT_REGISTERED: isize = 201;

#[link(name = "CoreText", kind = "framework")]
unsafe extern "C" {
    fn CTFontManagerRegisterFontsForURL(url: CFURLRef, scope: u32, error: *mut CFErrorRef) -> bool;
    fn CTFontManagerUnregisterFontsForURL(
        url: CFURLRef,
        scope: u32,
        error: *mut CFErrorRef,
    ) -> bool;
}

fn scope_code(scope: Scope) -> u32 {
    match scope {
        Scope::Session => SCOPE_SESSION,
        Scope::User => SCOPE_USER,
    }
}

/// Run a CoreText registration call; `Ok(code)` carries the CTFontManagerError code
/// when the call reported one.
fn ct_call(
    file: &Path,
    f: unsafe extern "C" fn(CFURLRef, u32, *mut CFErrorRef) -> bool,
    scope: u32,
) -> Result<Option<isize>> {
    let url = CFURL::from_path(file, false)
        .ok_or_else(|| PlatformError::Os(format!("{}: not a valid file URL", file.display())))?;
    let mut err: CFErrorRef = std::ptr::null_mut();
    // SAFETY: `url` outlives the call; `err` is a valid out-pointer that CoreText fills
    // with a +1 CFError we take ownership of below.
    let ok = unsafe { f(url.as_concrete_TypeRef(), scope, &mut err) };
    if ok {
        return Ok(None);
    }
    if err.is_null() {
        return Err(PlatformError::Os(format!(
            "{}: CoreText refused the font without an error",
            file.display()
        )));
    }
    // SAFETY: non-null and created by CoreText for us to release.
    let error = unsafe { CFError::wrap_under_create_rule(err) };
    Ok(Some(error.code() as isize))
}

fn describe(file: &Path, code: isize) -> PlatformError {
    let what = match code {
        101 => "the file is not a font file",
        102 => "the font file is invalid",
        103 => "the font file is unsupported",
        104 => "the font contains a table not supported",
        105 => "already registered",
        106 => "the font is in use",
        108 => "the file is not in an allowed location",
        201 => "not registered",
        202 => "a system font cannot be unregistered",
        203 => "registration scope not supported",
        _ => "CoreText error",
    };
    PlatformError::Os(format!("{}: {what} ({code})", file.display()))
}

pub fn user_fonts_dir() -> Result<PathBuf> {
    super::home()
        .map(|h| h.join("Library").join("Fonts"))
        .ok_or(PlatformError::NoUserDir)
}

impl FontActivator for CoreText {
    fn install(&self, file: &Path) -> Result<PathBuf> {
        let file = regular_file(file)?;
        let dir = user_fonts_dir()?;
        std::fs::create_dir_all(&dir).map_err(io(&dir))?;
        let slot = copy_slot(&dir, &file);
        if slot.exists() {
            return Ok(slot);
        }
        if let Some(theirs) = present_by_hand(&dir, &file) {
            return Err(PlatformError::AlreadyPresent(theirs));
        }
        std::fs::copy(&file, &slot).map_err(io(&slot))?;
        Ok(slot)
    }

    fn uninstall(&self, installed: &Path) -> Result<()> {
        let dir = user_fonts_dir()?;
        if !is_under(installed, &dir) || !is_copy_slot(installed) {
            return Err(PlatformError::NotManaged(installed.to_path_buf(), dir));
        }
        match std::fs::remove_file(installed) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(PlatformError::Io(installed.to_path_buf(), e)),
        }
    }

    fn activate(&self, file: &Path, scope: Scope) -> Result<()> {
        let file = regular_file(file)?;
        match ct_call(&file, CTFontManagerRegisterFontsForURL, scope_code(scope))? {
            None | Some(ERR_ALREADY_REGISTERED) => Ok(()),
            Some(code) => Err(describe(&file, code)),
        }
    }

    fn deactivate(&self, file: &Path) -> Result<bool> {
        let file = std::fs::canonicalize(file).unwrap_or_else(|_| file.to_path_buf());
        // Both scopes, always. A font activated for the session and then for the user is
        // registered twice; stopping at the first success would leave the other
        // registration behind, surviving a logout, with nothing left to point at it.
        let mut last = None;
        let mut removed = false;
        for scope in [SCOPE_SESSION, SCOPE_USER] {
            match ct_call(&file, CTFontManagerUnregisterFontsForURL, scope)? {
                None => removed = true,
                Some(ERR_NOT_REGISTERED) => {}
                Some(code) => last = Some(code),
            }
        }
        match last {
            Some(code) if !removed => Err(describe(&file, code)),
            _ => Ok(removed),
        }
    }
}

#[cfg(all(test, feature = "platform-tests"))]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // CoreText registration is process-global, and so is ~/Library/Fonts; serialise the
    // tests that touch either.
    static SYSTEM: Mutex<()> = Mutex::new(());

    fn fixture() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/Amiri-Regular.ttf")
    }

    #[test]
    fn session_activation_round_trips() {
        let _guard = SYSTEM.lock().unwrap_or_else(|e| e.into_inner());
        let a = CoreText;
        a.activate(&fixture(), Scope::Session).unwrap();
        a.activate(&fixture(), Scope::Session).unwrap(); // already registered: fine
        a.deactivate(&fixture()).unwrap();
        a.deactivate(&fixture()).unwrap(); // not registered: fine
        assert!(
            a.activate(Path::new("/nonexistent.ttf"), Scope::Session)
                .is_err()
        );
    }

    #[test]
    fn install_copies_into_user_fonts() {
        let _guard = SYSTEM.lock().unwrap_or_else(|e| e.into_inner());
        let a = CoreText;
        let installed = a.install(&fixture()).unwrap();
        assert!(installed.starts_with(user_fonts_dir().unwrap()));
        assert!(installed.exists());
        assert!(
            super::super::is_copy_slot(&installed),
            "the slot name proves fontina wrote it: {}",
            installed.display()
        );
        assert_eq!(a.install(&fixture()).unwrap(), installed, "idempotent");
        a.uninstall(&installed).unwrap();
        assert!(!installed.exists());
        assert!(matches!(
            a.uninstall(&fixture()),
            Err(PlatformError::NotManaged(..))
        ));
    }

    #[test]
    fn a_font_the_user_installed_by_hand_is_left_alone() {
        let _guard = SYSTEM.lock().unwrap_or_else(|e| e.into_inner());
        // The user drops a font into ~/Library/Fonts themselves, then installs the same
        // file from somewhere else. Adopting their copy as our slot would mean deleting
        // their file on uninstall, so we refuse and say why.
        let a = CoreText;
        let dir = user_fonts_dir().unwrap();
        std::fs::create_dir_all(&dir).unwrap();
        let source =
            std::env::temp_dir().join(format!("fontina-byhand-{}.ttf", std::process::id()));
        std::fs::copy(fixture(), &source).unwrap();
        let theirs = dir.join(source.file_name().unwrap());
        std::fs::copy(&source, &theirs).unwrap();

        let err = a.install(&source).expect_err("must not adopt their file");
        assert!(
            matches!(err, PlatformError::AlreadyPresent(ref p) if *p == theirs),
            "{err}"
        );
        assert!(theirs.exists(), "their file is untouched");

        // And uninstall refuses to delete it even when handed the path directly, because
        // the name is not one fontina would have chosen.
        assert!(matches!(
            a.uninstall(&theirs),
            Err(PlatformError::NotManaged(..))
        ));
        assert!(theirs.exists());
        std::fs::remove_file(&theirs).unwrap();
        std::fs::remove_file(&source).unwrap();
    }

    #[test]
    fn deactivate_clears_both_scopes_and_reports_what_it_did() {
        let _guard = SYSTEM.lock().unwrap_or_else(|e| e.into_inner());
        let a = CoreText;
        assert!(
            !a.deactivate(&fixture()).unwrap(),
            "nothing was registered yet"
        );
        a.activate(&fixture(), Scope::Session).unwrap();
        a.activate(&fixture(), Scope::User).unwrap();
        assert!(a.deactivate(&fixture()).unwrap());
        // Stopping at the first scope would have left the user-scope registration behind,
        // surviving a logout with nothing left pointing at it.
        assert!(
            !a.deactivate(&fixture()).unwrap(),
            "both scopes were cleared"
        );
    }
}
