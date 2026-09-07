---
id: 37
title: Two files claiming to be the same font, with different bytes
type: feat
status: done
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

- [x] a PostScript-name group whose members differ in identity hash says so (#188)
- [x] a group whose members are byte-identical across containers keeps the wording it has (#188)
- [x] a fixture-backed test for both shapes (#188)

## Notes

The Inter WOFF and WOFF2 fixtures are the benign case — same font, two containers, one
identity hash. The other shape needs a mutated fixture.

## Closed

Shipped in #188. `DuplicateGroup.distinct` counts the identity hashes in a group, and
`dupes` prints it:

    same PostScript name (Inter-Regular); 2 distinct builds under it

Builds, not fonts. The first wording said "2 distinct fonts", which is alarming nonsense
for the Inter pair — one typeface subset two ways, 515 glyphs against 518.

A count, not a verdict: fontina has no reference for which build was meant, and fetching
one would need the network. What it does is stop a name collision reading as automatically
benign, which is the honest half of "can we identify compromised fonts".

Worth recording how this nearly did not land. The commit was written and pushed to
`feat/dupes-distinct`, and no pull request was ever opened for it. The item said `in
review` and nothing was reviewing it; it surfaced only because closing this item meant
looking for `distinct` on `main` and not finding it. A branch with no pull request is
invisible to every check this repository has.
