---
id: 23
title: Filtering to nothing removes the way back
type: fix
status: backlog
milestone: unfiled
created: 2026-09-06
updated: 2026-09-06
priority: p0
effort: s
crate: ui
---

## Problem

Narrow the library to nothing and the facet pane empties with it, taking the
controls that would undo the filter. On the maintainer's own library, weight
`300 Light` plus script `Arab`:

    ┌ 0 faces ───────────┐┌ 0 families ─────────┐┌ Details ──────────
    │                    ││                     ││Nothing selected.
    │                    ││                     ││
    └────────────────────┘└─────────────────────┘└───────────────────
     $ fontina families --weight 250-349 --script Arab

Three empty panes. The only evidence of what happened is the status line, and
the only way out is `x`, which the reader has to know. The facets are rebuilt
from the filtered set, so the row that says `● Arab` — the one you would press
Enter on to undo — is gone precisely when you need it.

## Proposal

A selected facet is always drawn, whatever the count, marked and toggleable. It
is the reader's own last action; it cannot be the thing that disappears.

When the result is empty, the list pane says so and says how to get back:
"nothing matches these filters — Enter on a marked row to drop it, x clears
them all". Silence and three empty boxes is the worst version of this.

## Acceptance criteria

- [ ] a facet value the reader selected is present in the pane at every count,
      including zero, and pressing Enter on it removes the filter
- [ ] an empty result names the way back, in the pane where the reader is
      looking
- [ ] a test drives a filter combination that yields nothing and asserts both
