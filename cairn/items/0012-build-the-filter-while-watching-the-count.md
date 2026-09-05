---
id: 12
title: Build the filter while watching the count
type: feat
status: backlog
milestone: tui-depth
created: 2026-09-05
updated: 2026-09-05
priority: p1
effort: m
crate: ui
---

## Problem

Facets show what the library contains and let one value be chosen. Everything the
CLI can express — a weight range, two scripts at once, a licence and a coverage
threshold together — is unavailable, and the only way to see how many faces a
filter would leave is to run it.

## Proposal

A filter bar that composes `FaceFilter` interactively, showing the matching count
as each clause is added and offering the equivalent command line, so the browser
teaches the CLI rather than replacing it.

## Acceptance criteria

- [ ] every `FaceFilter` field is reachable, including the ranges M4 adds
- [ ] the count updates without blocking a frame (depends on the search worker)
- [ ] the composed filter is shown as a command that can be copied and run
- [ ] a filter can be saved as a collection in one keystroke
