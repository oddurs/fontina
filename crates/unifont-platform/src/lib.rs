//! Platform integration: where fonts live on each OS, and the activation trait that the
//! macOS, Windows and Linux backends implement.
//!
//! M0 ships the directory model and system enumeration. Native activation backends
//! (CoreText, `AddFontResourceEx`, fontconfig) land in M1.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
    #[error("not supported on this platform yet: {0}")]
    Unsupported(&'static str),
    #[error("{0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, PlatformError>;

/// The activation backend contract. One implementation per OS.
pub trait FontActivator {
    /// Persistent, per-user install. Never touches system directories.
    fn install(&self, file: &std::path::Path) -> Result<PathBuf>;
    fn uninstall(&self, file: &std::path::Path) -> Result<()>;
    fn activate(&self, file: &std::path::Path, scope: Scope) -> Result<()>;
    fn deactivate(&self, file: &std::path::Path) -> Result<()>;
    /// Font directories the OS reads, in precedence order.
    fn font_dirs(&self) -> Vec<SystemFontDir>;
}

#[cfg(not(target_os = "windows"))]
fn home() -> Option<PathBuf> {
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
        let data_home = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| home().map(|h| h.join(".local/share")));
        if let Some(d) = data_home {
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
