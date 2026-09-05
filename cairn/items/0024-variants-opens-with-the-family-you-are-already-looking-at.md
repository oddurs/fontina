---
id: 24
title: Variants opens with the family you are already looking at
type: feat
status: backlog
milestone: unfiled
created: 2026-09-05
updated: 2026-09-05
priority: p2
effort: s
crate: cli
---

## Problem

`fontina variants <id>` answers "what else covers nearly the same characters",
and on a real library the first screen is the face's own family:

    faces overlapping Gotham Book (#626):
      622  Gotham Black          100.00%  637  same
      623  Gotham Black Italic   100.00%  637  same
      624  Gotham Bold           100.00%  637  same
      625  Gotham Bold Italic    100.00%  637  same
      627  Gotham Book Italic    100.00%  637  same
      628  Gotham Light          100.00%  637  same

Sixteen Gothams share one character set, so sixteen rows of 100% arrive before
anything a person did not already know about. The question `variants` is asked
is "what else could set this text" — the weights of the family you are looking
at are the one answer nobody needs.

It is not wrong, it is unsorted for the question. A designer with 341 families
runs this to find the Graphik that could stand in for the Gotham, and has to
page past the family to reach it.

## Proposal

Group the same family together and put it last, or drop it behind a flag.
`--same-family` to include it, off by default, is the smallest change that makes
the first screen answer the question. The overlap and the metrics columns stay
exactly as they are; this is ordering, not scoring.

Worth deciding at the same time: whether "same family" means the typographic
family string or the wider grouping `families` uses, because `Gotham` and
`Gotham Narrow` are different families by name and the same design by eye.

## Available already

`Index::related` does the work and is fast. `FaceSummary` carries the family, so
the grouping needs no new query. The browser's counterpart is 0014, which will
want the same answer to the same question; whichever lands first should settle
the rule and the other should follow it.
