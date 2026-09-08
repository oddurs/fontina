---
id: 68
title: Two fontina processes writing one index is reasoned about but untested
type: test
status: done
milestone: unfiled
created: 2026-09-07
updated: 2026-09-07
priority: p2
effort: s
crate: core
---

## Problem

Two `fontina` processes against one index is a thing that happens — a scan in one
terminal, the browser in another, `watch` running as an agent — and the index is one
SQLite file.

The code has clearly thought about it. `index/mod.rs:259-331` is thirty lines of comment
about why setting WAL waits for another connection doing the same, why SQLite answers
`SQLITE_BUSY` for that pragma immediately rather than consulting the busy handler, and
what happens on a filesystem that cannot do WAL at all. `schema.rs:435` covers the same
ground for migration.

What is tested is creation: `several_connections_can_create_the_same_index_at_once`.
Nothing tests two processes *writing*. The reasoning is careful and unverified, which is
the combination worth a test — a subtle argument nobody has checked is exactly what
rots.

## Proposal

A test that spawns two real `fontina` processes against one database and has both write:
two concurrent `scan` runs over overlapping directories, or a `scan` against a `tag`.
Both must finish, the index must be usable afterwards, and neither may report a lock
error to the user.

`tests/interruption.rs` already spawns the binary and kills it, so the harness for this
exists.

## Acceptance criteria

- [x] two processes writing one index concurrently both succeed (#202)
- [x] the index passes `PRAGMA integrity_check` afterwards (#202)
- [x] no `SQLITE_BUSY` reaches the user as an error (#202)

## Notes

Found while mapping durability on 2026-09-07.

## Closed

Shipped in #202 as `crates/fontina-cli/tests/concurrent.rs`, two tests: two scans of
different directories into one index, and a `tag` arriving while a long scan runs — the
shape of the browser tagging a family while an agent rescans.

A fourth property turned out to be worth asserting beyond the three above: that
everything **both** writers wrote is there. "No crash" is a low bar, and a writer that
quietly lost its rows to the other's transaction would have passed all three criteria as
written.

Checked that the test is not vacuous, because a concurrency test that never achieves
concurrency passes for the wrong reason:

    one scan alone:        507ms
    two scans together:    936ms
    two scans, if serial:  ~1014ms

Both processes are alive at once, and a wall time close to serial is SQLite serialising
their writes — the contention under test, not evidence against it. Five consecutive runs
on macOS and one in a Linux container, all green.
