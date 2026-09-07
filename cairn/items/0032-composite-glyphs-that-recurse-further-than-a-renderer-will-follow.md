---
id: 32
title: Composite glyphs that recurse further than a renderer will follow
type: feat
status: dropped
milestone: integrity
created: 2026-09-06
updated: 2026-09-07
priority: p3
effort: m
crate: core
---

## Problem

A composite glyph references other glyphs, which may themselves be composite. Deep or
cyclic nesting has been a denial-of-service vector against font renderers for two decades,
and nothing here measures it.

## Proposal

Measure the maximum composite depth at parse time and report a `glyf/composite-depth`
finding past a threshold.

## Notes — read before starting this one

Filed at p3, and honestly it may not be worth doing.

fontations already refuses what it cannot safely traverse, and `scan::parse_paths` wraps
parsing in `catch_unwind` as a last line of defence. The fuzzing covers this ground
directly: `fuzz/regressions/` already holds `gpos-scriptlist-quadratic`,
`woff1-origlength-allocation` and `woff1-table-count-overflow` — real hostile-input
findings, fixed, and replayed on every build.

So the threat this would address is largely addressed, and by the right mechanism. A depth
number reported to a person is not obviously actionable: nobody knows what depth is too
deep for the renderer they happen to be using, and a threshold picked here would be
invention rather than a standard.

Before implementing, answer: what would a reader *do* with this finding? If the answer is
"nothing", drop the item rather than shipping a number that looks like a warning and is
not one. That is a legitimate outcome and it is why this is filed rather than built.

## Measured, and dropped

The item set its own test — *what would a reader do with this finding?* — so the depths
were surveyed before anything was written. 335 TrueType files across
`/System/Library/Fonts`, `~/Library/Fonts` and `fixtures/`:

    depth 0    78
    depth 1   203
    depth 2    24
    depth 3    28
    depth 4     2
    cycles      0

The two deepest are Apple's own `SFCompactRounded` and `SFNSMono`. Nothing reaches even
the five levels the OpenType spec recommends as a limit, and no font on the machine has a
cycle.

So there is no threshold to set. Above 4 the check never fires; at 4 or below it fires on
fonts Apple ships. Either way it tells a reader nothing they can act on, which is the
answer the item asked for.

The shape that *is* dangerous — a cycle, or a depth in the hundreds — does not occur in
real fonts at all. It occurs in hostile input, and hostile input already has the right
mechanism: fontations refuses what it cannot traverse, `scan::parse_paths` wraps parsing
in `catch_unwind`, and `fuzz/regressions/` holds three findings of exactly this kind
(`gpos-scriptlist-quadratic`, `woff1-origlength-allocation`, `woff1-table-count-overflow`)
that are replayed on every build.

Dropped. The threat is real and it is already addressed, by fuzzing rather than by a
number in a report.
