---
id: 34
title: The browser looks assembled rather than designed
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

Everything in the browser is drawn correctly and nothing in it is drawn *together*.
Nine `Block::default()`s, each choosing its own borders, its own title style and its
own highlight. Text set flush against every border. Square corners at every junction of
four rules. Three bar-like things — the coverage bars, the axis sliders, the feature
checkboxes — in three different visual languages. A seven-hundred-row list with no
indication of where in it you are. And the three rows of chrome at the top and bottom
start one column to the left of every pane's text, so nothing on the screen shares a
left edge.

Two of the colours are also not the reader's. `cursor()` sets a literal black on cyan,
which on a light theme is the one cell on the screen that looks like a mistake, and the
list highlight is the same hard reverse whether or not the pane it is in has the focus
— two reversed bars on one screen being two answers to "where am I".

## Proposal

One `pane_block`: rounded, padded by a column, dim until the pane has the focus, its
name on the top edge in bold and what to press on the bottom edge, dim. Every pane and
every overlay through it.

One bar language, in eighths, with a dim track: what a script covers, and — as a knob
on a track rather than a fill, because an axis is a position and not a quantity — where
an axis is set. A feature is `◉`/`○` rather than `[x]`/`[ ]`.

A scrollbar down the right edge of the list and the panel, drawn over the border so it
costs a pane nothing, and only when there is more than a screenful.

Colour that is the terminal's: `cursor()` reverses the *foreground* so the cell takes
the accent as its background and the reader's own background as its text, and the list
highlight reverses only in the pane with the focus and merely bolds in the others.

And two details the browser was sending people to `fontina info` for — the PostScript
name and the version — placed under the same height budget as everything else optional.

## Acceptance criteria

- [x] one block builder, and no pane building its own
- [x] no text touches a border
- [x] the coverage bars, the axis sliders and the feature toggles read as one family
- [x] a list longer than its pane says where in it you are
- [x] nothing sets a colour the reader did not choose
- [x] the filter line, the status line and the key hints share a left edge with the
      panes' text
