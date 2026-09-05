---
id: 13
title: Pin faces and set them side by side
type: feat
status: backlog
milestone: tui-discovery
created: 2026-09-05
updated: 2026-09-05
priority: p0
effort: m
crate: ui
---

## Problem

Choosing between two typefaces means looking at them together. The browser shows
one face at a time, so the comparison happens in memory, which is where comparisons
go wrong.

## Proposal

Pin up to four faces. A compare view sets all of them at the same size, in the same
text, one above another, with a single set of axis and feature controls applied to
each so what differs is the design rather than the settings.

## Acceptance criteria

- [ ] pinned faces survive filtering and searching
- [ ] one text, one size, one set of controls, applied to every pinned face
- [ ] the sheet already built for the waterfall is reused rather than reimplemented
- [ ] four faces at 48px stays inside the frame budget
