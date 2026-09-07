---
id: 35
title: A monospaced font whose advances are not all equal
type: feat
status: done
milestone: integrity
created: 2026-09-06
updated: 2026-09-07
priority: p1
effort: s
crate: core
---

## Problem

`post.isFixedPitch` is a claim. Nothing checks it against the advance widths, and §12
already named the check that would: `metrics/fixed-pitch`.

M4 made spacing filterable on the strength of that flag alone — `--mono`, the `spacing`
facet, the `M` column — and was careful to say so: *report what `post.isFixedPitch` says
and nothing more; a font whose advance widths contradict its own flag is a health check,
not a filter that quietly disagrees with the file.* This is that health check, and until
it exists the filter has no way to tell you it was lied to.

## Proposal

At parse time, measure the advances skrifa already exposes and store whether they agree
with the flag. `check_face` takes only `FaceMetadata`, so the fact has to be in the model
— which is also where it belongs: a value the parser knows and the index cannot be asked
about is the exact mistake M4 spent nine pull requests undoing.

`metrics/fixed-pitch` then reads the stored fact. Warn, not error: a font may set the flag
for a subset that genuinely varies, and the finding is information for a person, not a
verdict.

Blank glyphs are excluded from the comparison — a monospaced font with a zero-width
combining mark is normal and is not a contradiction.

## Acceptance criteria

- [x] the model carries whether the advances are uniform (#174)
- [x] `metrics/fixed-pitch` warns when the flag and the advances disagree, in either
      direction (#174)
- [x] a fixture-backed test, and one over a mutated fixture for the direction no fixture
      can honestly show (#174)
- [x] `schemas/face.json` regenerated; `SCHEMA_VERSION` does not move, the field is additive

## Closed

Shipped in #174. Verified against `main` rather than against the pull request:
`Metrics::distinct_advances` is in `model.rs`, the check fires in both directions in
`check.rs`, and `schemas/face.json` carries the field while `SCHEMA_VERSION` stays at 1 —
#174 does not touch `lib.rs`, where the constant lives.

Both tests are in `tests/checks.rs`. The warn direction runs the whole path from bytes on
disk: `post.isFixedPitch` is a uint32 at offset 12, so four bytes of surgery on Amiri
makes a font that claims to be monospaced over its own real, unequal advances. The info
direction edits the parsed measurement instead, because rewriting `hmtx` so every advance
matched would be more fragile than the thing it tests and would prove nothing the first
case does not.

The headline finding when it ran over 932 system faces: Hack Nerd Font, 11,970 glyphs, all
one advance, `isFixedPitch` unset.
