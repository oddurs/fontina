---
id: 27
title: Parse the OS/2 family class and PANOSE into the face model
type: feat
status: backlog
milestone: unfiled
created: 2026-09-06
updated: 2026-09-06
priority: p2
---

## Problem

The browser can rank faces for pairing (0016) on weight, width, spacing and
x-height, but not on the one distinction a designer would name first: serif
against sans. `FaceMetadata` stores no classification. There is no PANOSE and no
`OS/2.sFamilyClass`, so the pairing view leaves that axis out and says so in the
manual rather than dressing a `glyf`-versus-`CFF` difference up as a typographic
one.

## Proposal

Parse `OS/2.sFamilyClass` (class and subclass) and the ten PANOSE bytes into
`Os2Info`, and expose the family class as something usable — at minimum "serif",
"sans", "script", "decorative", "monospace", "symbol" — derived on read the way
`freedom` is, never stored as a verdict.

Both are already in the table the parser reads; nothing new is opened.

## Acceptance criteria

- [ ] `sFamilyClass` and PANOSE are in `Os2Info`, with a fixture-backed test each
- [ ] a derived classification, computed on read, with the mapping written down
- [ ] `SCHEMA_VERSION` bumped and `schemas/face.json` regenerated
- [ ] fonts that fill neither field are reported as unclassified, not as "sans"
- [ ] 0016's ranking gains the axis, and the manual stops saying it cannot
