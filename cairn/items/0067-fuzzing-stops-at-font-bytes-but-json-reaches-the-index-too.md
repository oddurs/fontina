---
id: 67
title: Fuzzing stops at font bytes, but JSON reaches the index too
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

`fuzz/fuzz_targets/` holds two targets, `parse` and `sfnt`, and both are fed font bytes.
Fonts are not the only untrusted input this program takes:

    main.rs:2547    serde_json::from_str(l)          one JSON object per line, from stdin
    main.rs:3042    serde_json::from_str(&text)      a collection file
    library.rs:652  import_collection                straight into the index

A collection file is something a person is invited to send someone else — the JSON
schema in `schemas/collection.json` exists so that they can. That makes it the one input
here most likely to arrive from a stranger, and the only one of the three that reaches
SQL.

## Proposal

A third target over `import_collection` and the stdin JSON path. It is the same shape as
the existing two — arbitrary bytes in, must return — and it reuses `scripts/fuzz` and
`fuzz/regressions/` unchanged.

## Acceptance criteria

- [ ] a `collection` fuzz target that drives `import_collection` from arbitrary bytes
- [ ] it is in the `fuzz` workflow's matrix alongside `parse` and `sfnt`
- [ ] any finding lands in `fuzz/regressions/` with a row in its README

## Notes

Lower than 0066 because this surface is not unguarded: `stdin.rs`, `collection_file.rs`
and `hostile_arguments.rs` all push malformed input through it by hand. What is missing
is mutation, not attention.

See [[0065-the-parse-fuzz-target-has-been-running-out-of-memory-on-main-since-112]] —
the fuzzing setup needs to be healthy before a third target is worth adding to it.
