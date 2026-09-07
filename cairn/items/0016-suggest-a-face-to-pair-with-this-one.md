---
id: 16
title: Suggest a face to pair with this one
type: feat
status: done
milestone: tui-discovery
created: 2026-09-05
updated: 2026-09-06
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

- [x] every suggestion shows the measurements it was ranked on
- [x] nothing is described as good, only as similar or contrasting in a named way
- [x] the ranking is derived from stored metadata; nothing new is parsed
- [x] a face with no plausible partner in the library says so

## What was left out, and why

"A different outline class" — serif against sans — is not derivable from what the
index stores. There is no PANOSE and no `OS/2.sFamilyClass` in `FaceMetadata`, and
the criterion above says nothing new is to be parsed, so the ranking leaves it out
and the manual says it does. Dressing a `glyf`-versus-`CFF` difference up as a
typographic one would have satisfied the letter of the proposal and lied to the
reader. The nearest thing the index does hold is the spacing class — monospace
against proportional — which is a real pairing signal, and it is measured.

Parsing `sFamilyClass` and PANOSE into `FaceMetadata` would make the original
criterion reachable. That is a core change with a schema bump behind it, so it
belongs in its own item rather than smuggled into this one.
