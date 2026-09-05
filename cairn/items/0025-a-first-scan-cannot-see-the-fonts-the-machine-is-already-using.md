---
id: 25
title: A first scan cannot see the fonts the machine is already using
type: feat
status: backlog
milestone: m5-ship
created: 2026-09-05
updated: 2026-09-05
priority: p1
effort: m
crate: cli
---

## Problem

A person who already manages fonts with Font Book — which is most people who
have a font library on a Mac — starts fontina with an index that cannot see any
of it. `scan ~/Fonts` indexes what they keep; the fonts that are actually
*active on the machine* live in `~/Library/Fonts`, and nothing points that out.

The cost showed up in `conflicts`. `MonoLisa-Regular` was sitting in
`~/Library/Fonts`, installed by Font Book, and `fontina conflicts` on the copy
in the library said "no conflicts". #136 made it say what it cannot see:

    no conflicts
    note: no operating-system font directory is in this index, so this cannot
    see fonts installed outside fontina; `fontina scan --system` puts them in

That is honest, and it is still a note about a command the person now has to
know to run, on an index they have already built once.

## Proposal

Make the operating system's font directories part of the index by default,
without pretending fontina manages what is in them.

The shape worth arguing about: `scan` adds the OS font directories as sources of
kind `system` the first time it runs on a fresh index, the same way `--system`
does now, and says so in one line. Everything else already distinguishes them —
`SourceKind::System` exists, activation state is per-face, and nothing fontina
does touches a file it did not put there.

The alternative is to leave it opt-in and make the note in #136 appear earlier —
at the end of the first `scan`, when the person is looking — rather than only
when a conflict question is asked.

Either way this is a first-run decision, which is why it belongs to M5 rather
than to a command.

## Available already

`scan --system`, `SourceKind::System`, `system_font_dirs()`, and the note from
#136. Nothing new is needed to see the fonts; the item is about when.
