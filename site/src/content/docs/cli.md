---
title: Command reference
description: "fontina(1), in the form of a man page."
order: 3
---

This chapter follows the layout of a manual page. `fontina help <command>` prints the
same information from the binary itself, and is always current for the version you
have.

## NAME

fontina: scan, search, inspect, check and export fonts.

## SYNOPSIS

```
fontina [--db PATH] <command> [options] [arguments]
```

## DESCRIPTION

fontina indexes font files into a SQLite database and answers questions about them.
The index is a single file; `--db` or the `FONTINA_DB` environment variable selects
it, otherwise it lives in the platform data directory. Every command that prints a
report accepts `--json`; the output types are published in
`schemas/cli-output.json`.

A *target* argument is a face id from `list`, a path to a font file, or, for
commands that act on a set of faces, `family:<name>`.

## COMMANDS

### Indexing

<dl>
<dt><code>scan [PATHS]... [--system] [--force] [--follow-symlinks] [--prune] [--json]</code></dt>
<dd>Index fonts under one or more directories or files. <code>--system</code> adds the
operating system's font directories. Unchanged files (same size and modification
time) are skipped unless <code>--force</code>. <code>--prune</code> drops index entries
under the scanned roots whose files no longer exist. Symlinks are not followed
unless asked.</dd>

<dt><code>source list</code></dt>
<dd>The directories the index was built from, with face counts and whether they are
watched.</dd>
<dt><code>source add PATH [--no-watch] [--json]</code></dt>
<dd>Register a directory and scan it now. It is followed by <code>watch</code> unless
<code>--no-watch</code>.</dd>
<dt><code>source remove PATH [--purge]</code></dt>
<dd>Forget a directory. With <code>--purge</code>, drop its faces from the index too.</dd>
<dt><code>source watch PATH [--off]</code></dt>
<dd>Turn watching on (the default) or off for a source.</dd>

<dt><code>stats [--json]</code></dt>
<dd>Index statistics: files, faces, families, variable and colour counts, failures,
tags, collections, sources, active faces; and the most recent parse failures.</dd>

<dt><code>dirs [--json]</code></dt>
<dd>Print the operating system's font directories, marking the per-user one.</dd>
</dl>

### Querying

