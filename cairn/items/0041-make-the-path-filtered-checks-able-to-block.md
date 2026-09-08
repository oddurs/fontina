---
id: 41
title: Make the path-filtered checks able to block
type: chore
status: review
milestone: m5-ship
created: 2026-09-06
updated: 2026-09-07
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
- [x] the gates have reported on at least one pull request (#183)
- [ ] `perf`, `fuzz`, `linux`, `site`, `desktop` added to branch protection

## Notes

The order matters and is the same trap from the other direction: adding the names to
branch protection before the gates have reported points protection at a check that has
never run, and every pull request waits for ever. #112 first, one green run, then the
settings change.

GitHub issue #67 has the longer write-up.

The gates took two attempts. #112 added them and every one of the four workflows then
failed on `main` for a day, with no jobs and no logs — an invalid expression stops a run
before the first job, so the only thing GitHub says is "This run likely failed because of
a workflow file issue". The expression language takes single-quoted strings and nothing
else, and the separator passed to `join` was double-quoted: valid YAML, ordinary-looking
bash, wrong in the third language sharing the line. #183 fixed it, and #184 put
`actionlint` in `ci.yml` — which has no path filter, so it still runs when one of these
four cannot.

So the second criterion is met, and by the strongest evidence there is: all four gates
reported `pass` on #183 itself, and `perf`, `linux` and `site` are green on `main` at
`0dc19599` (`fuzz` has no `push` trigger, so it correctly does not run there).

There are five now, not four. #199 added `desktop.yml` — the acceptance test on macOS and
Windows, which nothing was running before — and it carries a gate job in the same shape as
the other four. It reported `pass` on both runners on the pull request that added it, so
it meets the same bar as the rest and belongs in the same settings change.

What is left is the settings change, which needs repository admin and is the maintainer's
to make. The order in the note above still holds and is now satisfied: gates first, green
runs observed, then protection.
