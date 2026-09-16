---
id: 92
title: Everything fontina did, and how to undo it
type: feat
status: backlog
milestone: plain-sight
created: 2026-09-15
updated: 2026-09-15
priority: p0
effort: m
crate: cli
---

## Problem

`activations` lists what is active now. It does not say when, or what it replaced, or
what the state was before — and "what did this program do to my system" is the question
a font manager has to be able to answer, because the answer is the reason to trust it.

The browser has undo for the last index change (0010). The system side has nothing.

## Proposal

A log of every action that touched the system or the index: what, when, what it replaced,
and the command that undoes it. Plain file, readable without this program.

Undo is then not a feature but a consequence: the log already holds the inverse.

## Acceptance criteria

- [ ] Every activation, installation, tag and collection change is recorded.
- [ ] Each entry carries the command that reverses it.
- [ ] The log is a plain file in the data directory, readable without fontina.
- [ ] It is bounded, and says what it dropped rather than growing without limit.

## Notes

This is the audit trail the whole Apple half is allowed to exist *because of*. Decide the
default; show the reasoning; never hide the lever.
