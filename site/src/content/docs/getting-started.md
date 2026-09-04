---
title: Getting started
description: install, index your fonts, and ask the first questions.
order: 1
---

## Installing

Get a release binary from the [download page](../../download/) and put it on your
`PATH`, or build it with Rust 1.88 or newer:

```
cargo install --git https://github.com/oddurs/unifont unifont-cli
```

Check that it runs:

```
$ unifont --version
unifont 0.0.1
```

## Indexing

unifont keeps what it learns about your fonts in an index, a single SQLite file in
the platform data directory (see [The index and paths](../index-and-paths/)). The
index starts empty. Fill it by scanning:

```
$ unifont scan --system            # the operating system's font directories
$ unifont scan ~/Fonts             # and your own
scanned 6 candidates in 0.03s: 6 parsed (6 faces), 0 unchanged, 0 removed, 0 failed
```

A scan walks the directories, parses every font file it finds, and records one
*face* per font. A rescan is fast: files whose size and modification time have not
changed are skipped. Use `--prune` to drop entries for files that have gone, and
`--force` to re-parse everything, which is needed after upgrading to a version of
unifont that extracts more.

If you want a directory rescanned in future without naming it every time, register it
as a source instead:

```
$ unifont source add ~/Fonts
```

## Looking

`list` prints every face in the index, one line each:

```
$ unifont list
    id  family               style           wght  wdth  flags  license       path
     1  Amiri                Regular          400   100  ----   OFL-1.1       /home/me/Fonts/Amiri-Regular.ttf
     2  Bricolage Grotesque  96pt ExtraBold   800   100  V---   OFL-1.1       /home/me/Fonts/BricolageGrotesque[opsz,wdth,wght].ttf
     3  Nabla                Regular          400   100  VC--   OFL-1.1       /home/me/Fonts/Nabla[EDPT,EHLT].ttf
     4  Source Serif 4       Regular          400   100  ----   OFL-1.1       /home/me/Fonts/SourceSerif4-Regular.otf
4 face(s)
```

The `id` column is the face's number in the index; most commands accept it. The
flags are `V` for variable, `C` for colour, `I` for italic and `A` for active.

Filters narrow the list. They combine, and every one also applies to `families` and
`facets`:

```
$ unifont list --script Arab          # faces whose cmap covers Arabic
$ unifont list --variable             # variable fonts
$ unifont list --license OFL          # by SPDX identifier prefix
$ unifont list --weight 600-900       # by CSS font-weight range
$ unifont list --vendor ADBE          # by OS/2 vendor id
$ unifont list grotesque              # full-text search over names and designer
```

`info` prints everything known about one face. It takes an id, or a path to any font
file, indexed or not:

```
$ unifont info 2
$ unifont info ~/Downloads/Mystery.ttf
```

`facets` shows the shape of the whole collection, or of any filtered subset:

```
$ unifont facets
6 face(s) in 5 family(ies)
weight      400 Regular 5 · 800 ExtraBold 1
width       100% Normal 6
style       upright 6
variable    2   color 1
container   ttf 3 · otf 1 · woff 1 · woff2 1
script      Latn 6 · Zinh 6 · Zyyy 6 · Grek 2 · Arab 1 · Cyrl 1
license     OFL-1.1 6
vendor      RSMS 2 · ADBO 1 · ALIF 1 · ATLR 1 · TYPT 1
activation  none 6
source      /home/me/Fonts 6
```

## Asking

Which fonts can set this text?

```
$ unifont covers "Þórður át 12 blóðbergsbrauð"
```

Which files are the same font twice?

```
$ unifont dupes
```

Is this font sound?

```
$ unifont check ~/Fonts/*.ttf
PASS  Amiri Regular  (/home/me/Fonts/Amiri-Regular.ttf#0)  0 error(s), 0 warning(s)
1 face(s) checked, 0 failed
```

May I embed it?

```
$ unifont license 1
OFL-1.1  (1 face(s))
  Amiri Regular  [Installable]  /home/me/Fonts/Amiri-Regular.ttf
```

## Exporting

`css` writes `@font-face` rules, with a URL prefix of your choosing instead of file
paths:

```
$ unifont css 1 2 --url-prefix /fonts/ > fonts.css
```

`specimen` writes a self-contained HTML page with a waterfall, script samples, axis
sliders, feature toggles, a glyph map and side-by-side comparison. It embeds the font
files, so it works from disk with no server:

```
$ unifont specimen 1 2 3 -o specimen.html
```

Every command takes `--json` for machine-readable output; see
[JSON output and schemas](../json-and-schemas/).

## Where to go next

- [Concepts](../concepts/) explains faces, families, identity, sources, tags,
  collections and activation state.
- The [command reference](../cli/) is the man page.
