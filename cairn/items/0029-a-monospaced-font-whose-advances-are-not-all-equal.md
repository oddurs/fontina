---
id: 29
title: A monospaced font whose advances are not all equal
type: feat
status: review
milestone: integrity
created: 2026-09-06
updated: 2026-09-06
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

- [ ] the model carries whether the advances are uniform
- [ ] `metrics/fixed-pitch` warns when the flag and the advances disagree, in either
      direction
- [ ] a fixture-backed test, and one over a mutated fixture for the direction no fixture
      can honestly show
- [ ] `schemas/face.json` regenerated; `SCHEMA_VERSION` does not move, the field is additive
