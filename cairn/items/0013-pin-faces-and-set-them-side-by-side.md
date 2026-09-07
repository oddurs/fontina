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

Pin up to four faces and put what is *in* them side by side, since 2026-09-06 the
browser draws no type at all ([ADR 0010](../../docs/adr/0010-the-browser-shows-structure.md)):
a column per face, a row per property. Weight and width, units per em, ascender,
descender, cap height, x-height and the x-height ratio, coverage per script, feature
tags present in one and missing from the other, licence and freedom.

That is the comparison a terminal is actually good at, and it is one no specimen makes
easy: a specimen shows two faces, and the reader still has to hold six numbers in their
head to say which is larger on the page. `x/em 0.43` beside `x/em 0.52` says it.

`s` on a pinned set still opens the HTML specimen with all of them in it, for the half
of the question that is about looking.

## Acceptance criteria

- [ ] pinned faces survive filtering and searching
- [ ] a column per face, a row per property, aligned, with differences visible at a
      glance rather than by reading
- [ ] a property that is the same across every pinned face is dimmed or dropped: the
      point of the view is the difference
- [ ] `s` opens one specimen containing the whole pinned set
