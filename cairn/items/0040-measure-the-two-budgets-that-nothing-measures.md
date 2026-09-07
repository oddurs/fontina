---
id: 40
title: Measure the two budgets that nothing measures
type: test
status: backlog
milestone: m5-ship
created: 2026-09-06
updated: 2026-09-06
priority: p1
effort: m
crate: cli
---

## Problem

Two rows of PLAN.md §7 say *not measured*: idle RSS of `fontina ui` at 5k faces, and TUI
repaint. `scripts/bench` prints them and shrugs, which is honest and is not a budget.

Both need a terminal. A number produced without one would be a number about nothing,
which is why they were left — but a budget nothing measures is an aspiration with a
table row, and §13 calls this the honest test of whether M5 is real.

## Proposal

A harness that can hold a pty — `portable-pty` as a dev-dependency is the cheap way in —
that starts `fontina ui` against a generated corpus, waits for the first paint, samples
resident size, then drives keystrokes and times the repaints.

Resident size is per-OS. §7 already states its budgets "for the machine they are enforced
on: a GitHub-hosted runner", so measure there and say that is what the number means
rather than pretending it is portable.

## Acceptance criteria

- [ ] `scripts/bench` reports both numbers instead of naming them and stopping
- [ ] both fail the run when they exceed the budget, like every other row
- [ ] PLAN.md §7 says *yes* in the Measured column, or the rows stop calling themselves budgets

## Notes

The alternative outcome is legitimate: if a pty harness proves more machinery than the
two numbers are worth, the honest move is to delete the rows rather than leave them
asserting something nothing checks.
