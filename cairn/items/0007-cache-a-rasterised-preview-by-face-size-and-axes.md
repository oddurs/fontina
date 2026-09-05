---
id: 7
title: Cache a rasterised preview by face, size and axes
type: perf
status: done
milestone: tui-speed
created: 2026-09-05
updated: 2026-09-05
priority: p1
effort: m
crate: ui
---

## What is slow

A preview is rasterised every time it is drawn. Moving the selection down and back
up rasterises the same glyphs twice, and the family list in this milestone will ask
for one per visible row.

## Budget

Scrolling a list of previews rasterises each face once, not once per frame.

## Acceptance criteria

- [ ] key is the face, the pixel size and the axis coordinates, so a slider move is a miss and a scroll is a hit
- [ ] bounded, and evicts least-recently-used rather than growing without limit
- [ ] the bound is stated against the idle-memory budget in PLAN.md §7
- [ ] a test that the same request twice rasterises once
