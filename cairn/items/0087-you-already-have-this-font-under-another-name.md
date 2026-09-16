---
id: 87
title: You already have this font under another name
type: feat
status: backlog
milestone: judgement
created: 2026-09-15
updated: 2026-09-15
priority: p1
effort: m
crate: core
---

## Problem

`dupes` finds files with the same content hash. It does not find the thing that actually
happens to a font library: four builds of Inter from four sources, under three family
names, at two unit-per-em values, none of which are byte-identical and all of which are
Inter.

Activating the fifth is the mistake the program is best placed to prevent, and it says
nothing.

## Proposal

Before an activation or an install, say what is already there that the face is nearly.
The measurements exist — `related` ranks on coverage overlap, metrics and classification;
#234 gives the x-height ratio; the PostScript-name count from #188 already knows how many
builds hide under one name.

A note, not a block. Report, never enforce.

## Acceptance criteria

- [ ] `activate` and `install` say when something in the index is nearly the same face.
- [ ] The evidence is shown — what matched, by how much — never a bare warning.
- [ ] It never blocks, and never needs a flag to be got past.
- [ ] It costs nothing measurable on an activation in a library of ten thousand.
