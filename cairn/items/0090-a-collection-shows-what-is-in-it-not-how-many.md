---
id: 90
title: A collection shows what is in it, not how many
type: feat
status: backlog
milestone: the-shelf
created: 2026-09-15
updated: 2026-09-15
priority: p1
effort: s
crate: cli
---

## Problem

`collection list` prints names and counts. A count is the least informative fact about a
set of typefaces: "Body text (7)" tells you nothing you wanted to know.

## Proposal

A collection shows its faces — a few names, the families, whether they are free, whether
they are active. Enough to recognise the set without opening it, which is the whole job
of a list of sets.

## Acceptance criteria

- [ ] `collection list` shows enough of each set to recognise it.
- [ ] It stays one screen for a reasonable number of collections.
- [ ] `--json` keeps the full membership, which it should already.
