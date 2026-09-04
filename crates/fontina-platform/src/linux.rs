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

//! Linux and the BSDs: fontconfig reads `$XDG_DATA_HOME/fonts` recursively, so per-user
//! install and activation are symlinks into two subdirectories there. The active
//! directory is also declared in a fontconfig snippet so it works even for users whose
//! `fonts.conf` does not include the XDG directory. `fc-cache` is run best-effort so
//! applications that do not watch the directory pick the change up promptly.
//!
//! Everything reads the environment at call time, so tests isolate themselves with
//! `XDG_DATA_HOME` and `XDG_CONFIG_HOME`.

use super::{FontActivator, PlatformError, Result, Scope, io, is_under, regular_file, slot_name};
use std::path::{Path, PathBuf};

pub struct Fontconfig;

pub(crate) fn data_home() -> Option<PathBuf> {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| super::home().map(|h| h.join(".local/share")))
}

fn config_home() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| super::home().map(|h| h.join(".config")))
}

/// `$XDG_DATA_HOME/fonts/fontina`: persistent installs.
pub fn install_dir() -> Result<PathBuf> {
    data_home()
        .map(|d| d.join("fonts").join("fontina"))
        .ok_or(PlatformError::NoUserDir)
}

/// `$XDG_DATA_HOME/fonts/fontina-active`: activations.
pub fn active_dir() -> Result<PathBuf> {
    data_home()
        .map(|d| d.join("fonts").join("fontina-active"))
        .ok_or(PlatformError::NoUserDir)
}

/// `$XDG_CONFIG_HOME/fontconfig/conf.d/50-fontina.conf`.
pub fn fontconfig_snippet() -> Option<PathBuf> {
    config_home().map(|c| c.join("fontconfig").join("conf.d").join("50-fontina.conf"))
}

const SNIPPET: &str = r#"<?xml version="1.0"?>
<!DOCTYPE fontconfig SYSTEM "fonts.dtd">
<!-- Written by fontina. Fonts activated with `fontina activate` are linked here. -->
<fontconfig>
  <dir prefix="xdg">fonts/fontina-active</dir>
  <dir prefix="xdg">fonts/fontina</dir>
</fontconfig>
"#;

fn ensure_snippet() -> Result<()> {
    let Some(path) = fontconfig_snippet() else {
        return Ok(());
    };
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(io(parent))?;
    }
    std::fs::write(&path, SNIPPET).map_err(io(&path))
}

/// Ask fontconfig to refresh one directory. Missing `fc-cache` is not an error.
fn refresh(dir: &Path) {
    let _ = std::process::Command::new("fc-cache")
        .arg("-f")
        .arg(dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
}

fn link_target(link: &Path) -> Option<PathBuf> {
    std::fs::read_link(link).ok()
}

/// Link `file` into `dir`; returns the link path. Idempotent.
fn link_into(dir: &Path, file: &Path) -> Result<PathBuf> {
    let file = regular_file(file)?;
    std::fs::create_dir_all(dir).map_err(io(dir))?;
    if let Some(existing) = find_link(dir, &file)? {
        return Ok(existing);
    }
    let slot = slot_name(dir, &file, |p| link_target(p).as_deref() == Some(&file));
    if std::fs::symlink_metadata(&slot).is_ok() {
        std::fs::remove_file(&slot).map_err(io(&slot))?;
    }
    std::os::unix::fs::symlink(&file, &slot).map_err(io(&slot))?;
    refresh(dir);
    Ok(slot)
}

/// The link in `dir` that points at `file`, if any.
fn find_link(dir: &Path, file: &Path) -> Result<Option<PathBuf>> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Ok(None);
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if link_target(&p).as_deref() == Some(file) {
            return Ok(Some(p));
        }
    }
    Ok(None)
}

fn unlink_from(dir: &Path, file: &Path) -> Result<bool> {
    let file = std::fs::canonicalize(file).unwrap_or_else(|_| file.to_path_buf());
    match find_link(dir, &file)? {
        Some(link) => {
            std::fs::remove_file(&link).map_err(io(&link))?;
            refresh(dir);
            Ok(true)
        }
        None => Ok(false),
    }
}

impl FontActivator for Fontconfig {
    fn install(&self, file: &Path) -> Result<PathBuf> {
        let dir = install_dir()?;
        let link = link_into(&dir, file)?;
        ensure_snippet()?;
        Ok(link)
    }

