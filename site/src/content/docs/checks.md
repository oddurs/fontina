---
title: Health checks
description: "what `fontina check` looks for, every check identifier, and the exit status rules."
order: 7
---

`fontina check` is a small, fast subset of what
[fontbakery](https://github.com/fonttools/fontbakery) does: the checks that catch a
broken or misdeclared font before it reaches a project, not the checks that judge
its design. It runs on indexed faces or on any file.

```
$ fontina check ~/Fonts/*.ttf
PASS  Amiri Regular  (/home/me/Fonts/Amiri-Regular.ttf#0)  0 error(s), 0 warning(s)
1 face(s) checked, 0 failed
```

## Levels and exit status

Each finding has a level:

- **error**: the font is malformed or unusable as declared. `check` exits 1.
- **warn**: the font will misbehave somewhere: wrong style linking, unshaped text,
  varying line height. `check` exits 0 unless `--strict`.
- **info**: worth knowing, not wrong.

`--min warn` hides info findings; `--min error` hides warnings too. `--json` prints
every finding with its `id`, `level` and `message`.

## Identifiers

Every check has an identifier `area/check`. Identifiers are stable: one is never
renamed or reused, so a script that ignores `hinting/none` today will still ignore
it in ten years. New checks are added under a new identifier and announced in the
changelog. Some identifiers are reported at more than one level depending on what
was found.

| Id | Level | Finding |
|---|---|---|
| `name/family` | error | no family name (name IDs 1 and 16 are empty) |
| `name/postscript` | error, warn | no PostScript name; or one that is over 63 characters or contains characters outside printable ASCII |
| `name/full` | warn | full name does not start with the family name |
| `name/version` | warn | no version string (name ID 5), or one that is not parseable |
| `name/designer` | info | no designer or manufacturer recorded (name IDs 8, 9) |
| `os2/missing` | error | no `OS/2` table |
| `os2/weight-class` | error | `usWeightClass` outside 1..1000 |
| `os2/width-class` | error | `usWidthClass` outside 1..9 |
| `os2/bold-weight` | warn | `fsSelection` BOLD is set but `usWeightClass` disagrees |
| `os2/fs-selection` | warn | `fsSelection` sets REGULAR together with BOLD or ITALIC |
| `os2/italic-angle` | warn | `fsSelection` says italic but `post.italicAngle` is 0 |
| `os2/fs-type` | info | `fsType` restricts embedding: no embedding, preview and print only, or bitmap only |
| `os2/vendor-id` | info | `achVendID` is unset or `UKWN` |
| `metrics/typo-vs-hhea` | warn | `OS/2` typo metrics differ from `hhea` and USE_TYPO_METRICS is not set; line height will vary by platform |
| `head/units-per-em` | error | `unitsPerEm` outside 16..16384 |
| `head/created` | info | `head.created` is unset |
| `hhea/ascender` | error | `hhea.ascender` is not positive |
| `hhea/descender` | warn | `hhea.descender` is positive |
| `glyf/empty` | error | font has no glyphs |
| `outlines/none` | error | no `glyf`, `CFF` or `CFF2` outlines and no bitmap strikes |
| `hinting/none` | info | TrueType outlines without hinting programs |
| `cmap/empty` | error | `cmap` maps no codepoints |
| `cmap/space` | warn | U+0020 SPACE is not mapped |
| `cmap/nbsp` | info | U+00A0 NO-BREAK SPACE is not mapped |
| `cmap/basic-latin` | warn | a Latin font does not map all of A to Z |
| `fvar/stat` | warn | variable font without a `STAT` table; style linking will be wrong in many apps |
| `fvar/instances` | warn | variable font without named instances |
| `fvar/instance-name` | warn | named instance without a resolvable name |
| `fvar/axis-range` | warn | an axis has a zero-width range |
| `fvar/axis-tag` | warn | a lowercase axis tag that is not a registered axis; custom tags should be uppercase |
| `layout/shaping` | warn | the `cmap` covers a script that needs shaping, but `GSUB`/`GPOS` have no such script; text will not shape |
| `layout/kerning` | info | no `GPOS` table and no legacy `kern` table |
| `license/missing` | warn | no license text or URL embedded (name IDs 13, 14) |
| `license/unknown` | warn | license text present but not recognised as any SPDX license |
| `license/url` | info | OFL font without a license URL (name ID 14) |
| `license/rfn` | info | a Reserved Font Name is declared that does not match the family name |
| `license/copyright` | warn | no copyright notice (name ID 0) |
| `file/extension` | warn | the file extension does not match the container and outline format |

## What it does not do

It does not rasterise, so it cannot find overlapping contours, wrong-direction
paths or missing glyph outlines. It does not judge vertical metrics against a
family, spacing, or kerning quality. For those, run fontbakery. fontina's checks
are meant to be cheap enough to run on every scan.

## Adding a check

A new check needs a new identifier under an existing or new area, a message that
says what was found and where in the font, and a fixture font under an open license
that triggers it with a test asserting so. See [Contributing](../../contributing/).
