---
id: 80
title: '`fontina` on its own does the obvious thing'
type: feat
status: review
milestone: arrival
created: 2026-09-15
updated: 2026-09-15
priority: p0
effort: m
crate: cli
---

## Problem

```
$ fontina
error: 'fontina' requires a subcommand but one was not provided
  [subcommands: scan, list, families, facets, tag, collection, source, activate, ...]
```

The word `error`, then thirty-four nouns. The person has done nothing wrong; they typed
the name of the program they just installed. Every part of this is a reasonable default
from clap and together they are a product with no opening move.

## Proposal

A bare `fontina` is not an error. What it does depends on what it finds, and in every
case it ends with one obvious next thing:

- **No index.** Say so in a sentence, say where the fonts on this machine are
  (`dirs` already knows), and offer the one command that starts: `fontina scan --system`.
- **An index.** The shape of the library in four lines — how many faces, how many
  families, how many free, what is active — and the two or three things somebody
  actually does next.

Exit 0 in both cases. Nothing here is an error until something has gone wrong.

## Acceptance criteria

- [x] `fontina` with no arguments exits 0 and prints no clap error.
- [x] With an empty index it names the command that fills one.
- [x] With a full index it says what is in it without being asked for a table.
- [x] `--help` is unchanged and still lists everything.
- [x] A pipe gets the same words without colour, as everything else here does.

## Notes

The restraint: it prints a *suggestion*, never runs one. A program that scans your disk
because you typed its name is a program nobody trusts twice.
