---
id: 5
title: Draw only the rows that are on screen
type: perf
status: done
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

## Measured

One frame of the browser at 120x36, best of 200, release build, on a developer's
laptop — so roughly two and a half times faster than the runner the budgets in
PLAN.md are stated for. `cargo test -p fontina-cli --bins -- --ignored --nocapture
what_a_frame_costs` reproduces it.

| Families | Before | After |
|---|---|---|
| 100 | 0.288 ms | 0.266 ms |
| 1,000 | 0.411 ms | 0.173 ms |
| 10,000 | 2.559 ms | 0.162 ms |

Linear before, flat after. The 10,000 figure is the one that matters: 2.56 ms on a
laptop is about 6.4 ms on the runner, which is 40% of the whole 16 ms repaint budget
spent on one pane before anything else in the frame is drawn.

## Acceptance criteria

- [ ] each list builds rows for the visible window plus a small margin, no more
- [ ] scroll position and selection survive a filter change
- [ ] measured at 100, 1,000 and 10,000 faces, before and after, in this item
- [ ] `scripts/bench` still meets every other budget
