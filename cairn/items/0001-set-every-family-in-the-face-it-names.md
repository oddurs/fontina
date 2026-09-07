---
id: 1
title: Set every family in the face it names
type: feat
status: ready
milestone: tui-craft
created: 2026-09-05
updated: 2026-09-05
priority: p0
effort: l
crate: ui
---

## Problem

The browser lists fonts the way a file manager lists files: one name per row, all
set in the terminal's own face. A person choosing a typeface is choosing how it
looks, and the one view built for choosing shows them none of it.

Everything needed already exists. `render` rasterises shaped text through harfrust
and skrifa; `encode` speaks kitty graphics, iTerm2 images, sixel and half-blocks.
Nothing new has to be parsed or drawn — it has to be put in the list.

## Proposal

Each row in the family pane draws the family's own name in its own face, at the
row's height, through whichever protocol the terminal supports. Where none is
supported the row falls back to the name in the terminal's face, which is exactly
what happens today, so nothing is lost on a terminal that cannot do it.

This is the item that makes the browser worth opening rather than worth reading
about. Everything else in this milestone supports it.

## Acceptance criteria

- [ ] every row in the family pane is set in the face it names, on kitty, iTerm2 and sixel
- [ ] a terminal with no image protocol renders exactly what it renders today
- [ ] a row costs no rasterisation once it has been drawn (see the cache item)
- [ ] `--no-images` turns it off for anyone who wants a text-only browser
- [ ] scrolling a 500-family list stays inside the frame budget

## Notes

Row height is the constraint that decides the design: an image sized to one cell
is unreadable and one sized to three makes a list of twenty. Try two.

## Where this stands after the preview came out of the browser

The half-block rendering was removed in #158, on the argument that a terminal cell is
about one pixel wide and two tall, so type drawn into it is honest about the glyphs and
dishonest about the typeface. `ui/preview.rs` and `ui/sheet.rs` are gone.

That does not retire this item — it settles how it has to be done. The acceptance
criteria above already say **kitty, iTerm2 and sixel**, with the terminal's own face as
the fallback where no protocol is available, and a real image is not what #158 objected
to. What it rules out is the half-block version, which is what the first attempt (#162,
closed) built.

So this stays open, and the thing to build is the image path.
