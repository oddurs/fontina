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

//! Walk directories, parse fonts in parallel, and feed the index.

use crate::error::Result;
use crate::index::{Index, SourceKind};
use crate::model::Container;
use crate::{FaceMetadata, FileInfo};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use walkdir::WalkDir;

#[derive(Debug, Clone, Default)]
pub struct ScanOptions {
    /// Re-parse files even if size and mtime are unchanged.
    pub force: bool,
    pub follow_symlinks: bool,
    /// Remove index entries under the scanned roots whose files no longer exist.
    pub prune: bool,
    /// How scanned directories are recorded as sources.
    pub kind: Option<SourceKind>,
}

#[derive(Debug, Default, serde::Serialize, schemars::JsonSchema)]
pub struct ScanReport {
    pub candidates: usize,
    pub parsed: usize,
    pub faces: usize,
    pub unchanged: usize,
    pub removed: usize,
    pub failed: Vec<ScanFailure>,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub struct ScanFailure {
    pub path: String,
    pub error: String,
}

pub fn is_font_candidate(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .is_some_and(|e| Container::extensions().contains(&e.as_str()))
}

/// Collect candidate font files under `roots`.
pub fn collect_candidates(roots: &[PathBuf], follow_symlinks: bool) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for root in roots {
        if root.is_file() {
            if is_font_candidate(root) {
                out.push(root.clone());
            }
            continue;
        }
        for entry in WalkDir::new(root)
            .follow_links(follow_symlinks)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() && is_font_candidate(entry.path()) {
                out.push(entry.into_path());
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// One parsed file: its info and faces, or the error.
pub type ParsedFile = Result<(FileInfo, Vec<FaceMetadata>)>;

/// Parse one path, catching a panic in the parser as a last line of defence.
fn parse_one(path: &Path) -> (PathBuf, ParsedFile) {
    let result = std::panic::catch_unwind(|| crate::load_file(path)).unwrap_or_else(|_| {
        Err(crate::Error::Parse(
            "parser panicked on malformed input".into(),
        ))
    });
    (path.to_path_buf(), result)
}

/// Parse a set of paths in parallel. Does not touch the index.
///
/// One scoped thread per core, each claiming the next path with an atomic counter. The
/// work is handed out as threads finish rather than split up front because parse cost
/// varies by two orders of magnitude between a small icon font and a large CJK face,
/// and a static split would leave threads idle at the end of a scan. `scan` calls this
/// once per run, so the cost of starting the threads is paid once over the whole corpus.
///
/// Results are returned in input order, which the scan report depends on.
pub fn parse_paths(paths: &[PathBuf]) -> Vec<(PathBuf, ParsedFile)> {
    if paths.len() < 2 {
        return paths.iter().map(|p| parse_one(p)).collect();
    }

    let workers = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .min(paths.len());

    let next = AtomicUsize::new(0);
    let mut numbered: Vec<(usize, (PathBuf, ParsedFile))> = std::thread::scope(|scope| {
        let threads: Vec<_> = (0..workers)
            .map(|_| {
                scope.spawn(|| {
                    let mut mine = Vec::new();
                    loop {
                        let i = next.fetch_add(1, Ordering::Relaxed);
                        let Some(path) = paths.get(i) else { break };
                        mine.push((i, parse_one(path)));
                    }
                    mine
                })
            })
            .collect();
        threads
            .into_iter()
            // A worker only panics if something outside the parser did, and rayon
            // propagated such a panic to the caller too. Keep that behaviour.
            .flat_map(|t| t.join().unwrap_or_else(|e| std::panic::resume_unwind(e)))
            .collect()
    });

    numbered.sort_unstable_by_key(|(i, _)| *i);
    numbered.into_iter().map(|(_, parsed)| parsed).collect()
}

/// Scan `roots` into `index`.
pub fn scan(index: &mut Index, roots: &[PathBuf], opts: &ScanOptions) -> Result<ScanReport> {
    let roots: Vec<PathBuf> = roots
        .iter()
        .map(|r| std::fs::canonicalize(r).unwrap_or_else(|_| r.clone()))
        .collect();
    let candidates = collect_candidates(&roots, opts.follow_symlinks);
    let mut report = ScanReport {
        candidates: candidates.len(),
        ..Default::default()
    };

    let mut to_parse = Vec::new();
    for path in &candidates {
        if !opts.force
            && let Ok(meta) = std::fs::metadata(path)
        {
            let mtime = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            if index.file_is_unchanged(&path.to_string_lossy(), meta.len(), mtime)? {
                report.unchanged += 1;
                continue;
            }
        }
        to_parse.push(path.clone());
    }

    let results = parse_paths(&to_parse);
    let tx = index.begin()?;
    for (path, result) in results {
        match result {
            Ok((file, faces)) => {
                report.parsed += 1;
                report.faces += faces.len();
                Index::upsert_file_tx(&tx, &file, &faces)?;
            }
            Err(e) => {
                report.failed.push(ScanFailure {
                    path: path.to_string_lossy().into_owned(),
                    error: e.to_string(),
                });
                Index::record_failure_tx(&tx, &path.to_string_lossy(), &e.to_string())?;
            }
        }
    }
    tx.commit()?;

    if opts.prune {
        for root in &roots {
            report.removed += index.prune_missing(&root.to_string_lossy())?;
        }
    }
    // Directories become sources so `watch` and `facets` know about them.
    for root in roots.iter().filter(|r| r.is_dir()) {
        index.touch_source(
            &root.to_string_lossy(),
            opts.kind.unwrap_or(SourceKind::User),
        )?;
    }
    Ok(report)
}
