//! macOS: CoreText's font manager registers files in place with a scope. `user` scope
//! persists across logins without copying; `session` ends at logout. Persistent install
//! copies into `~/Library/Fonts`, which fontd watches.

use super::{FontActivator, PlatformError, Result, Scope, io, is_under, regular_file, slot_name};
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
        let same_bytes = |p: &Path| {
            std::fs::read(p).ok().map(|b| blake3::hash(&b))
                == std::fs::read(&file).ok().map(|b| blake3::hash(&b))
        };
        let slot = slot_name(&dir, &file, same_bytes);
        if !slot.exists() {
            std::fs::copy(&file, &slot).map_err(io(&slot))?;
        }
        Ok(slot)
    }

    fn uninstall(&self, installed: &Path) -> Result<()> {
        let dir = user_fonts_dir()?;
        if !is_under(installed, &dir) {
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

    fn deactivate(&self, file: &Path) -> Result<()> {
        let file = std::fs::canonicalize(file).unwrap_or_else(|_| file.to_path_buf());
        let mut last = None;
        for scope in [SCOPE_SESSION, SCOPE_USER] {
            match ct_call(&file, CTFontManagerUnregisterFontsForURL, scope)? {
                None => return Ok(()),
                Some(ERR_NOT_REGISTERED) => {}
                Some(code) => last = Some(code),
            }
        }
        match last {
            Some(code) => Err(describe(&file, code)),
            None => Ok(()),
        }
    }
}

#[cfg(all(test, feature = "platform-tests"))]
mod tests {
    use super::*;

    fn fixture() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/Amiri-Regular.ttf")
    }

    #[test]
    fn session_activation_round_trips() {
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
        let a = CoreText;
        let installed = a.install(&fixture()).unwrap();
        assert!(installed.starts_with(user_fonts_dir().unwrap()));
        assert!(installed.exists());
        assert_eq!(a.install(&fixture()).unwrap(), installed, "idempotent");
        a.uninstall(&installed).unwrap();
        assert!(!installed.exists());
        assert!(matches!(
            a.uninstall(&fixture()),
            Err(PlatformError::NotManaged(..))
        ));
    }
}
