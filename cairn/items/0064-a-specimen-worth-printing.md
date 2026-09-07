---
id: 64
title: A specimen worth printing
type: feat
status: backlog
milestone: specimen
created: 2026-09-07
updated: 2026-09-07
priority: p3
effort: s
crate: core
---

## Problem

A printed specimen is a real deliverable — the thing a designer puts in front of a client,
and the form the genre was invented in. The page prints legibly today and no more: it
knows nothing about paper size, it has no cover, and what it prints is whatever was on
screen.

## Proposal

Treat print as an output rather than a side effect. A cover carrying the family, the
designer, the licence and the date. Page size and margins that a person can choose.
Running heads so a loose page still says which face it is.

Decide deliberately what a printed sheet includes rather than inheriting it: the ladder
and the set paragraph always; the glyph map only where the reader asked for it, since a
CJK map is a hundred pages nobody wanted.

## Acceptance criteria

- [ ] A cover page carrying family, designer, licence and date.
- [ ] Page size and margins are settable.
- [ ] Running heads name the face.
- [ ] The glyph map is included only on request.
- [ ] Printing still needs no network and no external stylesheet.
