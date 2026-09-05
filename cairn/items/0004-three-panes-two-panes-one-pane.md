---
id: 4
title: Three panes, two panes, one pane
type: feat
status: backlog
milestone: tui-craft
created: 2026-09-05
updated: 2026-09-05
priority: p1
effort: m
crate: ui
---

## Problem

The browser assumes room for facets, families and details. There is a snapshot at
eighty columns that proves every pane still appears, but appearing and being usable
are different claims: at eighty columns the details pane wraps a file path over four
lines.

## Proposal

Three breakpoints, chosen the way the web side chose its own: wide keeps three
panes; medium drops facets to a toggled overlay; narrow shows one pane at a time
with the others a keystroke away.

## Acceptance criteria

- [ ] usable at 60 columns, not merely drawn
- [ ] a snapshot test at each breakpoint
- [ ] no pane ever truncates a value without showing that it did
