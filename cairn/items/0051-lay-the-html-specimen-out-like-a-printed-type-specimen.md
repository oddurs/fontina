---
id: 51
title: Lay the HTML specimen out like a printed type specimen
type: feat
status: done
milestone: specimen
created: 2026-09-07
updated: 2026-09-07
priority: p1
effort: m
crate: core
---

## Problem

`specimen.rs` showed everything a specimen has to show and gave none of it a shape. Every
section ran the full width of the window with no measure, the controls sat between the
samples so the type was interrupted by chrome at every step, and the metadata was a grid
of grey pairs nobody could scan. It worked; it did not let anyone judge a typeface.

There was also a defect the layout hid. The Size slider drove `.sample`, which only exists
in the comparison block, so on a single-face specimen — the common case, and the one the
TUI's `s` key writes — dragging it did nothing at all.

## Proposal

The page's one rule: a specimen exists so a person can judge letterforms, so nothing on it
may be mistaken for the type on it. Take the convention prepress already settled on and
draw every guide in non-photo blue, the ink that does not reproduce. Rules, grid lines,
size markers, coverage meters and every interactive label are blue; `--ink` is spent only
on the face being shown. Nothing sets `font-smoothing` or a text shadow, for the same
reason.

Structurally that splits each sheet in two: a sticky rail carrying the instrument — the
specifications, the axis sliders, the feature toggles — and a column carrying nothing but
type, divided by a guide line. The face names itself as the hero, set in itself, and the
live sample under it is what the Size slider drives, which is also the fix for the dead
slider.

The waterfall baseline-aligns each rung to its size mark in the guide column, so the marks
read as measurements of the type rather than labels beside it. The glyph map draws its own
grid lines per cell — a background showing through the gaps left a block whose last row
was short ending in a slab of blue — and each collapsed block carries a coverage meter so
the closed ones still say something.

## Acceptance criteria

- [x] The Size slider changes the type on a single-face specimen.
- [x] A named instance selects itself when the coordinates land exactly on one, and says
      Custom otherwise, matching `typography::matching_instance`'s exactness rule.
- [x] No horizontal overflow at a 390px layout width.
- [x] The specimen still makes no network request.
- [x] Printing drops the guides to grey, hides the controls, and keeps the axis values, so
      a printed sheet records where the face was set.
- [x] `cargo test`, `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`
      pass.

## Notes

Verified by rendering in headless Chrome at desktop, narrow and print: the script executes
(189 glyph cells built for the two blocks open by default, 95 + 94), the document reports
`scrollWidth == clientWidth` at 390px, and the output contains no `http://` or `https://`.

The type column is about 220px narrower than the old full-width layout, so the waterfall
truncates a little earlier at 72 and 96px. That is the price of controls that stay in
reach; the hero sample takes the full page width to make up for it.

`unfiled` because this is core and HTML, not the TUI, and none of the shipped milestones
covers it — triage should move it or drop it.
