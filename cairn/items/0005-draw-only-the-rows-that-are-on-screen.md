---
id: 5
title: Draw only the rows that are on screen
type: perf
status: ready
milestone: tui-speed
created: 2026-09-05
updated: 2026-09-05
priority: p0
effort: m
crate: ui
---

## What is slow

Every pane builds a widget for every item it holds, whether or not the item is
visible. At the fixture scale that is six faces and free. On a real library it is
ten thousand, and the cost is paid on every keystroke because the frame is rebuilt
from scratch.

## Budget

PLAN.md §7 has no TUI frame budget yet; this item proposes one and the budget item
in this milestone enforces it. Target: a keystroke to a drawn frame in under 16ms
at 10,000 faces.

## Acceptance criteria

- [ ] each list builds rows for the visible window plus a small margin, no more
- [ ] scroll position and selection survive a filter change
- [ ] measured at 100, 1,000 and 10,000 faces, before and after, in this item
- [ ] `scripts/bench` still meets every other budget
