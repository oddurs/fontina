# 0001 — Parse fonts with fontations

**Status:** accepted, 2026-09-03

## Context
We need a correct, fast, safe OpenType parser covering variable fonts, color tables,
layout tables and collections. Options: hand-written parser, `ttf-parser`, `allsorts`,
`fontations` (`read-fonts` + `skrifa`).

## Decision
fontations. It is maintained by Google Fonts, used by Chrome/Skia, zero-copy, fuzzed,
tracks the spec closely and exposes every table we need through typed accessors.

## Consequences
Metadata quality follows fontations releases. WOFF2 decoding is out of scope for
fontations and handled separately (ADR 0005). We never hand-parse a table it exposes.
