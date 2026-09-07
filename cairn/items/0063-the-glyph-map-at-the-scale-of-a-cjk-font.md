---
id: 63
title: The glyph map at the scale of a CJK font
type: perf
status: backlog
milestone: specimen
created: 2026-09-07
updated: 2026-09-07
priority: p2
effort: m
crate: core
---

## Problem

Every codepoint a face covers is written into the page as a comma-separated `data-cps`
attribute, and every one becomes a DOM element when its block is opened.

Measured on the fixtures: Amiri, at 1,699 codepoints, produces 9KB of `data-cps` across
twenty blocks, in a 32KB page with `--link`. That is about 6KB of markup per thousand
codepoints. A 25,000-glyph CJK face is therefore roughly 150KB of attributes before the
font is embedded at all, and opening one CJK block builds 25,000 elements in a single
synchronous loop.

Nothing in the fixtures is near that size, so the ceiling has never been met. A font
manager pointed at a real library will meet it on the first Noto CJK it finds.

## Proposal

Stop shipping the codepoints twice. `Coverage::ranges` is already ranges; emit ranges
rather than an expanded list and let the page expand them, which is most of the markup
gone for the fonts where it matters and no change at all for the fonts where it does not.

Build cells for what is on screen rather than for the whole block, so opening a large
block costs what is visible.

Measure it: a fixture-sized page must not regress, and a synthetic large-coverage face
should open a block without a visible stall.

## Acceptance criteria

- [ ] Codepoints travel as ranges, not as an expanded list.
- [ ] Opening a block of 20,000 codepoints does not stall.
- [ ] Page size for the existing fixtures does not grow.
- [ ] The map is still complete and still correct at the block boundaries.

## Notes

Numbers above are measured on `fixtures/Amiri-Regular.ttf`, not estimated.
