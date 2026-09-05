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

//! Platform integration: where fonts live on each OS, and the activation backend that
//! makes a font file visible to other applications without touching system directories
//! or asking for elevation.
//!
//! One trait, three implementations, all per-user:
//!
//! | OS | install | activate |
//! |---|---|---|
//! | Linux | symlink into `$XDG_DATA_HOME/fonts/fontina/` | symlink into `$XDG_DATA_HOME/fonts/fontina-active/`, declared in `~/.config/fontconfig/conf.d/50-fontina.conf` |
//! | macOS | copy into `~/Library/Fonts` | `CTFontManagerRegisterFontsForURL`, session or user scope |
//! | Windows | copy into `%LOCALAPPDATA%\Microsoft\Windows\Fonts` + `HKCU\...\Fonts` | `AddFontResourceExW` (+ registry for user scope) |
//!
//! Deleting fontina leaves everything reversible: links and copies are ordinary files in
//! the per-user font directory, registrations are the OS's own per-user mechanisms.

pub mod agent;
pub mod open;
pub mod tags;

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[cfg(all(unix, not(target_os = "macos")))]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

/// How long an activation should last.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    /// Until logout or reboot.
    Session,
    /// Persistent for the current user.
    User,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemFontDir {
    pub path: PathBuf,
    /// True for the directory a per-user install writes to.
    pub user_writable: bool,
    pub description: &'static str,
}

#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[error("not supported on this platform: {0}")]
    Unsupported(&'static str),
    #[error("{0}: {1}")]
    Io(PathBuf, #[source] std::io::Error),
    #[error("{0} is not a file")]
    NotAFile(PathBuf),
    #[error("{0} was not installed by fontina (it is outside {1})")]
    NotManaged(PathBuf, PathBuf),
    #[error(
        "an identical font is already in your font directory at {0}, put there by hand; fontina will not touch it"
    )]
    AlreadyPresent(PathBuf),
    #[error("no per-user font directory on this system")]
    NoUserDir,
    #[error("{0} is on a filesystem that does not keep file tags")]
    NoTags(PathBuf),
    #[error("{0}")]
    Os(String),
}

pub type Result<T> = std::result::Result<T, PlatformError>;

/// The activation backend contract. One implementation per OS; get it from
/// [`activator`].
pub trait FontActivator {
    /// Persistent, per-user install of `file`. Returns the path the OS now reads (a copy
    /// or link inside the per-user font directory). Never touches system directories.
    fn install(&self, file: &Path) -> Result<PathBuf>;
    /// Undo [`FontActivator::install`], given the path it returned.
    fn uninstall(&self, installed: &Path) -> Result<()>;
    /// Make `file` visible in place, for the session or persistently for the user.
    fn activate(&self, file: &Path, scope: Scope) -> Result<()>;
    /// Undo [`FontActivator::activate`] for every scope. `Ok(false)` means there was
    /// nothing registered to undo, which is not an error but is worth reporting: it is
    /// how a caller learns that clearing its own record is all that happened.
    fn deactivate(&self, file: &Path) -> Result<bool>;
    /// Font directories the OS reads, in precedence order.
    fn font_dirs(&self) -> Vec<SystemFontDir> {
        system_font_dirs()
    }
}

/// The backend for the running OS.
pub fn activator() -> Box<dyn FontActivator> {
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Box::new(linux::Fontconfig)
    }
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::CoreText)
    }
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::Gdi)
    }
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn home() -> Option<PathBuf> {
    directories::BaseDirs::new().map(|b| b.home_dir().to_path_buf())
}

/// Font directories for the current OS. The first `user_writable` entry is the per-user
/// install location.
pub fn system_font_dirs() -> Vec<SystemFontDir> {
    let mut dirs = Vec::new();
    #[cfg(target_os = "macos")]
    {
        if let Some(h) = home() {
            dirs.push(SystemFontDir {
                path: h.join("Library/Fonts"),
                user_writable: true,
                description: "user fonts",
            });
        }
        dirs.push(SystemFontDir {
            path: "/Library/Fonts".into(),
            user_writable: false,
            description: "local fonts (all users)",
        });
        dirs.push(SystemFontDir {
            path: "/System/Library/Fonts".into(),
            user_writable: false,
            description: "system fonts",
        });
        dirs.push(SystemFontDir {
            path: "/System/Library/Fonts/Supplemental".into(),
            user_writable: false,
            description: "system supplemental fonts",
        });
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            dirs.push(SystemFontDir {
                path: PathBuf::from(local)
                    .join("Microsoft")
                    .join("Windows")
                    .join("Fonts"),
                user_writable: true,
                description: "per-user fonts",
            });
        }
        let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
        dirs.push(SystemFontDir {
            path: PathBuf::from(windir).join("Fonts"),
            user_writable: false,
            description: "system fonts",
        });
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Some(d) = linux::data_home() {
            dirs.push(SystemFontDir {
                path: d.join("fonts"),
                user_writable: true,
                description: "user fonts (XDG_DATA_HOME/fonts)",
            });
        }
        if let Some(h) = home() {
            dirs.push(SystemFontDir {
                path: h.join(".fonts"),
                user_writable: false,
                description: "legacy user fonts",
            });
        }
        let data_dirs =
            std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
        for d in data_dirs.split(':').filter(|s| !s.is_empty()) {
            dirs.push(SystemFontDir {
                path: PathBuf::from(d).join("fonts"),
                user_writable: false,
                description: "system fonts",
            });
        }
    }
    dirs.retain(|d| d.path.exists() || d.user_writable);
    dirs
}

