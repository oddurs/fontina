---
id: 58
title: Show the words the designer chose
type: feat
status: backlog
milestone: specimen
created: 2026-09-07
updated: 2026-09-07
priority: p2
effort: s
crate: core
---

## Problem

Name ID 19 is the sample text the type designer wrote to show their own face off, and
`parse.rs` already reads it into `Names::sample_text`. `fontina preview` uses it. The
specimen — the one place the type is actually drawn — ignores it and shows a pangram
chosen by `typography::DEFAULT_TEXT` instead.

The words a designer picked to display their face are better evidence about that face than
any string fontina could pick, and they cost nothing: they are parsed, stored and unused.

## Proposal

Where a face declares name ID 19, offer it as the sample text and say whose words they
are, so the reader knows they are looking at the designer's choice rather than fontina's.

Do not make it the default silently. `DEFAULT_TEXT` exists so that the terminal, `fontina
preview` and the specimen show the same words for the same font, and swapping in a
per-font string by surprise breaks that on purpose. Offer it, name it, let the reader take
it.

## Acceptance criteria

- [ ] A face declaring name ID 19 offers it alongside the default sample text.
- [ ] It is attributed, not presented as the reader's own text.
- [ ] A face without one shows no empty control.
