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

//! An error that says what kind of thing is missing should say how to get one.
//!
//! `check`, `css` and `specimen` all need faces and all said `pass face ids or font file
//! paths`. That names the missing thing and not the way to supply it, which is the half
//! somebody stuck at the prompt needs: a person who has just run `fontina scan` has an
//! index full of faces and no idea what an id looks like.
//!
//! All three read targets from standard input, so the answer is one line, and it belongs
//! in the error rather than several pages into the manual.

use fontina_testkit::cli;

const NEEDS_FACES: [&str; 3] = ["check", "css", "specimen"];

#[test]
fn every_command_that_needs_faces_says_how_to_name_them() {
    let cli = cli!("no-targets");
    let fixtures = cli.fixtures();
    cli.ok(&["scan", &fixtures.to_string_lossy()]);

    for command in NEEDS_FACES {
        let err = cli.fails(&[command]);

        assert!(
            err.contains("pass face ids or font file paths"),
            "`{command}` lost the original line:\n{err}"
        );
        // The incantation, naming the command the person actually typed rather than a
        // generic one they would have to adapt.
        assert!(
            err.contains(&format!("fontina list --json | fontina {command} -")),
            "`{command}` does not say how to name every face:\n{err}"
        );
        assert!(
            err.contains(&format!("fontina {command} path/to/font.otf")),
            "`{command}` does not say a path works:\n{err}"
        );
    }
}

/// And the advice is true, which is the part worth checking rather than asserting.
///
/// A help message nobody ran is a help message that eventually stops working. This runs
/// the thing the error tells you to run.
#[test]
fn the_incantation_the_error_prints_actually_works() {
    let cli = cli!("no-targets-works");
    let fixtures = cli.fixtures();
    cli.ok(&["scan", &fixtures.to_string_lossy()]);

    let listed = cli.ok(&["list", "--json"]);

    for command in NEEDS_FACES {
        let mut child = cli
            .cmd(&[command, "-"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("fontina runs");

        use std::io::Write;
        child
            .stdin
            .take()
            .expect("stdin")
            .write_all(listed.as_bytes())
            .expect("writing the face list");

        let out = child.wait_with_output().expect("fontina finishes");
        assert!(
            out.status.success(),
            "`fontina list --json | fontina {command} -` failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}