The three query commands share one set of filters, listed under
[FILTERS](#filters).

<dl>
<dt><code>list [QUERY] [filters] [-n LIMIT] [--json]</code></dt>
<dd>List indexed faces, one per line: id, family, style, weight, width, flags,
license, path. <code>QUERY</code> is a full-text search over family, style,
PostScript name and designer.</dd>

<dt><code>families [QUERY] [filters] [-n LIMIT] [--json]</code></dt>
<dd>The same faces grouped by typographic family name.</dd>

<dt><code>facets [QUERY] [filters] [--json]</code></dt>
<dd>Count the matching faces per weight, width, style, script, license, vendor, tag,
collection, activation state and source.</dd>

<dt><code>info TARGET [--json]</code></dt>
<dd>Everything known about one face. A path that is not indexed is parsed on the
spot.</dd>

<dt><code>covers TEXT [--variable] [--under PREFIX] [-n LIMIT] [--json]</code></dt>
<dd>Faces whose character map covers every character of <code>TEXT</code>.</dd>

<dt><code>glyphs TARGET [--block NAME] [--json]</code></dt>
<dd>A face's character coverage by Unicode block. <code>--block</code> prints the
characters of one block, matched by case-insensitive substring of its name.</dd>

<dt><code>dupes [--json]</code></dt>
<dd>Faces that are the same font in several containers, and faces that share a
PostScript name.</dd>
</dl>

### Organising

<dl>
<dt><code>tag list</code></dt>
<dd>All tags with their face counts.</dd>
<dt><code>tag add TAG TARGETS...</code></dt>
<dd>Add a tag to faces; the tag is created if new.</dd>
<dt><code>tag remove TAG TARGETS...</code></dt>
<dd>Remove a tag from faces.</dd>
<dt><code>tag rename OLD NEW</code>, <code>tag delete TAG</code></dt>
<dd>Rename a tag everywhere; delete it from every face.</dd>

<dt><code>collection list</code></dt>
<dd>All collections with their face counts.</dd>
<dt><code>collection create NAME</code>, <code>collection delete NAME</code>, <code>collection rename OLD NEW</code></dt>
<dd>Manage collections by name.</dd>
<dt><code>collection add NAME TARGETS...</code></dt>
<dd>Append faces to a collection, creating it if missing.</dd>
<dt><code>collection remove NAME TARGETS...</code></dt>
<dd>Remove faces from a collection.</dd>
<dt><code>collection show NAME</code></dt>
<dd>The faces of a collection, in order.</dd>
<dt><code>collection export NAME [OUTPUT]</code></dt>
<dd>Write a collection as JSON conforming to <code>schemas/collection.json</code>.
<code>OUTPUT</code> defaults to standard output.</dd>
<dt><code>collection import FILE</code></dt>
<dd>Read a collection JSON file into this index, matching faces by identity hash,
then PostScript name, then path.</dd>
</dl>

### Checking

<dl>
<dt><code>check [TARGETS]... [--strict] [--min LEVEL] [--json]</code></dt>
<dd>Run health checks. Findings are <code>info</code>, <code>warn</code> or
<code>error</code>; <code>--min</code> hides those below a level. The exit status is 1
if any check reports an error, or with <code>--strict</code>, a warning. See
<a href="../checks/">Health checks</a>.</dd>

<dt><code>license [TARGETS]... [--json]</code></dt>
<dd>License and embedding report: SPDX identifier, embedding rights from
<code>fsType</code>, and reserved font names. Covers every indexed face when no
target is given.</dd>
</dl>

### Exporting

<dl>
<dt><code>css [TARGETS]... [--url-prefix PREFIX]</code></dt>
<dd>Emit one <code>@font-face</code> rule per face with <code>font-weight</code>,
<code>font-stretch</code>, <code>font-style</code>, <code>font-display: swap</code>,
<code>src</code> with the right <code>format()</code>, and <code>unicode-range</code>
from the character map. Without <code>--url-prefix</code> the sources are
<code>file://</code> paths.</dd>

<dt><code>specimen [TARGETS]... [-o FILE] [--text TEXT] [--title TITLE] [--link]</code></dt>
<dd>Write a self-contained HTML specimen: waterfall, script samples, axis sliders,
feature toggles, glyph map, and side-by-side comparison of several faces. Fonts are
embedded unless <code>--link</code>, which references them by path and needs an HTTP
server or a browser that allows <code>file://</code> font loads. See
<a href="../specimen/">Specimens</a>.</dd>

<dt><code>schema [face|collection|cli-output]</code></dt>
<dd>Print one of the JSON Schemas. <code>face</code> is the default.</dd>
</dl>

## FILTERS

Accepted by `list`, `families` and `facets`; all of them combine.

| Option | Meaning |
|---|---|
| `--family NAME` | exact typographic family name |
| `--variable[=BOOL]` | variable fonts only; `--variable=false` for static only |
| `--color[=BOOL]` | colour fonts |
| `--italic[=BOOL]` | italic or oblique faces |
| `--script CODE` | faces covering this script, ISO 15924: `Arab`, `Cyrl`, `Hani`, ... |
| `--license PREFIX` | SPDX identifier prefix: `OFL`, `Apache`, `LicenseRef-Proprietary` |
| `--weight RANGE` | CSS weight range, `600-900` |
| `--width RANGE` | width range in percent, `50-87` |
| `--vendor ID` | `OS/2` vendor id, `GOOG`, `ADBE` |
| `--tag TAG` | faces carrying this tag |
| `--collection NAME` | faces in this collection |
| `--active[=BOOL]` | faces activated or installed through fontina |
| `--activation STATE` | `session`, `user` or `installed` |
| `--container KIND` | `ttf`, `otf`, `ttc`, `woff` or `woff2` |
| `--under PREFIX` | faces whose path starts with this prefix |
| `-n, --limit N` | at most N results |

## OPTIONS

<dl>
<dt><code>--db PATH</code></dt>
<dd>Path to the index database. Defaults to <code>FONTINA_DB</code>, then the
platform data directory.</dd>
<dt><code>--json</code></dt>
<dd>Machine-readable output. Available on every reporting command.</dd>
<dt><code>-h, --help</code>, <code>-V, --version</code></dt>
<dd>As usual.</dd>
</dl>

## ENVIRONMENT

<dl>
<dt><code>FONTINA_DB</code></dt>
<dd>Path of the index database when <code>--db</code> is not given.</dd>
<dt><code>XDG_DATA_HOME</code>, <code>XDG_CONFIG_HOME</code></dt>
<dd>Honoured on Linux for the default index location and, in future, configuration.</dd>
</dl>

## FILES

See [The index and paths](../index-and-paths/).

## EXIT STATUS

`0` on success. `1` on any error, including a `check` that reports an error (or a
warning with `--strict`). Parse failures during `scan` are counted and reported but
do not fail the scan.

## EXAMPLES

Index the system fonts and your own, then look for a Cyrillic serif:

```
fontina scan --system ~/Fonts
fontina list --script Cyrl serif
```

Everything that is not under an open license:

```
fontina list --license LicenseRef
```

The bold weights of one family as CSS for a web project:

```
fontina css $(fontina list --family "Source Serif 4" --weight 600-900 --json | jq '.[].id') --url-prefix /fonts/
```

A specimen comparing three candidates:

```
fontina specimen 12 34 56 --text "Hamburgefonstiv" -o compare.html
```

Fail a build if any shipped font has an error:

```
fontina check dist/fonts/* --min warn
```

## SEE ALSO

`fc-list(1)`, `fc-query(1)`, fonttools' `ttx(1)`.
The [architecture decision records](../../adr/).
