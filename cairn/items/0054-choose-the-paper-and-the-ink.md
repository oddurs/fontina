---
id: 54
title: Choose the paper and the ink
type: feat
status: backlog
milestone: specimen
created: 2026-09-07
updated: 2026-09-07
priority: p1
effort: s
crate: core
---

## Problem

The page takes its polarity from the reader's operating system and offers no way to change
it. That is fine for chrome and wrong for type.

Polarity is a typographic fact, not a preference: the same face set light on dark reads
heavier than dark on light, which is one of the things a person opens a specimen to judge.
A designer choosing a face for a dark interface cannot see it on dark unless their laptop
happens to be in dark mode, and a colour font makes it worse — the Nabla fixture renders
gold on near-black by accident, and a COLR face with dark parts in it would disappear into
the page with no way to rescue it.

## Proposal

Two controls in the bar: paper and ink, each a colour, plus an invert that swaps them.
Default to the current theme so nothing changes for a reader who does not care.

They set the specimen surface only — the type column, the samples, the glyph cells — and
leave the chrome alone. The rail, the guides and the controls stay legible against the
page whatever the paper is, which is the point of drawing them in a separate ink in the
first place.

Persist the choice in the document state so it survives a print and travels with a shared
specimen.

## Acceptance criteria

- [ ] Paper and ink are settable, with an invert, defaulting to the reader's theme.
- [ ] The controls do not repaint the chrome into illegibility at any setting.
- [ ] A colour font is judged against the chosen paper, not the OS's.
- [ ] Printing uses the chosen paper, or white, deliberately rather than by accident.
