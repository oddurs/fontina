---
id: 2
title: A specimen in the terminal that matches the HTML one
type: feat
status: backlog
milestone: tui-craft
created: 2026-09-05
updated: 2026-09-05
priority: p1
effort: l
crate: ui
---

## Problem

`fontina specimen` writes an HTML page with a waterfall, a paragraph, a glyph grid
and the axis controls. The browser shows a details pane of facts. The same font
gets two entirely different treatments depending on which one you opened, and the
richer one is the one you have to leave the tool to see.

## Proposal

A specimen view in the browser, opened from a face, reusing `typography` for every
judgement it makes so it cannot disagree with the HTML specimen about sizes,
sample text or which features are worth offering.

## Acceptance criteria

- [ ] waterfall at the sizes `typography::WATERFALL_SIZES` names
- [ ] a paragraph in the face's own sample text, or the script's, or the pangram
- [ ] the glyph grid already built for the glyph map, at specimen scale
- [ ] axis sliders and feature toggles apply to everything on screen at once
- [ ] the same font in `fontina specimen` and in this view makes the same choices
