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

//! Follow directories and keep the index current. Events are coalesced for a short
//! quiet period, then only the touched paths are rescanned: a changed font file is
//! re-parsed on its own, a removed one is dropped, and a directory-level change (rename,
//! bulk copy) rescans that directory with pruning.

use crate::error::{Error, Result};
use crate::index::Index;
use crate::scan::{ScanOptions, ScanReport, is_font_candidate};
use notify::{RecursiveMode, Watcher};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct WatchOptions {
    /// Quiet period after the last event before the index is updated.
    pub debounce: Duration,
    pub follow_symlinks: bool,
}

impl Default for WatchOptions {
    fn default() -> Self {
        WatchOptions {
            debounce: Duration::from_millis(500),
            follow_symlinks: false,
        }
    }
}

/// One batch of changes applied to the index.
#[derive(Debug, Default, serde::Serialize, schemars::JsonSchema)]
pub struct WatchEvent {
    /// Files re-parsed or dropped, plus directories rescanned.
    pub paths: Vec<String>,
    pub report: ScanReport,
}

/// Keep `index` current for `roots` until `on_change` returns `false`. Blocks the
/// calling thread. Events arriving while a batch is applied are handled in the next
/// batch, so nothing is lost.
pub fn watch(
    index: &mut Index,
    roots: &[PathBuf],
    opts: &WatchOptions,
    mut on_change: impl FnMut(&WatchEvent) -> bool,
) -> Result<()> {
    if roots.is_empty() {
        return Err(Error::Other("nothing to watch".into()));
    }
    let roots: Vec<PathBuf> = roots
        .iter()
        .map(|r| std::fs::canonicalize(r).unwrap_or_else(|_| r.clone()))
        .collect();
    let (tx, rx) = std::sync::mpsc::channel::<notify::Result<notify::Event>>();
    let mut watcher = notify::recommended_watcher(move |res| {
        let _ = tx.send(res);
    })
    .map_err(|e| Error::Other(format!("cannot start file watcher: {e}")))?;
    for root in &roots {
        watcher
            .watch(root, RecursiveMode::Recursive)
            .map_err(|e| Error::Other(format!("cannot watch {}: {e}", root.display())))?;
    }

    let mut pending: BTreeSet<PathBuf> = BTreeSet::new();
    let mut quiet_since: Option<Instant> = None;
    loop {
        let wait = match quiet_since {
            Some(t) => opts.debounce.saturating_sub(t.elapsed()),
            None => Duration::from_secs(3600),
        };
        match rx.recv_timeout(wait) {
            Ok(Ok(event)) => {
                if matches!(event.kind, notify::EventKind::Access(_)) {
                    continue;
                }
                for p in event.paths {
                    pending.insert(p);
                }
                quiet_since = Some(Instant::now());
            }
            Ok(Err(e)) => return Err(Error::Other(format!("file watcher error: {e}"))),
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                if pending.is_empty() {
                    quiet_since = None;
                    continue;
                }
                let batch = std::mem::take(&mut pending);
                quiet_since = None;
                let event = apply(index, &roots, opts, batch)?;
                if !event.paths.is_empty() && !on_change(&event) {
                    return Ok(());
                }
            }
        }
    }
}

/// Bring the index up to date for a set of touched paths.
pub fn apply(
    index: &mut Index,
    roots: &[PathBuf],
    opts: &WatchOptions,
    touched: BTreeSet<PathBuf>,
) -> Result<WatchEvent> {
    let mut files: Vec<PathBuf> = Vec::new();
    let mut dirs: BTreeSet<PathBuf> = BTreeSet::new();
    let mut event = WatchEvent::default();
    for p in touched {
        if p.is_dir() {
            dirs.insert(p);
        } else if is_font_candidate(&p) {
            files.push(p);
        } else if !p.exists() {
            // A path that went away. It cannot be stat'd any more, and its name proves
            // nothing: `Fonts.old` is a directory with an extension, `LICENSE` a file
            // without one. Ask the index instead — rows at or under the path mean it
            // was a directory of fonts — and fall back to the name only when the index
            // knew nothing about it. Drop what was under it and rescan the root that
            // held it, so a rename's new name is picked up.
            let key = p.to_string_lossy().into_owned();
            let removed = index.remove_under(&key)?;
            if removed == 0 && p.extension().is_some() {
                continue;
            }
            event.report.removed += removed;
            if let Some(root) = roots.iter().find(|r| p.starts_with(r)) {
                dirs.insert(root.clone());
            }
            event.paths.push(key);
        }
    }
    // A file inside a directory being rescanned is covered by that rescan.
    files.retain(|f| !dirs.iter().any(|d| f.starts_with(d)));
    let (present, gone): (Vec<PathBuf>, Vec<PathBuf>) = files.into_iter().partition(|f| f.exists());
    for g in &gone {
        if index.remove_file(&g.to_string_lossy())? {
            event.report.removed += 1;
        }
        event.paths.push(g.to_string_lossy().into_owned());
    }
    if !present.is_empty() {
        let r = crate::scan::scan(
            index,
            &present,
            &ScanOptions {
                follow_symlinks: opts.follow_symlinks,
                ..Default::default()
            },
        )?;
        merge(&mut event.report, r);
        event
            .paths
            .extend(present.iter().map(|p| p.to_string_lossy().into_owned()));
    }
    for d in &dirs {
        let r = crate::scan::scan(
            index,
            std::slice::from_ref(d),
            &ScanOptions {
                prune: true,
                follow_symlinks: opts.follow_symlinks,
                ..Default::default()
            },
        )?;
        merge(&mut event.report, r);
        event.paths.push(d.to_string_lossy().into_owned());
    }
    Ok(event)
}

fn merge(into: &mut ScanReport, r: ScanReport) {
    into.candidates += r.candidates;
    into.parsed += r.parsed;
    into.faces += r.faces;
    into.unchanged += r.unchanged;
    into.removed += r.removed;
    into.failed.extend(r.failed);
}

/// Whether `path` lies under any of `roots`.
pub fn is_under_any(path: &Path, roots: &[PathBuf]) -> bool {
    roots.iter().any(|r| path.starts_with(r))
}
