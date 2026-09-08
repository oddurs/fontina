---
id: 72
title: The five functions in watch.rs that nothing calls
type: test
status: done
milestone: unfiled
created: 2026-09-08
updated: 2026-09-08
priority: p2
effort: s
crate: core
---

## Problem

Coverage was measured for the first time when the CI job was added. The workspace is at
**92.42% lines, 91.19% regions, 91.49% functions** across 28,670 regions, which is high
and matches what reading the tests suggested.

`watch.rs` is the weakest thing in `fontina-core` by function coverage, and by some
distance:

    68.75%  fontina-core/src/watch.rs      5 of 16 functions never called
    78.95%  fontina-core/src/container.rs  4 of 19
    83.87%  fontina-core/src/typography.rs 5 of 31

Lines are 86.78%, so this is not a file nobody tests — `library.rs` drives `watch::apply`
and `watch::watch` hard. It is that a third of its functions are reached by nothing, which
in a file whose job is to react to the filesystem changing underneath a running program is
worth knowing about specifically.

The two below it are less alarming and are noted so the next person does not have to
measure again: `container.rs`'s misses are the WOFF paths that need a fixture nobody has,
and `typography.rs` is mostly formatting helpers for shapes the fixtures do not contain.

## Proposal

Find out which five, and for each one decide whether it wants a test or wants deleting.
Not every uncovered function is a gap — a `Debug` impl or an error path only reachable
from a filesystem that has gone away is fine uncovered, and saying so in the item is worth
more than a test that exercises it artificially.

    cargo llvm-cov --workspace --features fontina-platform/platform-tests \
      --html --open

## Acceptance criteria

- [x] each of the five is named, with a decision: tested, or deliberately not
- [x] `watch.rs` function coverage is above 85%, or the item says why the remainder stays

## Notes

`macos.rs` (70%) and `tags.rs` (76.67%) are lower still and are not this item: both are
platform code whose uncovered half is the other platform's, which is what
`#[cfg(target_os)]` does to a coverage report and not a thing to fix.

## Closed

The title is wrong and the premise was half wrong, which is the useful part of this item.

**Four of the five were not functions.** `watch` is generic over its callback, and
llvm-cov counts each monomorphisation separately: the closures inside it at lines 68, 71,
74 and 78 appear once per instantiation, and the instantiation that is *not* covered is
the one the CLI's `watch` command creates. `watch` itself is tested hard — `library.rs`
runs a live filesystem watcher on a thread and joins it. So "five functions nothing
calls" was an artifact of how a coverage summary counts generic code, and reading the
report without checking the source would have produced four tests for something already
tested.

**One was real.** `is_under_any` is public, has no caller anywhere in the tree, and had
no test. It is now tested rather than deleted, because what it does is not what a
reimplementation would do: `Path::starts_with` compares whole components, so
`/fonts-backup` does not lie under `/fonts`, and a watcher that believed a string prefix
would index a directory nobody asked for.

**And one more, found the same way in another file.** `Freedom::is_free` has no caller
either, which is why `cargo mutants` could replace it with `true` and leave 498 tests
green (#211). Also now tested — the interesting half being that `Unknown` and `Unstated`
are both `false`: a licence nobody recognises is not free until somebody says what it is,
and a font that states nothing is not free by saying nothing.

Two dead public functions in a library crate is worth knowing as a pattern. Neither was
deleted: `fontina-core` is publishable and the desktop app is the plausible consumer. If
they are still uncalled at 1.0 they should go, and the tests make that a one-line
decision rather than an investigation.
