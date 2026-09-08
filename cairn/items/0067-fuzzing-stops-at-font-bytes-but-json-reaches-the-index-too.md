---
id: 67
title: Fuzzing stops at font bytes, but JSON reaches the index too
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

- [x] a `collection` fuzz target that drives `import_collection` from arbitrary bytes (#201)
- [x] it is in the `fuzz` workflow's matrix alongside `parse` and `sfnt` (#201)
- [x] any finding lands in `fuzz/regressions/` with a row in its README — nothing found
      in 6.17M executions, so there is nothing to land

## Notes

Lower than 0066 because this surface is not unguarded: `stdin.rs`, `collection_file.rs`
and `hostile_arguments.rs` all push malformed input through it by hand. What is missing
is mutation, not attention.

See [[0065-the-parse-fuzz-target-has-been-running-out-of-memory-on-main-since-112]] —
the fuzzing setup needs to be healthy before a third target is worth adding to it.

## Closed

Shipped in #201. Three numbers decided the design, and each one said the previous version
was wrong.

**Seeding.** `seed()` copied fonts into every target's corpus, and a font is an input this
target can never parse. Unseeded it ran at 34,368 exec/sec — not a fast fuzzer, a fuzzer
bouncing off serde and never reaching the index. Seeded with one real export it dropped to
10/sec, and *that* number is the proof it reaches `import_collection`: a hundred
milliseconds an iteration, all of it opening an in-memory index and running seven
migrations. `seed()` is target-aware now, with the reasoning in a comment so the next
target added does not repeat it.

**Reuse.** Six hundred runs a CI minute cannot find anything. Building the index once took
it to 35,642/sec, and the isolation it gives up is a gain rather than a cost: rows left by
one import are state the next one meets, which is closer to a real library than an empty
database.

**A ceiling.** Reused without limit it reached 652 MB in a minute, and `scripts/fuzz`
passes `-rss_limit_mb=2048` with a ten-minute weekly run — it would have reported an
out-of-memory finding of its own making, which is the noise
[[0065-the-parse-fuzz-target-has-been-running-out-of-memory-on-main-since-112]] describes.
Rebuilding every 10,000 iterations holds it flat: 461 MB over 2.3M runs, 503 MB over 6.2M.

No crash in 6.17 million executions. The surface is covered and nothing came out of it,
which is the honest result rather than a disappointing one.
