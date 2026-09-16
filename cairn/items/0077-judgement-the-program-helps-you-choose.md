---
id: 77
key: judgement
title: Judgement — the program helps you choose
type: milestone
status: backlog
created: 2026-09-15
updated: 2026-09-15
---

A font manager's actual job is not storage. It is helping somebody decide which
typeface to use, which is a judgement made by eye and defended by fact.

fontina is very good at the facts and has almost nothing to say about the judgement. It
will tell you a face has 2,847 codepoints, a weight axis from 200 to 800 and an x-height
of 0.52 em. It will not tell you that the four faces you are looking at include two that
are nearly the same, or that the one you picked will fight the one beside it, or that
the thing you are about to activate you already have under a different name.

The material is all there — `related`, `variants`, pairing scores, coverage, the
x-height ratio — and it is spread across five commands that each answer a different
question in a different shape. Judgement is the milestone where the program stops being
a reference and starts having a view: comparison that does not lie, difference stated
in the words of the difference, and the one question a person actually asks — *is this
the right one?* — answered with evidence rather than deflected to a table.

Never with an adjective. `pairing.rs` already holds that line — "no adjective anywhere;
'weight +400' is a fact, 'a strong contrast' is an opinion, and one the reader is better
placed to have" — and it is the line that keeps this from becoming a recommendation
engine.
