---
id: 60
title: Will this font set my text?
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

The glyph map answers "which Unicode blocks does this font cover", which is a question
about the font. The question a person actually has is about their document: *will this
face set the thing I need to set?* A twenty-block coverage list does not answer it, and
nobody is going to check three thousand codepoints by eye.

`Coverage::ranges` already holds exactly what is needed to answer it exactly.

## Proposal

Take text — pasted, or the sample field — and report what the face cannot set: the missing
codepoints, named and counted, with the words they appear in so the reader sees the damage
in context rather than as a list of hex.

Answer it for the comparison too. Three faces and one document, and the useful column is
"how much of this can each of you set", which is a real reason to choose one over another
and something no other specimen tool will tell you.

Reported, never enforced: a face that cannot set the text is still drawn at full size.
The finding is information for a person choosing, not a verdict.

## Acceptance criteria

- [ ] Pasted text yields the codepoints the face is missing, named and in context.
- [ ] Text the face covers completely says so plainly.
- [ ] The comparison reports coverage of the same text per face.
- [ ] Combining marks and codepoints outside the BMP are counted correctly.
