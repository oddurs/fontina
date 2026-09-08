---
id: 68
title: Two fontina processes writing one index is reasoned about but untested
type: test
status: backlog
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

- [ ] two processes writing one index concurrently both succeed
- [ ] the index passes `PRAGMA integrity_check` afterwards
- [ ] no `SQLITE_BUSY` reaches the user as an error

## Notes

Found while mapping durability on 2026-09-07.
