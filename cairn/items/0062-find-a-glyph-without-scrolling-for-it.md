---
id: 62
title: Find a glyph without scrolling for it
type: feat
status: backlog
milestone: specimen
created: 2026-09-07
updated: 2026-09-07
priority: p2
effort: m
crate: core
---

## Problem

The glyph map is a wall. Twenty collapsed blocks, and once one is open, hundreds of cells
in codepoint order with a hex label. There is no way to search it, and no way to look at
one glyph properly — a 54px cell is too small to judge a letterform, which is what someone
opening a glyph map is usually trying to do.

## Proposal

A search over the map that takes a codepoint, a hex value, a character pasted in, or a
Unicode name, and filters the blocks to what matches.

Clicking a cell opens the glyph large, with its codepoint, its Unicode name, and its
advance — the detail view the cell is too small to be. Appending to the sample text, which
is what a click does today, moves onto that view where there is room to say so.

Both want the same substrate: `unicode.rs` already blocks and names ranges, so the naming
belongs there rather than in a second table inside `specimen.rs`.

## Acceptance criteria

- [ ] Searching by codepoint, hex, pasted character or name filters the map.
- [ ] A glyph opens large with its codepoint, name and advance.
- [ ] Adding a glyph to the sample text is still reachable.
- [ ] The map stays usable by keyboard without becoming a thousand tab stops.
