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

//! Hand a local file to whatever the user has chosen to open it with.
//!
//! This is the graphical escape hatch and the whole of it: a terminal cannot show a
//! typeface at the fidelity that choosing one needs, and a browser can. Rather than build
//! a second interface to close that gap, fontina writes the specimen it already knows how
//! to write and asks the desktop to open it.
//!
//! The user's choice comes first. `$BROWSER` is the freedesktop convention for "this is
//! the program I want", and it is honoured before the desktop's default.

use crate::{PlatformError, Result};
use std::path::Path;
use std::process::Command;

/// What was run, so a caller can say so instead of leaving the user guessing why nothing
/// appeared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Opened {
    /// The program that was launched.
    pub with: String,
}

/// Open `path` with the user's chosen handler.
///
/// Returns as soon as the handler has been started; it is not waited for, because a
/// browser runs for hours and the caller is a terminal UI that has to keep drawing.
pub fn file(path: &Path) -> Result<Opened> {
    if !path.exists() {
        return Err(PlatformError::NotAFile(path.to_path_buf()));
    }
    for (program, args) in handlers() {
        let mut cmd = Command::new(&program);
        cmd.args(&args).arg(path);
        // A handler that writes to the terminal would draw over a running TUI.
        cmd.stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .stdin(std::process::Stdio::null());
        match cmd.spawn() {
            Ok(_) => return Ok(Opened { with: program }),
            // Not installed: try the next one. Anything else is this handler failing,
            // which is worth reporting rather than papering over.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(PlatformError::Io(path.to_path_buf(), e)),
        }
    }
    Err(PlatformError::Os(format!(
        "nothing here knows how to open {}: set $BROWSER, or open it yourself",
        path.display()
    )))
}

/// Programs to try, in order. The user's `$BROWSER` first where the convention exists.
fn handlers() -> Vec<(String, Vec<String>)> {
    let mut out: Vec<(String, Vec<String>)> = Vec::new();
    #[cfg(not(target_os = "windows"))]
    if let Ok(browser) = std::env::var("BROWSER") {
        // `$BROWSER` is a colon-separated list of commands, each of which may carry
        // arguments. `%s` is where the file goes; a handler that does not say gets it
        // appended, which is what every one of these does anyway.
        for entry in browser.split(':').filter(|s| !s.trim().is_empty()) {
            let mut words = entry.split_whitespace().map(str::to_string);
            if let Some(program) = words.next() {
                out.push((
                    program,
                    words.filter(|w| w != "%s" && w != "%u").collect::<Vec<_>>(),
                ));
            }
        }
    }
    #[cfg(target_os = "macos")]
    out.push(("open".into(), Vec::new()));
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        out.push(("xdg-open".into(), Vec::new()));
        // A desktop that is running but has no xdg-utils installed.
        out.push(("gio".into(), vec!["open".into()]));
    }
    #[cfg(target_os = "windows")]
    {
        // `start` is a shell builtin, not a program, and the empty string is the window
        // title `start` would otherwise take the path for.
        out.push((
            "cmd".into(),
            vec!["/C".into(), "start".into(), String::new()],
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn there_is_always_something_to_try() {
        assert!(!handlers().is_empty());
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn the_users_browser_is_tried_first_and_its_arguments_kept() {
        // SAFETY: single-threaded test, and the variable is read back immediately.
        unsafe { std::env::set_var("BROWSER", "my-viewer --new-window %s: fallback-viewer") };
        let h = handlers();
        unsafe { std::env::remove_var("BROWSER") };
        assert_eq!(h[0].0, "my-viewer");
        assert_eq!(h[0].1, vec!["--new-window".to_string()], "%s is dropped");
        assert_eq!(h[1].0, "fallback-viewer", "the list is colon-separated");
        assert!(
            h.len() > 2,
            "the desktop's own handler is still there behind them"
        );
    }

    #[test]
    fn a_file_that_is_not_there_is_not_handed_to_anything() {
        let missing = std::env::temp_dir().join("fontina-no-such-specimen.html");
        let _ = std::fs::remove_file(&missing);
        assert!(matches!(file(&missing), Err(PlatformError::NotAFile(_))));
    }
}
