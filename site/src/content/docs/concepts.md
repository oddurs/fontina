---
title: Concepts
description: "faces, families, identity, containers, sources, tags, collections, activation state."
order: 2
---

## Face

The unit of everything in fontina is the *face*: one font, in one file, at one index.
A `.ttf` holds one face. A `.ttc` collection holds several, numbered from zero. A
variable font is one face with axes, not one face per instance.

A face has an id in the index, assigned on first scan and stable until the file is
pruned. Most commands take a *target*, which is any of:

- a face id from `list`;
- a path to a font file, which is looked up in the index or, for `info`, `check`,
  `css`, `license`, `glyphs` and `specimen`, parsed on the spot;
- `family:<name>`, meaning every face of that family, where the command acts on a set
  (tags and collections).

## What is read

For every face, fontina records, from the tables named:

| Area | Recorded | From |
|---|---|---|
| Names | every `name` record with platform, encoding and language; the preferred family and subfamily (IDs 16 and 17) resolved over the legacy ones (1 and 2); PostScript name; version; designer, manufacturer and their URLs; license text and URL | `name` |
| Classification | weight class, width class, selection flags, embedding rights (`fsType`), vendor id, x-height and cap height | `OS/2` |
| Metrics | units per em, revision, created and modified dates, ascender and descender, italic angle, fixed pitch | `head`, `hhea`, `post` |
| Variation | axes with tag, range and default; named instances; whether `STAT` and `avar` are present | `fvar`, `STAT`, `avar` |
| Layout | feature tags, scripts and languages | `GSUB`, `GPOS` |
| Coverage | the full character map summarised by Unicode block and script; glyph count | `cmap`, `maxp` |
| Capabilities | colour tables (`COLR` v0 and v1, `SVG`, `sbix`, `CBDT`), hinting programs, bitmap strikes, `MATH`, legacy `kern` | table directory |
| CSS descriptor | `font-family`, `font-weight` from 1 to 1000, `font-stretch` as a percentage, `font-style` including oblique angles, `unicode-range` | derived |
| License | an SPDX identifier when the license text is recognised (`OFL-1.1`, `Apache-2.0`, `UFL-1.0`, `MIT`, ...), else `LicenseRef-Unknown`; reserved font names for OFL fonts | derived from `name` |

All of it goes through [fontations](https://github.com/googlefonts/fontations). The
metadata for one face is a JSON document described by `schemas/face.json`.

## Family

Faces are grouped into families by the typographic family name (name ID 16), falling
back to the legacy family name (ID 1), with `STAT` consulted for variable fonts. File
names are never used. `families` lists families and their faces; `list --family`
filters on the exact name.

## Identity and duplicates

Every file gets a BLAKE3 hash of its bytes. Every face also gets an *identity hash*
over its names and outlines, so that the same face delivered as TTF, OTF, WOFF and
WOFF2 is recognised as one font in four containers. `dupes` reports both kinds: same
identity in several files, and different identities sharing a PostScript name, which
is the case that causes conflicts when both are installed.

## Containers

fontina reads TrueType and CFF-flavoured OpenType (`.ttf`, `.otf`), collections
(`.ttc`, `.otc`), WOFF 1.0 and WOFF 2.0. WOFF files are unwrapped to sfnt bytes and
then parsed like anything else; see [ADR 0005](../../adr/0005-woff-decoding/). The
container is recorded on the face and is a filter (`--container woff2`).

## Sources

A *source* is a directory the index was built from. `scan` records the directories it
was given; `source add` registers one explicitly and scans it. Watched sources are
what `fontina watch` follows for changes. A source can be forgotten with
`source remove`, optionally dropping its faces from the index with `--purge`.

## Tags

A *tag* is a free-form label on a face. A face can carry any number. Tags are yours;
fontina never assigns them. They are a filter (`--tag serif`) and a facet.

```
$ fontina tag add serif family:Amiri 4
$ fontina tag list
$ fontina list --tag serif
```

## Collections

A *collection* is an ordered, named set of faces. Unlike a tag, it has an order and it
can be written out as a JSON file and read back into another index. The file format is
`schemas/collection.json`; faces are matched on import by identity hash first, then
PostScript name, then path, so a collection travels between machines whose fonts live
in different directories.

```
$ fontina collection add Editorial family:Amiri 4
$ fontina collection export Editorial > editorial.json
$ fontina collection import editorial.json
```

## Activation state

Each face carries an activation state: `none`, `session`, `user` or `installed`,
set by `activate`, `activate --session` and `install`, and cleared by their inverses.
The index records it together with when it happened, so `restore` can re-apply it
after a reboot. The state is a filter (`--active`, `--activation user`), a facet, and
the `A` flag in `list`. See [Activation and installation](../activation/).

## Conflicts

Two active fonts with the same PostScript name, or the same family and style,
compete for the same slot in every application. fontina checks for that before
activating and refuses with exit status 2 unless told to `--replace`. Duplicates
across containers (the same face as TTF and WOFF2) are not conflicts; activating
one is enough.

## Health checks

`check` runs a fixed set of checks against a face and reports findings at three
levels. Each check has a stable identifier of the form `area/check` that is never
renamed. The full list is in [Health checks](../checks/).
