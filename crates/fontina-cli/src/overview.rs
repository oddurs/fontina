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

//! What `fontina` says when you type its name and nothing else.
//!
//! It used to say this:
//!
//! ```text
//! error: 'fontina' requires a subcommand but one was not provided
//!   [subcommands: scan, list, families, facets, tag, collection, source, activate, ...]
//! ```
//!
//! The word *error*, then thirty-four nouns. The person has done nothing wrong — they
//! typed the name of the program they just installed — and every part of that message is
//! a reasonable default from the argument parser. Together they are a product with no
//! opening move.
//!
//! # What it says instead
//!
//! Whatever is true. With no index, that fonts have not been indexed yet, where this
//! machine keeps them, and the one command that starts. With an index, the shape of the
//! library in a line and the two or three things somebody actually does next.
//!
//! Exit 0 in both cases. Nothing here is an error until something has gone wrong.
//!
//! # What it does not do
//!
//! It does not scan. A program that walks your disk because you typed its name is a
//! program nobody trusts twice, and the rule this project already holds — *never run
//! what the person did not name* — is the whole difference between a suggestion and a
//! surprise. Every command it prints is one you can read before you run it.
//!
//! It is also not a second interface. Everything here is reachable by a command that
//! existed before this file did; the only thing being added is knowing which one to
//! suggest.

use crate::term;
use anyhow::Result;
use fontina_core::Index;
use std::fmt::Write as _;

/// The library, as a first screen needs it.
struct Shape {
    faces: i64,
    families: i64,
    free: i64,
    active: i64,
    failed: i64,
}

/// Print the opening screen for an index that may or may not exist.
///
/// `open` is handed in rather than opened here so the tests can drive both states
/// without a database on disk, and so the caller keeps the one place that knows how
/// `--db` and `FONTINA_DB` resolve.
pub fn show(open: impl FnOnce() -> Result<Index>) -> Result<()> {
    // An index that will not open is not the same as one that is empty, and this is the
    // one screen where the difference does not matter to the reader: either way they
    // have no fonts indexed and the next thing to do is the same. The error is not
    // swallowed — every other command still reports it — but greeting somebody with a
    // database error when they typed the program's name is the failure this file exists
    // to remove.
    let Ok(index) = open() else {
        return empty();
    };
    let stats = index.stats()?;
    if stats.faces == 0 {
        return empty();
    }

    let free = index
        .list(&fontina_core::FaceFilter {
            freedom: Some(fontina_core::Freedom::Free),
            ..Default::default()
        })
        .map(|faces| faces.len() as i64)
        .unwrap_or(0);

    full(&Shape {
        faces: stats.faces,
        families: stats.families,
        free,
        active: stats.activations,
        failed: stats.failed_files,
    })
}

/// No fonts indexed yet.
///
/// The directories come from `platform`, which is the same answer `fontina dirs` gives:
/// a person on a machine they did not set up should not have to know where their
/// operating system keeps fonts in order to index them.
fn empty() -> Result<()> {
    let t = term::term();
    println!("{}", t.head("No fonts indexed yet."));
    println!();

    let dirs = fontina_platform::system_font_dirs();
    if !dirs.is_empty() {
        println!("{}", t.dim("This machine keeps fonts in:"));
        for dir in dirs.iter().take(4) {
            println!("  {}", dir.path.display());
        }
        println!();
    }

    println!("{}", t.dim("To index them:"));
    suggest(
        "fontina scan --system",
        "the operating system's own directories",
    );
    suggest("fontina scan ~/Fonts", "a directory of your own");
    Ok(())
}

/// An index with something in it.
fn full(shape: &Shape) -> Result<()> {
    let t = term::term();

    // One line, in the order somebody reads it: how much, of what, how free, how much of
    // it is switched on.
    let mut line = String::new();
    let _ = write!(
        line,
        "{} in {}",
        t.head(&crate::n_of(shape.faces, "face", "faces")),
        crate::n_of(shape.families, "family", "families")
    );
    if shape.free > 0 {
        let _ = write!(
            line,
            " {} {} free",
            t.dim("·"),
            t.good(&shape.free.to_string())
        );
    }
    if shape.active > 0 {
        let _ = write!(
            line,
            " {} {} active",
            t.dim("·"),
            t.accent(&shape.active.to_string())
        );
    }
    println!("{line}");

    // A failure is the one number here that asks for something to be done, so it gets
    // its own line and the colour of a thing that went wrong.
    if shape.failed > 0 {
        println!(
            "{} {}",
            t.bad(&crate::n_of(shape.failed, "file", "files")),
            t.dim("could not be read — `fontina stats` says which")
        );
    }

    println!();
    println!("{}", t.dim("Next:"));
    suggest("fontina list", "every face, newest scan first");
    suggest("fontina families", "grouped by family");
    suggest("fontina ui", "the browser");
    Ok(())
}

/// One suggestion: the command, then what it does.
///
/// The command is the accent and the gloss is dim, because the command is the part you
/// are meant to take away and the gloss is the part you read once.
fn suggest(command: &str, gloss: &str) {
    let t = term::term();
    println!("  {:<24} {}", t.accent(command).to_string(), t.dim(gloss));
}
