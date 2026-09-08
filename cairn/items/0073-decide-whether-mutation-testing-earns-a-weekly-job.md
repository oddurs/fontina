---
id: 73
title: Decide whether mutation testing earns a weekly job
type: test
status: done
milestone: unfiled
created: 2026-09-08
updated: 2026-09-08
priority: p2
effort: m
crate: workspace
---

## Problem

Coverage says which lines ran. It does not say whether anything would have noticed if
they ran wrongly, and this repository has met that difference several times by hand:
the CoreText scope mutation that closed 0066, deleting the manual's mentions of a key to
prove the manual test could fail, seeding the collection fuzz corpus to prove the target
reached the index at all.

`cargo-mutants` does that automatically. Twenty minutes over one file found two real
things (#211): a test that only passed while the repository was not inside `$TMPDIR`, and
a test that pinned an ordering it did not pin — replacing the whole `rank` function with
`0` left all 498 tests green, because `min_by_key` keeps the first of equal keys and
`max_by_key` the last, and every case happened to be written in the order that made the
tie-break right.

Neither is the kind of thing coverage finds. `freedom.rs` was at 100% function coverage
when the second one was true.

## The cost, measured

    19 mutants over freedom.rs         3 minutes
    1,545 mutants in fontina-core      three to four hours, extrapolated
    the whole workspace                not measured, and much larger

So it is not a per-pull-request check under any configuration. The realistic shapes:

1. **Weekly, scoped to `fontina-core`**, beside the existing `fuzz` cron — same
   philosophy, same "a finding is a finding rather than a red build" posture, and the
   workflow already exists to copy.
2. **On demand only**, documented in `scripts/preflight`'s header as a tool to reach for
   when writing tests for something delicate.
3. **On changed files**, via `--in-diff`, which is cheap per pull request but only ever
   examines what somebody is already thinking about.

## The thing to decide

Whether the findings are worth the minutes, and whether a weekly list of missed mutants
gets triaged or becomes noise. A job producing a hundred findings nobody reads is worse
than no job — that is the same argument that dropped `head/checksum` from the integrity
milestone.

Option 3 is the cheapest way to find out and the easiest to withdraw.

## Acceptance criteria

- [x] a decision, with the reason written down, even if the decision is "not worth it"
- [x] if it runs anywhere, it is clear who reads the output and what they do with it

## Notes

Not wired up while passing through: it is a commitment of CI minutes rather than a
change to the code, and the numbers above are the point of this item rather than a
preamble to it.

## Closed

Option 1, weekly, scoped to `fontina-core`, in `.github/workflows/mutants.yml`.

Sharded four ways rather than time-boxed, which is the one thing this item got wrong.
`cargo mutants` has no total-time budget, and inventing one with an outer timeout would
have produced a different arbitrary prefix of the same list every week — the mutants it
never reached would be the same ones for ever. `--shard 1/4` through `4/4` finishes, and
each shard is about ninety minutes.

**It reports and never gates.** `|| true` on the run and `if: always()` on the summary:
a surviving mutant is a question about a test, not a broken build, and a red weekly job
nobody can act on is a job people mute. The missed list goes in the step summary and the
whole `mutants.out/` is kept as an artefact for thirty days.

`--in-place` is deliberately not used. mutants copies the tree to a scratch directory,
and a test that only passes in its original location is a finding worth having rather
than an inconvenience to configure away — that is exactly how the first of #211's two was
found.

Who reads it: nobody is assigned, and pretending otherwise would be the wrong answer to
the second criterion. What makes it survivable is that it is quiet by construction —
green every week, with a list in the summary — so it costs nothing to ignore for a month
and is there when somebody is writing tests for something delicate and wants to know
whether they hold.
