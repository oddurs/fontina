# 0010 — the browser shows what is in a font, not a picture of it

**Status:** accepted, 2026-09-06.

## Context

`fontina ui` drew type. The details pane rendered the sample text as half-block
characters under the metadata; `w` set one face down a size ladder and `C` set every
face in the selection side by side, both full screen, both in the same half-blocks.

A terminal cell is roughly one pixel wide and two tall. Type drawn into it arrives
through a filter that changes its apparent weight, loses its spacing, and destroys the
detail that distinguishes one face from another — which is precisely the detail a person
opens a font manager to judge. The rendering was honest about the glyphs and dishonest
about the typeface: two faces that differ by exactly the things a designer chooses
between looked the same, and one face looked like something it is not.

There is no fixing this by rendering harder. The resolution is the medium.

## Decision

The browser shows structure. Everything it draws is text and number: names, style, axes
and their ranges, features, coverage per script, metrics, licence, activation state, the
glyph map, the facets.

Looking at type happens where type can be drawn:

- `s` writes a self-contained HTML specimen and opens it — a waterfall, script samples,
  axis sliders, feature toggles, a glyph map, with real antialiasing and real spacing.
- `fontina preview` draws a true image over kitty graphics, iTerm2 or sixel, and a
  half-block fallback for a terminal with no protocol, because a fallback a person asked
  for on the command line is a different thing from a rendering nobody chose.

So the half-block preview pane, the waterfall sheet and the comparison sheet are gone,
with the keys that reached them (`e`, `+`, `-`, `w`, `C`) and the render cache behind
them. `crates/fontina-cli/src/ui/preview.rs` and `sheet.rs` are deleted.

The space the preview used now carries the measurements: units per em, ascender,
descender, line gap, cap height, x-height, the x-height ratio, and coverage per script
with a bar and a count. Those are comparable between two faces at a glance, which the
picture never was.

The controls stay. They move a position, and the position goes to the status line as the
command that would draw it, in keeping with the rule that every action in the browser
names its own command line.

## Consequences

Nothing in the browser is approximate any more; what it shows is what the index holds.
The program is smaller by about 1,700 lines, has one fewer cache, one fewer rasteriser
call path, and no per-frame render budget to defend.

A reader who wants to *see* a font is one keystroke from a specimen that shows it
properly, which is a better answer than the one they had. The cost is that the answer
opens outside the terminal, and on a machine with no browser and no image protocol there
is no way to look at type at all — a real cost, accepted, because the alternative was a
picture that misleads.

Two open pieces of work are dropped by this: the specimen sheet (#135), which added a
third rasterised sheet, and cairn 0002, which proposed making the terminal's rendering
match the HTML one. Cairn 0013 (pin faces and compare them side by side) survives as a
comparison of *structure* rather than of pictures.
