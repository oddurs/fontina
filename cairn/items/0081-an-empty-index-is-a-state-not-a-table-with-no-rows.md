---
id: 81
title: An empty index is a state, not a table with no rows
type: feat
status: review
milestone: arrival
created: 2026-09-15
updated: 2026-09-15
priority: p1
effort: m
crate: cli
---

## Problem

Every command that reads the index treats empty as "zero results" and prints a header
with nothing under it, or nothing at all. `fontina list` on a fresh install is a blank
line. That is correct and it is not an answer.

## Proposal

Empty is a state the program has a plan for. Each command that can be run against an
empty index says, in one line, that there is nothing indexed yet and what fills it —
in the words of *that* command, not a generic banner.

`list` says it lists what has been scanned. `activate` says it works on faces in the
index and a path also works. `check` already does the right thing after #233 and is the
model.

## Acceptance criteria

- [x] No command prints an empty table with a header and no rows.
- [x] Each says what to do, naming the command.
- [x] Exit code is unchanged: an empty index is not a failure.
- [x] `--json` is unaffected; a machine gets `[]`, which is the true answer.

## Notes

The `--json` exemption is the whole shape of this: prose for a person, data for a
program, and neither pretending to be the other.
