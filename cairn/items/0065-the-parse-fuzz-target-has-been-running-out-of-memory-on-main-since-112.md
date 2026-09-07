---
id: 65
title: 'The parse fuzz target has been running out of memory on main since #112'
type: fix
status: ready
milestone: unfiled
created: 2026-09-07
updated: 2026-09-07
priority: p1
effort: m
crate: core
---

## Problem

The `fuzz` workflow has failed on every run on `main` since 2026-09-07T16:05, starting at
`5927aa5b`. The last success was 2026-09-06. The `parse` target reports:

    stat::peak_rss_mb: 651
    Failing input: artifacts/parse/oom-f553a554e7d73f675bbb015f68c3ae46048d24c1

So `parse` is finding an input that drives allocation past libFuzzer's `rss_limit_mb`,
and the `fuzz` gate has stood red for every pull request since.

Found while checking whether #195 had broken anything: it had not — the target is
path-filtered onto `crates/`, so it only runs on pull requests that touch them, and the
failure predates that branch by hours. `fuzz` and `parse` are not required checks, so
nothing is being blocked from merging. A gate that is always red is also a gate that
stops being read, which is the real cost.

## Proposal

Get the failing input out of CI and into the repository. `fuzz/regressions/` already holds
five findings and `scripts/fuzz` replays them on stable, which is where an OOM belongs so
it cannot regress quietly.

Then find the allocation. The existing regressions name the shape to look for —
`woff1-origlength-allocation` and `woff1-table-count-overflow` are both a length read from
the file and trusted before anything checks it against the bytes actually present. The
core never panics on font input and should not be trusted to allocate on it either.

Report, do not resolve: whatever the file claims, the parser returns an `Err` for it rather
than trying to honour it.

## Acceptance criteria

- [ ] The OOM input is committed under `fuzz/regressions/` with a note on what it claims.
- [ ] `scripts/fuzz` replays it on stable and it does not OOM.
- [ ] The allocation is bounded by bytes present, not by a number the file states.
- [ ] The `fuzz` workflow is green on `main`.

## Notes

Run showing the failure: actions/runs/34153980519. Every run since `5927aa5b` failed;
`45796674` on 2026-09-06 was the last green one.
