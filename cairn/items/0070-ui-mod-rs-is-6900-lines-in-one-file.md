---
id: 70
title: ui/mod.rs is 6900 lines in one file
type: chore
status: backlog
milestone: unfiled
created: 2026-09-07
updated: 2026-09-07
priority: p3
effort: l
crate: ui
---

## Problem

`crates/fontina-cli/src/ui/mod.rs` is 6,900 lines. The whole `ui/` tree is 11,838, so one
file is more than half of it, and it is the part of the program changing fastest.

This is not a test problem. The tests there are among the best in the repository: ninety
inline cases, ten snapshots, and property-style invariants that hold under an arbitrary
run of keys (`holding_any_single_key_down_keeps_every_invariant`,
`a_long_run_of_arbitrary_keys_keeps_every_invariant`). It is a review problem. Six
thousand nine hundred lines is past what a reviewer can hold, and "each PR is one logical
change with the smallest diff that does it" is hard to honour in a file where everything
is adjacent to everything.

## Proposal

Lift the parts that already look like siblings of `glyphs.rs` and `search.rs` — the facet
panel, the filter bar, the detail-summary cache — into their own modules. No behaviour
change; the invariant tests are the safety net that makes this the kind of refactor worth
doing.

## Acceptance criteria

- [ ] no single file in `ui/` is over about 2,000 lines
- [ ] every existing test passes unchanged
- [ ] no snapshot moves

## Notes

p3 and deliberately not scheduled: this is best done when TUI churn settles, and doing it
mid-milestone would conflict with every open branch in `tui-craft`. Filed so it is on the
board rather than rediscovered.
