---
id: 27
title: Parse the OS/2 family class and PANOSE into the face model
type: feat
status: done
milestone: unfiled
created: 2026-09-06
updated: 2026-09-07
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

- [x] `sFamilyClass` and PANOSE are in `Os2Info`, with a fixture-backed test each
- [x] a derived classification, computed on read, with the mapping written down
- [x] `schemas/face.json` regenerated — see the note below on the version
- [x] fonts that fill neither field are reported as unclassified, not as "sans"
- [x] 0016's ranking gains the axis, and the manual stops saying it cannot

## On the schema version

Not bumped, and the criterion above was wrong to ask for it. `SCHEMA_VERSION` bumps
for a change that is *not* backwards-compatible; both new fields are optional and
defaulted, so an index written by an older build still parses. `schemas/face.json`
is regenerated, which is the part that matters.

## What the fixtures actually say

Worth writing down, because it is the measure of how much this axis is worth. Of the
five fonts this repository tests against, **not one** fills in `sFamilyClass`. Three
decline to classify themselves at all — including Amiri, a serif Arabic face, and
Inter, which fills in the PANOSE family kind and then leaves the serif style at
"any". The chromatic display face calls itself a normal sans.

So the classification is a report of what a font claims, and most fonts claim
nothing. `Unclassified` is the common answer, not the error case.
