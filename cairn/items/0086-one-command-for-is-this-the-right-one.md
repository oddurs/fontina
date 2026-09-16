---
id: 86
title: One command for is this the right one
type: feat
status: backlog
milestone: judgement
created: 2026-09-15
updated: 2026-09-15
priority: p0
effort: m
crate: cli
---

## Problem

The evidence for choosing a typeface is spread across five commands that each answer a
different question in a different shape: `info` for the facts, `variants` for the score
and metrics, `related` for what is near it, `covers` for whether it can set your text,
`check` for whether it is sound.

A person deciding between two faces runs four of them and holds the answers in their
head. The program has all of it and no view.

## Proposal

`fontina compare <faces>...` — one screen, one judgement. What differs, in the words of
the difference. Where they are the same. What each can set that the other cannot. What
the x-height ratio between them is, now that #234 makes that a shared function.

No adjectives, which `pairing.rs` already holds the line on: "weight +400" is a fact, "a
strong contrast" is an opinion, and one the reader is better placed to have.

## Acceptance criteria

- [ ] `fontina compare A B` puts the differences that matter on one screen.
- [ ] It reuses `typography` and `pairing` rather than computing anything a second time.
- [ ] No adjective describes a difference; the numbers are the description.
- [ ] `--json` emits a type in `schemas/cli-output.json`.
- [ ] It works on faces, on families, and on paths to files that are not indexed.
