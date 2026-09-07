---
id: 59
title: A feature you can watch working
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

The feature toggles say a feature exists and let you turn it on. They do not show what it
does. A reader who ticks `ss03` on an unfamiliar face sees the page change and has no idea
which glyphs changed, which is the entire content of the question "what is stylistic set
three".

Kerning is worse: `Features::gpos` is parsed and never offered, so the specimen cannot
answer "does this font kern", which is among the first things anyone asks of a text face
and is trivially visible in `AV`, `To`, `Yo` at size.

## Proposal

Set a before-and-after line for the active feature: the same string with the feature off
and on, one above the other, with the glyphs that differ marked. That turns a checkbox
into a demonstration, and it is the only way a numbered stylistic set is legible at all.

Offer the GPOS features too, kerning first, on the same footing — with a kerning pair
string that shows the pairs a reader would look for rather than the general sample.

Feature labelling stays in `typography`, where `feature_label` and `is_toggleable` already
live, so the browser and the specimen keep the same opinion about which features are a
typographic choice.

## Acceptance criteria

- [ ] Turning a feature on shows the same text with it off and on, differences marked.
- [ ] Kerning is toggleable, with a string that makes kerning visible.
- [ ] The set of features offered still comes from `typography::toggleable_features`.
