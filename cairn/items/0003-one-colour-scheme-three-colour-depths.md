---
id: 3
title: One colour scheme, three colour depths
type: feat
status: done
milestone: tui-craft
assignee: Oddur Sigurdsson
created: 2026-09-05
updated: 2026-09-05
priority: p1
effort: m
crate: ui
---

## Problem

Colour in the browser accumulated a pane at a time. There is no palette written
down, no statement of what colour means, and no answer for a terminal with sixteen
colours or a person who set `NO_COLOR`.

## Proposal

A palette in one module, defined once in truecolor and degraded deliberately: 256
colours by nearest match, 16 by hand, none at all under `NO_COLOR` where every
distinction falls back to weight, brackets and position.

Colour carries hierarchy. It never carries meaning alone — the same rule the web
side already follows.

## Acceptance criteria

- [ ] one module owns every colour; no literal colour anywhere else in `ui/`
- [ ] `NO_COLOR=1` is honoured, and the browser stays fully usable
- [ ] `COLORTERM` absent falls back to 256, then to 16
- [ ] a snapshot test per depth, so a regression is visible in review