    fn uninstall(&self, installed: &Path) -> Result<()> {
        let dir = install_dir()?;
        if !is_under(installed, &dir) {
            return Err(PlatformError::NotManaged(installed.to_path_buf(), dir));
        }
        match std::fs::symlink_metadata(installed) {
            Ok(_) => std::fs::remove_file(installed).map_err(io(installed))?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(PlatformError::Io(installed.to_path_buf(), e)),
        }
        refresh(&dir);
        Ok(())
    }

    fn activate(&self, file: &Path, _scope: Scope) -> Result<()> {
        // Session and user activations look the same to fontconfig; the index records
        // which is which and `fontina restore` (or a login agent) re-links session ones.
        let dir = active_dir()?;
        link_into(&dir, file)?;
        ensure_snippet()
    }

    fn deactivate(&self, file: &Path) -> Result<()> {
        unlink_from(&active_dir()?, file).map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Environment variables are process-global; serialise the tests that set them.
    static ENV: Mutex<()> = Mutex::new(());

    fn sandbox(name: &str) -> (PathBuf, PathBuf) {
        let root =
            std::env::temp_dir().join(format!("fontina-linux-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let data = root.join("data");
        let config = root.join("config");
        std::fs::create_dir_all(&data).unwrap();
        std::fs::create_dir_all(&config).unwrap();
        (data, config)
    }

    fn fixture() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/Amiri-Regular.ttf")
    }

    #[test]
    fn install_links_into_xdg_and_writes_the_snippet() {
        let _g = ENV.lock().unwrap();
        let (data, config) = sandbox("install");
        // SAFETY: tests in this module are serialised through ENV and restore nothing:
        // the sandbox paths are only meaningful inside this process.
        unsafe {
            std::env::set_var("XDG_DATA_HOME", &data);
            std::env::set_var("XDG_CONFIG_HOME", &config);
        }
        let a = Fontconfig;
        let link = a.install(&fixture()).unwrap();
        assert_eq!(link, data.join("fonts/fontina/Amiri-Regular.ttf"));
        assert_eq!(
            std::fs::read_link(&link).unwrap(),
            std::fs::canonicalize(fixture()).unwrap()
        );
        assert_eq!(a.install(&fixture()).unwrap(), link, "idempotent");
        let snippet = config.join("fontconfig/conf.d/50-fontina.conf");
        assert!(snippet.exists());
        assert!(
            std::fs::read_to_string(&snippet)
                .unwrap()
                .contains("fontina-active")
        );

        // A different file with the same basename gets a distinct slot.
        let other_dir = data.join("elsewhere");
        std::fs::create_dir_all(&other_dir).unwrap();
        let other = other_dir.join("Amiri-Regular.ttf");
        std::fs::copy(fixture(), &other).unwrap();
        let link2 = a.install(&other).unwrap();
        assert_ne!(link2, link);
        assert!(
            link2
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("Amiri-Regular-")
        );

        a.uninstall(&link).unwrap();
        assert!(std::fs::symlink_metadata(&link).is_err());
        a.uninstall(&link).unwrap(); // already gone: fine
        assert!(matches!(
            a.uninstall(&fixture()),
            Err(PlatformError::NotManaged(..))
        ));
        assert!(a.install(&other_dir).is_err(), "directories are refused");
    }

    #[test]
    fn activate_and_deactivate_by_target() {
        let _g = ENV.lock().unwrap();
        let (data, config) = sandbox("activate");
        unsafe {
            std::env::set_var("XDG_DATA_HOME", &data);
            std::env::set_var("XDG_CONFIG_HOME", &config);
        }
        let a = Fontconfig;
        a.activate(&fixture(), Scope::Session).unwrap();
        let active = data.join("fonts/fontina-active");
        assert_eq!(std::fs::read_dir(&active).unwrap().count(), 1);
        a.activate(&fixture(), Scope::User).unwrap();
        assert_eq!(std::fs::read_dir(&active).unwrap().count(), 1, "idempotent");
        a.deactivate(&fixture()).unwrap();
        assert_eq!(std::fs::read_dir(&active).unwrap().count(), 0);
        a.deactivate(&fixture()).unwrap(); // nothing active: fine
        assert!(matches!(
            a.activate(Path::new("/nonexistent/font.ttf"), Scope::Session),
            Err(PlatformError::Io(..))
        ));
    }
}
