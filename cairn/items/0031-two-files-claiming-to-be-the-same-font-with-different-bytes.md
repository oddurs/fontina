---
id: 31
title: Two files claiming to be the same font, with different bytes
type: feat
status: review
milestone: integrity
created: 2026-09-06
updated: 2026-09-07
priority: p2
effort: s
crate: cli
---

## Problem

`dupes` already groups faces that share a PostScript name, and reports it as *installing
both would conflict*. That is true and it is not the only thing it means.

Two files claiming the same PostScript name with different outlines is also what a
substituted font looks like. The data is already indexed and the query already runs; only
the framing treats it as a packaging inconvenience rather than something worth a second
look.

## Proposal

Say both. Where a PostScript-name group holds faces whose identity hashes differ, report
that they are not the same font despite saying they are — alongside the conflict warning,
not instead of it.

No verdict. fontina cannot know which copy is right; it has no reference and cannot fetch
one. What it can do is stop the reader assuming a name collision is always benign.

## Acceptance criteria

- [ ] a PostScript-name group whose members differ in identity hash says so
- [ ] a group whose members are byte-identical across containers keeps the wording it has
- [ ] a fixture-backed test for both shapes

## Notes

The Inter WOFF and WOFF2 fixtures are the benign case — same font, two containers, one
identity hash. The other shape needs a mutated fixture.
