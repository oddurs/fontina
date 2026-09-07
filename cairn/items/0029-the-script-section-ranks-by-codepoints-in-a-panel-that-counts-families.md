---
id: 29
title: The script section ranks by codepoints, in a panel that counts families
type: fix
status: backlog
milestone: tui-craft
created: 2026-09-06
updated: 2026-09-06
priority: p1
effort: s
crate: core
---

## Problem

`Index::facets` orders the script facet by `SUM(fs.codepoints)` — how much of each
script is there, deepest first. That was the right answer when the facet was a ranking
of coverage: a handful of Latin codepoints in a Japanese face is not a Latin font.

It is the wrong answer for the Narrow-by panel, which shows a section's top three and
counts families. On the maintainer's 733-family library the script section opens on:

    Hani                 37 (88)
    Latn                525 (2607)
    Hang                 11 (40)

CJK sorts above Latin because a Han face carries thousands of codepoints, so the row a
reader is most likely to want is second and the two either side of it are eleven and
thirty-seven families. Every other section in the panel sorts by families, descending;
this one does not, and nothing on the screen says why.

## Proposal

Order the script facet by families, then faces, then depth as the tie-break — the same
order `counts_by_count` gives every other facet. Depth is still what `--script-min`
filters on and still what decides whether a face is counted under a script at all, so
nothing about "a handful of Latin codepoints" changes; only the row order does.

Keep the SUM in the query as the last tie-break, and say in the comment why the order
moved, so the next person does not move it back.

## Acceptance criteria

- [ ] the script facet is ordered families-descending, like every other facet
- [ ] a test on a fixture set where depth and family count disagree asserts the order
- [ ] `--script-min` and the coverage bars in the face pane are untouched
