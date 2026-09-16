---
title: The index and paths
description: "where the database lives, what is in it, migrations, and how to move or discard it."
order: 9
---

## Location

The index is one SQLite file. Its location, in order of precedence:

1. `--db PATH` on the command line;
2. the `FONTINA_DB` environment variable;
3. the platform data directory:

| Platform | Default |
|---|---|
| Linux and BSDs | `$XDG_DATA_HOME/fontina/index.db`, normally `~/.local/share/fontina/index.db` |
| macOS | `~/Library/Application Support/fontina/index.db` |
| Windows | `%APPDATA%\fontina\index.db` |

These come from the `directories` crate, which follows the XDG Base Directory
specification and the platform conventions. `fontina stats` prints the path in use.

Keep a second index for experiments by setting `FONTINA_DB` in the shell; nothing
else needs to change.

## What is in it

The database is in WAL mode with an FTS5 full-text table over names and designers.
The tables:

| Table | Holds |
|---|---|
| `files` | path, size, modification time, BLAKE3 hash, container |
| `faces` | one row per face: the indexed columns for filtering, and the full metadata JSON |
| `axes`, `instances`, `features` | variable axes, named instances, layout features per face |
| `tags`, `face_tags` | tags and their assignment |
| `collections`, `collection_faces` | ordered collections |
| `sources` | registered directories and whether they are watched |
| `activations` | activation state, scope and timestamp per face |
| `faces_fts` | the full-text index |

It is an ordinary SQLite database and any client can read it:

```
sqlite3 ~/.local/share/fontina/index.db 'select family, count(*) from faces group by 1 order by 2 desc limit 10'
```

Write to it only through fontina.

## Migrations

The schema version is `PRAGMA user_version`. fontina applies any pending migrations
when it opens the database, in order, inside a transaction. Migrations are
append-only: an applied one is never edited. A migration that needs data fontina
already extracted reads it from the stored metadata JSON, so adding a new indexed
column never needs a rescan. A new version of fontina that extracts *new* metadata
does need `scan --force` to see it in older faces; the changelog says when.

An older fontina refuses to open a newer database rather than guess.

## Moving, backing up, discarding

Copy the single file (with its `-wal` and `-shm` siblings if present, or after
fontina has closed cleanly). Paths inside are absolute, so an index copied to another
machine will list faces whose files are not there until you `scan --prune` and scan
the new locations. A collection export is the portable form; see
[Concepts](../concepts/).

Deleting the file loses tags, collections, source registrations and activation
state. Everything else is recreated by a scan.

## Configuration

One TOML file, in the platform configuration directory:

| Platform | Default |
|---|---|
| GNU/Linux and the BSDs | `$XDG_CONFIG_HOME/fontina/config.toml`, normally `~/.config/fontina/config.toml` |
| macOS | `~/Library/Application Support/fontina/config.toml` |
| Windows | `%APPDATA%\fontina\config.toml` |

`FONTINA_CONFIG` names a different one. `fontina config --path` prints whichever is
in force, and `fontina config --example` prints a commented file to save there.

It holds **defaults only**. Every setting in it is one a flag can override, so
nothing in the file can make a command do something its arguments do not say, and
you can read somebody else's config and still predict what their commands do.
Precedence runs: the flag, then the environment, then this file, then fontina's own
default.

```
$ fontina config
~/.config/fontina/config.toml

index.db           ~/.local/share/fontina/index.db               config
scan.sources       ~/Fonts                                       config
scan.system        false                                         default
preview.text       Sphinx of black quartz, judge my vow          config
preview.size       48                                            default
preview.protocol   auto                                          default
preview.fg         (the terminal's foreground)                   default
preview.bg         (the terminal's background)                   default
colours.head       bold                                          default
colours.dim        bright-black                                  default
colours.accent     bold magenta                                  config
colours.good       green                                         default
colours.warn       yellow                                        default
colours.bad        red                                           default
```

The last column is where each value came from, because a setting whose origin you
cannot see is worse than no setting at all.

A missing file is not an error: with no file, fontina behaves exactly as it did
before there was one. A file that exists and does not parse is an error naming the
line, and so is a key nobody recognises, since a typo that is quietly ignored is a
setting that quietly does nothing.

### Colours

`[colours]` — or `[colors]`, both are read — says what each of the six roles looks
like, on the command line and in the browser alike. They share one scheme, so there
is no second place to change.

```toml
[colours]
accent = "bold magenta"
dim = "blue"
```

Name a role and that role changes; the five you did not name keep what they had.
There is no need to restate a colour you are happy with, and no way for a file to
freeze the others at whatever they were the day it was written.

A value is one of the sixteen terminal colours —

```
black  red  green  yellow  blue  magenta  cyan  white
bright-black  bright-red  bright-green  bright-yellow
bright-blue  bright-magenta  bright-cyan  bright-white
```

— optionally with `bold` or `reverse` (`bold cyan`, in either order), or the word
`none`.

Sixteen and no more, deliberately. These are the colours your terminal theme already
defines, so an accent of `cyan` is *your* cyan; a hex value chosen here would be the
one colour on the screen that ignores the theme you picked. It is also why there is
no background setting: a role paints its own text, and the ground stays yours.

The six roles are `head` (a column heading), `dim` (labels, units, the directory part
of a path), `accent` (the thing being pointed at), `good`, `warn` and `bad`.

Two things this will refuse. A value that is not a colour, naming what it would have
taken. And a scheme where two roles end up looking identical — colour here carries
hierarchy and never meaning, and that only works when you can see the hierarchy. A
file that paints `good` and `bad` the same green is a mistake worth hearing about at
startup rather than discovering in a table of health checks.

`NO_COLOR` still turns all of it off. The scheme says what colour *means*, not whether
to use any.
