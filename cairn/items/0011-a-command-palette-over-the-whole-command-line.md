---
id: 11
title: A command palette over the whole command line
type: feat
status: backlog
milestone: tui-depth
created: 2026-09-05
updated: 2026-09-05
priority: p1
effort: m
crate: ui
---

## Problem

The browser reaches perhaps a third of what the CLI can do. The rest is learned
from the manual and typed in another window, and the two surfaces drift because
nothing forces them together.

## Proposal

`:` opens a palette listing every command `fontina --help` lists, with its
arguments and the same help text, filtered as you type. Running one applies it to
the selection or the face under the cursor.

The palette is generated from the clap command tree rather than written out, so a
new subcommand appears in it without anyone remembering to add it.

## Acceptance criteria

- [ ] built from clap's command tree; a new subcommand needs no change here
- [ ] the help shown is the help `fontina help <command>` prints
- [ ] a command that would change the filesystem asks first
- [ ] a test that every subcommand is reachable, so drift fails the build