/// The directory a per-user install should write to.
pub fn user_font_dir() -> Option<PathBuf> {
    system_font_dirs()
        .into_iter()
        .find(|d| d.user_writable)
        .map(|d| d.path)
}

// ----- helpers shared by the backends -----

fn io(path: &Path) -> impl FnOnce(std::io::Error) -> PlatformError + '_ {
    move |e| PlatformError::Io(path.to_path_buf(), e)
}

/// `file` must exist and be a regular file; returns its canonical path.
fn regular_file(file: &Path) -> Result<PathBuf> {
    let canonical = std::fs::canonicalize(file).map_err(io(file))?;
    if !canonical.is_file() {
        return Err(PlatformError::NotAFile(file.to_path_buf()));
    }
    Ok(canonical)
}

/// A name for `file` inside `dir` that does not collide with a different file of the
/// same basename: the basename itself when free (or already ours), otherwise the stem
/// plus a short hash of the source path. Used by the backend that links, where the link
/// target is proof of what fontina created; the backends that copy use [`copy_slot`].
#[cfg(all(unix, not(target_os = "macos")))]
fn slot_name(dir: &Path, file: &Path, is_ours: impl Fn(&Path) -> bool) -> PathBuf {
    let base = file
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_else(|| "font".into());
    let candidate = dir.join(&base);
    if !candidate.exists() && std::fs::symlink_metadata(&candidate).is_err() || is_ours(&candidate)
    {
        return candidate;
    }
    let hash = blake3::hash(file.to_string_lossy().as_bytes()).to_hex();
    let stem = file
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "font".into());
    let ext = file
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    dir.join(format!("{stem}-{}{ext}", &hash[..8]))
}

/// The name a *copied* font takes inside `dir`: always the stem, a short hash of the
/// source path, and the extension.
///
/// The backends that copy (macOS, Windows) cannot tell one file's bytes from another's,
/// so a plain basename would let `install` adopt a font the user had put in their own
/// font directory by hand, and `uninstall` would then delete it. A name derived from the
/// source path is proof that fontina wrote the file, and it is stable, so installing the
/// same font twice reuses the same slot instead of making a second copy.
#[cfg(any(target_os = "macos", target_os = "windows"))]
pub(crate) fn copy_slot(dir: &Path, file: &Path) -> PathBuf {
    let hash = blake3::hash(file.to_string_lossy().as_bytes()).to_hex();
    let stem = file
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "font".into());
    let ext = file
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    dir.join(format!("{stem}-{}{ext}", &hash[..8]))
}

/// Whether `path` has the shape [`copy_slot`] gives a file fontina copied. Guards
/// `uninstall` against deleting anything else, including a plain basename recorded by a
/// version of fontina that still adopted files.
#[cfg(any(target_os = "macos", target_os = "windows"))]
pub(crate) fn is_copy_slot(path: &Path) -> bool {
    path.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .and_then(|stem| {
            let (_, hash) = stem.rsplit_once('-')?;
            Some(hash.len() == 8 && hash.chars().all(|c| c.is_ascii_hexdigit()))
        })
        .unwrap_or(false)
}

/// An identical font already sitting under its own plain name in `dir`, which means the
/// user put it there themselves. Only the obvious candidate is checked: hashing every
/// file in a font directory to answer this would cost more than it is worth.
#[cfg(any(target_os = "macos", target_os = "windows"))]
pub(crate) fn present_by_hand(dir: &Path, file: &Path) -> Option<PathBuf> {
    let twin = dir.join(file.file_name()?);
    let same = std::fs::read(&twin).ok().map(|b| blake3::hash(&b))
        == std::fs::read(file).ok().map(|b| blake3::hash(&b));
    same.then_some(twin)
}

/// True when `path` is inside `dir` (after canonicalising `dir`).
fn is_under(path: &Path, dir: &Path) -> bool {
    let dir = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    let parent = path
        .parent()
        .map(|p| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf()));
    parent.is_some_and(|p| p.starts_with(&dir))
}
