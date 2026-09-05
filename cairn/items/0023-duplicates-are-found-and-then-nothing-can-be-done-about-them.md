---
id: 23
title: Duplicates are found and then nothing can be done about them
type: feat
status: backlog
milestone: unfiled
created: 2026-09-05
updated: 2026-09-05
priority: p1
effort: s
crate: cli
---

## Problem

`fontina dupes` answers the question and then stops. On a real library of 1,998
fonts it reports 61 groups holding 122 faces — an entire brand directory that
duplicates the master one, file for file:

    identical outlines and names (0837f7d9aa7f10d3):
      [1196] National Light  /Users/…/Fonts/Master Library/National-Light.otf
      [1891] National Light  /Users/…/Fonts/Outreach Brand/National-Light.otf

That is the useful half. The other half is that a person now wants to keep one
of each and be rid of the rest, and fontina offers nothing: no flag, no listing
shaped for a pipe, no way to say which copy is the one to keep. The obvious
next move is `dupes | grep | xargs rm`, which is a person hand-writing a
deletion loop over their font library from a listing that was designed to be
read rather than parsed.

`--json` exists and carries the groups, so the data is there. What is missing is
a stance on what to do with it.

## Proposal

Decide what fontina is willing to do here, then do that much.

The safe shape, and probably the right one: `dupes --keep <RULE>` prints the
paths of the copies that would go, one per line, and nothing else — so a person
can read them, then pipe them wherever they like. Rules worth having are
`first` (the earliest path, which is stable), `shortest-path`, and `--under
<DIR>` for "prefer the copy in this directory". Deleting is theirs to do.

A stronger version — `dupes --remove` — is a font manager deleting a person's
files, which is a different promise from anything fontina makes today and wants
its own argument before it is written.

## Available already

`Index::duplicates` groups by outline hash and by PostScript name and is fast
(61 groups out of 1,998 faces in 14 ms). `DuplicateGroup` is in
`schemas/cli-output.json`. This is a view and a decision, not a query.
