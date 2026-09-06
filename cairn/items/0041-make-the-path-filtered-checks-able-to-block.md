---
id: 41
title: Make the path-filtered checks able to block
type: chore
status: review
milestone: m5-ship
created: 2026-09-06
updated: 2026-09-06
priority: p1
effort: s
crate: workspace
---

## Problem

`perf`, `fuzz`, `linux` and `site` are green and none of them blocks anything: none can be
a required status check as it stands. All four are path-filtered, so a pull request that
touches none of their paths gets no run, and GitHub never marks an unreported required
check as passed — it waits. #38 changed one file and got exactly the eight checks already
required.

Both `budgets` and `fuzz` spent their first days red, and pull requests merged straight
over them. A check nobody can block on is a check everybody learns to ignore.

## Proposal

A gate job per workflow, named for the workflow, depending on every job in it, with
`if: always()` so it reports whatever those jobs did — including nothing. A skipped
dependency is a pass, because the guarded paths were untouched.

## Acceptance criteria

- [x] a gate job in each of the four workflows (#112)
- [ ] the gates have reported on at least one pull request
- [ ] `perf`, `fuzz`, `linux`, `site` added to branch protection

## Notes

The order matters and is the same trap from the other direction: adding the names to
branch protection before the gates have reported points protection at a check that has
never run, and every pull request waits for ever. #112 first, one green run, then the
settings change.

GitHub issue #67 has the longer write-up.
