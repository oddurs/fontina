---
id: 52
title: Compare at a matched x-height, not a matched pixel size
type: feat
status: ready
milestone: specimen
created: 2026-09-07
updated: 2026-09-07
priority: p0
effort: m
crate: core
---

## Problem

The comparison block sets every face at the same `px` and calls that side by side. That is
the comparison that lies. Two faces at 48px with different x-heights do not look like the
same size — one reads two sizes larger — so the reader is asked to choose between three
faces on the strength of a difference nobody chose.

The project already knows this. `ui/pairing.rs` says it outright: *"x-height at a common
size is most of what 'the same size' means"*, and it ranks pairings on exactly that. The
specimen, which exists to compare, is the one view that ignores it.

## Proposal

Offer a normalisation control over the comparison: **match size** (what it does today) or
**match x-height**, with match x-height the default because it is the honest one.

Under match x-height, one face is the reference — the first, or whichever the reader picks
— and every other face is scaled by the ratio of its x-height to the reference's, both
taken from `Metrics::x_height` over `units_per_em`. The scale factor is shown next to each
name, so the reader can see what was done to the type rather than trusting it.

The arithmetic is `pairing.rs`'s and should be shared, not copied: an x-height ratio
computed two ways is two views of one font disagreeing, which is the thing `typography`
exists to prevent. If that means the ratio moves from `ui/pairing.rs` into `typography`,
that is the right move.

## Acceptance criteria

- [ ] The comparison offers match size and match x-height; match x-height is the default.
- [ ] The scale applied to each face is visible, not implicit.
- [ ] A face that reports no x-height is shown at nominal size and says so, rather than
      being scaled by a guess or treated as zero — `pairing.rs` already holds that line.
- [ ] The ratio comes from one shared function, not a second implementation.

## Notes

`Metrics::x_height` is `Option<i16>`; `metrics/x-height` is already a health check, so a
font with no x-height is a case the codebase has met before.
