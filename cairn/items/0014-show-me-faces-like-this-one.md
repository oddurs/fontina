---
id: 14
title: Show me faces like this one
type: feat
status: backlog
milestone: tui-discovery
created: 2026-09-05
updated: 2026-09-05
priority: p1
effort: m
crate: ui
---

## Problem

The library holds families that are nearly the same font: a patched build, a
re-encoding, a subset, an interpolation of the same design. Nothing surfaces that,
so a person scrolls past six rows that are one typeface.

A real library makes the point: 149 faces reporting twenty families and holding
about eight typefaces, because a patch spaced three ways names itself three times.

## Proposal

`Index::related` from M4 §12, in the browser: a key on a face lists what else
covers nearly the same characters, with the overlap and the metrics that say
whether "covers the same" means "is the same design".

## Available already

`Index::related` shipped with M4 (#102). This is that query with a view on it, which
is why it is a medium rather than a large.

## Acceptance criteria

- [ ] the score is shown, not thresholded away
- [ ] the four metrics that distinguish a variant from a coincidence are beside it
- [ ] a face with nothing near it says so rather than showing a weak list
