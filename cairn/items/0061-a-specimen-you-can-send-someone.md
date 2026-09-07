---
id: 61
title: A specimen you can send someone
type: feat
status: backlog
milestone: specimen
created: 2026-09-07
updated: 2026-09-07
priority: p2
effort: s
crate: core
---

## Problem

A reader sets six axis sliders, ticks three features, picks a size and a paper, and has
arrived at the thing they wanted to show somebody. There is no way to send it, and no way
to get back to it — reloading the file throws all of it away.

For an artefact whose whole purpose is to be looked at and passed around, that is the
gap that most reliably turns a good specimen into a screenshot of one.

## Proposal

Keep the state in the URL fragment: axis coordinates, active features, size, leading,
measure, paper and ink, sample text, and which blocks are open. Read it on load, write it
on change.

The fragment is the right home for it. It needs no server, survives `file://`, costs
nothing when unused, and makes the specimen shareable by the mechanism people already
reach for, which is copying the address.

Add a control that says what it does — copy a link to this setting — rather than leaving
the reader to notice the address changed.

## Acceptance criteria

- [ ] Every reader-set value round-trips through the fragment.
- [ ] Opening a shared fragment reproduces the sheet exactly.
- [ ] A specimen opened with no fragment behaves as it does today.
- [ ] The fragment stays legible enough to hand-edit.
