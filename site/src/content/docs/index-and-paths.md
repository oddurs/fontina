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
```

The last column is where each value came from, because a setting whose origin you
cannot see is worse than no setting at all.

A missing file is not an error: with no file, fontina behaves exactly as it did
before there was one. A file that exists and does not parse is an error naming the
line, and so is a key nobody recognises, since a typo that is quietly ignored is a
setting that quietly does nothing.
