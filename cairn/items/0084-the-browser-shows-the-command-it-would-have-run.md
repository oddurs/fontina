---
id: 84
title: The browser shows the command it would have run
type: feat
status: backlog
milestone: one-verb
created: 2026-09-15
updated: 2026-09-15
priority: p0
effort: m
crate: ui
---

## Problem

The browser and the command line are one program with two vocabularies. Somebody who
learns `fontina ui` learns nothing about `fontina activate`, and somebody who lives on the
command line has no reason to open the browser.

The site already claims this is solved — *"the row at the bottom is the command line that
would show the same thing"* — and it is true of filters and searches. It is not true of
actions.

## Proposal

Every action the browser takes prints the command that would have done it, in the same
place the filter line already appears, and yanking it to the clipboard is one key.

Activate a face, and see `fontina activate 412`. Tag a selection, and see
`fontina tag add draft 412 87 903`. The browser becomes the way people learn the command
line rather than an alternative to it.

## Acceptance criteria

- [ ] Every index-changing action shows its equivalent command.
- [ ] The command shown, run in a shell, does the same thing — a test asserts this by
      running it rather than by comparing strings.
- [ ] One key copies it.
- [ ] Nothing is shown for a navigation that changes nothing.
