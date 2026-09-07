---
id: 28
title: The command line prints in one colour and one width
type: feat
status: done
milestone: tui-craft
created: 2026-09-06
updated: 2026-09-07
priority: p1
effort: l
crate: cli
---

## Problem

The browser got a palette (ADR-less, `ui/theme.rs`), a layout that knows the terminal's
width, and a year of attention. The command line — which is the half of the program a
script and a first-time reader both meet — prints in one colour, at one width, in
columns sized to their content and nothing else.

On a real 733-family library:

    id  family                            style                             wght    wdth  flags   license       path
     5  .ADT Slab Numeric                 Light                         300-1000     100  V----N  LicenseRef-…  /System/Library/Fonts/ADTNumeric.ttc#4

- `style` is padded to twenty-eight columns whether or not anything needs them, so a
  hundred-column terminal spends a third of its width on air and then wraps the path.
- `V----N` is a six-character mask with no legend anywhere. Nothing says which position
  is which, and the four that are off are as loud as the two that are on.
- A path is the longest thing in the row and the least scannable: the part that
  identifies the font is the last twenty characters, and the eighty before it are the
  same eighty on every row.
- Nothing is coloured. Not the header, not a failing check, not a nonfree licence, not
  the difference between a face that is active and one that is not.
- `stats` misaligns its own labels (`collections:` is a column wider than the rest).
- `facets` prints one line per facet however long it is, so on a real library every row
  wraps three times and the facet names stop lining up.

## Proposal

One `Term`: what the terminal is — its colour depth and its width — resolved once, in
`main`, and read by every printer.

- **Colour by role, never by name.** `head`, `dim`, `accent`, `good`, `warn`, `bad`,
  the same six the browser's theme resolves, so one word means one thing in both halves
  of the program. No flag: `--color` is already a filter here (`fontina list --color` is
  the fonts that carry their own colour), and two flags of one name is worse than
  reading the two variables every other tool reads. `NO_COLOR` says never,
  `CLICOLOR_FORCE` says always, and between them a terminal is coloured and a pipe is
  not.
- **Width from the terminal, or from `COLUMNS`.** Columns are fitted to the width when
  there is one and left exactly as they are when there is not, so a pipe gets the bytes
  it has always got and a person gets a table that fits.
- **A path is dimmed to its directory and lit at its filename**, and shortened from the
  middle, because the end is the part that says which font this is.
- **The flag mask keeps its positions and loses its noise**: what is set is coloured,
  what is not is a dim `·`, and a legend goes under the table.

Then every human-readable printer moves onto it: `list`, `families`, `facets`, `info`,
`stats`, `check`, `license`, `glyphs`, `dupes`, `variants`, `conflicts`, `activations`,
`covers`, `dirs`, `config`.

## Acceptance criteria

- [x] no escape ever reaches something that is not a terminal, and a pipe is never
      re-fitted
- [x] `NO_COLOR` produces those same bytes on a terminal, and beats `CLICOLOR_FORCE`
- [x] `CLICOLOR_FORCE` colours with no terminal at all, so the tests can see it
- [x] strip the escapes from a coloured run and it is the piped run, byte for byte
- [x] `COLUMNS` fits the table, and no cell ever exceeds the width it was given
- [x] a path column keeps the filename and shortens the directory on a separator
- [x] every readable command paints something, so the palette cannot rot into a module
      nothing calls
