---
id: 33
title: The face pane is thirteen rows in the order they were written
type: chore
status: done
milestone: tui-craft
created: 2026-09-07
updated: 2026-09-07
priority: p1
effort: m
crate: ui
---

## Problem

The face pane is thirteen labelled rows, then the axis controls, then three more rows,
in the order they were added over a year. Nothing in it is grouped, nothing is ranked,
and three of the rows say what a row below them already said.

On a real library, at eighty columns:

    ┌ Details ───────────────────────────────────────┐
    │.ADT Slab Numeric Light                         │
    │style     weight 300 · width 100% · normal      │
    │axes      wght 300–1000 (300), GRAD 0–1 (0)     │
    │glyphs    80 · 73 codepoints · Zyyy Arab Deva   │
    │Khmr Mymr                                       │
    │features  7: aalt calt cv04 locl ss03 dist kern │
    │license   LicenseRef-Proprietary                │
    │freedom   nonfree                               │
    │          the license withholds the freedom to  │
    │change or redistribute the font                 │
    │embedding Editable (reported, not enforced)     │
    │designer  Apple Inc.                            │
    │state     not active                            │
    │file      /System/Library/Fonts/ADTNumeric.ttc  │
    │#4                                              │
    └────────────────────────────────────────────────┘

- **Every wrapped value falls to column zero.** `Khmr Mymr`, `change or redistribute
  the font`, `#4`. The pane's one structural device is a label column you can run your
  eye down, and the wrap breaks it on exactly the rows that are long enough to matter.
  `Paragraph::wrap` cannot hang-indent, and nothing here wraps the values itself.
- **The pane is titled `Details`** and its first row is the name of the face. The
  border says nothing and costs a row saying it.
- **Three rows are duplicates.** `axes` repeats the axis controls drawn below it;
  the script list at the end of `glyphs` repeats the coverage bars below it; `license`
  and `freedom` are two rows for one answer.
- **No order.** Licence sits between coverage and metrics. `state` — the one line that
  changes when the reader presses a key — is eleventh. When the pane runs out of height
  it drops whatever happened to be last, which today is the metrics.

## Proposal

Group the pane by what a reader is asking, blank line between groups, and let the
bottom fall off in the order they can most afford to lose it:

1. what it is, and what you have done to it — style, state, tags
2. what you can move — the axis and feature controls
3. what it can set — glyphs, codepoints, coverage per script with a bar
4. what you may do with it — licence, verdict, embedding, reserved names
5. where it came from — designer, file
6. how it is drawn — metrics, shape

The measurements go last on the second thought: they are the numbers you go *looking*
for, comparing one face against another, and everything above them is a question you
arrive with. Where the file is and whether you may redistribute it are worth more of a
short terminal than the line gap.

And:

- **Wrap the values here**, hanging under the label column, so `Paragraph` is handed
  lines it will not touch. The row count then *is* the line count, which also retires
  the estimate the pane currently makes of its own height.
- **Title the pane with the face**, and drop the row that repeated it.
- Merge `license` and `freedom` into one row; drop the duplicated axes and script list.
- The list pane's title is a count the filter line already carries a row above it; it
  says what the list is instead.

## Acceptance criteria

- [x] no value in the pane wraps to column zero, at any width
- [x] nothing in the pane is said twice
- [x] the groups are in the order above, and a pane too short to hold them all keeps
      the earlier ones — the blanks go first, then the coverage beyond two scripts
- [x] the pane names the face it is showing
- [x] a test wraps every kind of value at every width from a label plus a word to two
      hundred, and the pane draws the longest of them under its label
