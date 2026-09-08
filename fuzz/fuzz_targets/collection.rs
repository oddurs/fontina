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

//! A collection file, from arbitrary bytes, all the way into the index.
//!
//! Font bytes are not the only untrusted input this program takes. A collection is
//! written by `collection export`, described by `schemas/collection.json`, and exists so
//! that one person can send it to another — which makes it the input here most likely to
//! arrive from a stranger, and the only one of them that reaches SQL.
//!
//! The target is the whole path a `collection import` takes: `serde_json` into
//! `CollectionExport`, then `import_collection` against a real index. Deserialising alone
//! would be fuzzing serde; what is worth the budget is what the *values* do afterwards —
//! a name that is not a name, a face that matches nothing, a tag list longer than the
//! faces it belongs to, a `schema_version` from the future.
//!
//! It must return. A panic is a bug and so is a hang; `import_collection` returning `Err`
//! is a pass, because a collection that cannot be imported is an ordinary thing.
//!
//! # Why the index is built once
//!
//! `Index::open_in_memory` runs seven migrations, and doing that per iteration costs
//! about a tenth of a second — measured, not guessed: a fresh index every run gave 10
//! executions a second against a seeded corpus, so a minute of CI would be six hundred
//! runs and a target that cannot find anything is no better than no target. Built once
//! and reused, the budget goes where it is worth spending.
//!
//! What that trades away is isolation between iterations, and here that is a gain rather
//! than a cost: rows left by one import are state the next one meets, which is closer to
//! the library a real person imports into than an empty database is.
//!
//! It does have to be bounded, though, and measurement is why. Reused without limit the
//! target reached 652 MB after two million runs in a minute; `scripts/fuzz` passes
//! `-rss_limit_mb=2048`, and the weekly run is ten times as long, so it would have
//! crossed that and reported an out-of-memory finding of its own making. Rebuilding the
//! index every `REBUILD_EVERY` iterations costs one migration per ten thousand runs —
//! nothing — and holds the ceiling flat.

#![no_main]

use libfuzzer_sys::fuzz_target;
use std::cell::RefCell;

/// How many imports one index takes before it is thrown away. See the note above: this
/// is what keeps the target's own memory out of the fuzzer's out-of-memory report.
const REBUILD_EVERY: u64 = 10_000;

thread_local! {
    static INDEX: RefCell<Option<fontina_core::Index>> =
        const { RefCell::new(None) };
    static RUNS: RefCell<u64> = const { RefCell::new(0) };
}

fuzz_target!(|data: &[u8]| {
    let Ok(export) = serde_json::from_slice::<fontina_core::CollectionExport>(data) else {
        return; // not a collection; serde already said so, and that is not this target's bug
    };
    let stale = RUNS.with(|n| {
        let mut n = n.borrow_mut();
        *n += 1;
        *n % REBUILD_EVERY == 0
    });
    INDEX.with(|cell| {
        let mut slot = cell.borrow_mut();
        if stale {
            *slot = None;
        }
        if slot.is_none() {
            let Ok(index) = fontina_core::Index::open_in_memory() else {
                return; // nothing to test against
            };
            *slot = Some(index);
        }
        let index = slot.as_mut().expect("just filled");
        // Both directions of `apply_tags`: the tag path is the half that writes rows per
        // face rather than per collection, and it is where a hostile tag list would land.
        let _ = index.import_collection(&export, None, true);
        let _ = index.import_collection(&export, None, false);
    });
});
