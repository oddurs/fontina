---
id: 16
title: Suggest a face to pair with this one
type: feat
status: backlog
milestone: tui-discovery
created: 2026-09-05
updated: 2026-09-05
priority: p2
effort: l
crate: ui
---

## Problem

Pairing is the question after choosing, and it is the one thing here that no field
in the index answers directly.

## Proposal

Rank the library against a chosen face on the things pairing actually turns on:
contrast in weight and width, a different outline class, comparable x-height at a
common size, and scripts covered in common.

Worth being clear about what this is. It is not taste and it must not pretend to
be. It is a ranking, shown with the numbers behind it, that puts twenty plausible
candidates in front of someone instead of four hundred faces.

## Acceptance criteria

- [ ] every suggestion shows the measurements it was ranked on
- [ ] nothing is described as good, only as similar or contrasting in a named way
- [ ] the ranking is derived from stored metadata; nothing new is parsed
- [ ] a face with no plausible partner in the library says so
