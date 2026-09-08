---
id: 71
title: Twenty-two test files still build their own sandbox
type: chore
status: backlog
milestone: unfiled
created: 2026-09-08
updated: 2026-09-08
priority: p3
effort: m
crate: workspace
---

## Problem

`fontina-testkit` landed in #209 with two users. Twenty-two integration test files still
build their own sandbox: a temporary directory, the environment redirected at it, the
binary run against an index inside it, and an `impl Drop` to delete it.

Nothing is broken. `cargo test` runs them all and they all pass. What the duplication
costs is that a fix reaches one copy at a time — ten of them redirect `HOME` and only
five also redirect `LOCALAPPDATA`, which nobody decided and which the kit fixes for
everything that uses it.

## Proposal

A file at a time, whenever one is being edited anyway. Not a flag day: a pull request
that converts twenty-two test files at once conflicts with every branch in flight and is
unreviewable besides.

`colour.rs` is the one to watch out for and the reason this is filed rather than done: it
wants three helpers of its own to come along (`output` taking per-call environment,
`a_face`, `readable`), and converting it halfway is worse than leaving it. Whoever takes
it should decide whether those belong in the kit or stay in the file.

## Acceptance criteria

- [ ] no integration test file builds its own temporary directory
- [ ] `fn fixtures()` appears once in the tree rather than nineteen times
- [ ] every migrated file passes unchanged, without its assertions being touched

## Notes

The count when this was filed: 19 copies of `fixtures()`, 18 of the
`CARGO_BIN_EXE_fontina` wiring, 14 hand-rolled `impl Drop`, `remove_dir_all` in 24 files.

p3 because it is tidying with a long tail, and the thing that made it urgent — the
drift in what each copy redirects — stops getting worse the moment new tests use the kit.
