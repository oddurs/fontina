---
id: 56
title: Set a paragraph, not only a ladder
type: feat
status: backlog
milestone: specimen
created: 2026-09-07
updated: 2026-09-07
priority: p1
effort: m
crate: core
---

## Problem

A waterfall answers "what do the letterforms do as they grow". It does not answer the
question a person choosing a text face actually has, which is "what does a page of this
look like at the size, leading and measure I am going to set it at".

The sheet has one paragraph per covered script at a fixed 19px and a fixed measure, chosen
by `specimen.rs` rather than by the reader. There is no way to set 10 on 14 across 32 ems
and look at the colour of the resulting block, which is most of what choosing a text face
means.

## Proposal

One setting block, under the reader's control: size, leading, measure, and alignment
including justified — the four decisions that turn a typeface into set type. Text is the
reader's own, from the sample field, falling back to the script paragraph.

Show the resulting measure in characters as it changes, because "how many characters to
the line" is the number that actually governs readability and nobody can eyeball it in
ems.

Justified is worth having rather than a nicety: a face's fitness for justified setting is
a real property, and hyphenation and word-space behaviour are visible only there.

## Acceptance criteria

- [ ] Size, leading, measure and alignment are settable, and the block re-sets live.
- [ ] The measure is reported in characters as well as in its own unit.
- [ ] The setting travels into print at the size it was set at.
- [ ] A right-to-left script sets right-to-left under the same controls.
