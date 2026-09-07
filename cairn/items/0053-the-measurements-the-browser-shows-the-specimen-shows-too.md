---
id: 53
title: The measurements the browser shows, the specimen shows too
type: feat
status: ready
milestone: specimen
created: 2026-09-07
updated: 2026-09-07
priority: p0
effort: s
crate: core
---

## Problem

ADR 0010 took the preview pane out of the browser and put the measurements in its place —
units per em, ascender, descender, line gap, cap height, x-height, the x-height ratio —
with a stated reason: *"Those are comparable between two faces at a glance."*

The specimen shows none of them. Its rail carries the PostScript name, version, designer,
vendor, licence, a glyph count, scripts and a file name, and stops. So the view that was
built to compare faces omits precisely the numbers the other view added in order to
compare faces, and the two disagree about what matters about a font.

CLAUDE.md names this failure directly: a judgement has to be the same in every client or
two views of one font disagree.

## Proposal

Put the metrics in the rail, in the same order and with the same derived values the
browser uses, read from `Metrics` — which already carries every one of them.

The x-height ratio is the one that earns its place hardest: it is the number that says
whether two faces will look the same size, and it is what the comparison should key on.

Show `is_fixed_pitch` and `distinct_advances` alongside, since the pair is the whole
content of the `metrics/fixed-pitch` check and a monospace claim is worth reading next to
its evidence.

## Acceptance criteria

- [ ] The rail carries upem, ascender, descender, line gap, cap height, x-height and the
      x-height ratio.
- [ ] The values and their derivations match what the browser's details pane prints for
      the same face.
- [ ] A metric the font does not report reads as unknown, never as zero.

## Notes

Small, and it is the cheapest way to stop the two views drifting further apart.
