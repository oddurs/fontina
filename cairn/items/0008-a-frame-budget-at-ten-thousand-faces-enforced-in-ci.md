---
id: 8
title: A frame budget at ten thousand faces, enforced in CI
type: test
status: backlog
milestone: tui-speed
created: 2026-09-05
updated: 2026-09-05
priority: p1
effort: m
crate: ui
---

## Problem

PLAN.md §7 holds budgets for scan, list, search and preview, and `scripts/bench`
fails the build when one is missed. The browser has none, which is why the three
performance items above have to propose their own.

## Proposal

Extend `scripts/bench` to drive the browser headlessly at 100, 1,000 and 10,000
faces: open, filter, scroll a page, open a family, and assert the worst frame in
each against a budget. Add the numbers to §7 so they are stated where every other
budget is stated.

## Acceptance criteria

- [ ] a budget per interaction, in PLAN.md §7, with the corpus size it holds at
- [ ] `scripts/bench` fails when one is missed, the way it does for scan and list
- [ ] runs in the existing `budgets` CI job, no new workflow
