---
id: 72
title: The five functions in watch.rs that nothing calls
type: test
status: backlog
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

- [ ] each of the five is named, with a decision: tested, or deliberately not
- [ ] `watch.rs` function coverage is above 85%, or the item says why the remainder stays

## Notes

`macos.rs` (70%) and `tags.rs` (76.67%) are lower still and are not this item: both are
platform code whose uncovered half is the other platform's, which is what
`#[cfg(target_os)]` does to a coverage report and not a thing to fix.
