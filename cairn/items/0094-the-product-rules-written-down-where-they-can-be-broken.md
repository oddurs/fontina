---
id: 94
title: The product rules, written down where they can be broken
type: docs
status: backlog
milestone: plain-sight
created: 2026-09-15
updated: 2026-09-15
priority: p1
effort: s
crate: workspace
---

## Problem

This roadmap asks for a lot of product judgement — defaults chosen on somebody's behalf,
commands grouped by intent, an opening screen with an opinion. Every one of those is a
place where a program can start deciding *for* a person rather than *with* them, and the
line between the two is not obvious from any single change.

`CLAUDE.md` holds the engineering rules and they are excellent. It holds nothing about
this.

## Proposal

Write the product rules down, in the same form: short, absolute, checkable in review.

The draft:

- A default may be chosen for somebody. A capability may not be taken away.
- Nothing the interface can do is unavailable to a script.
- The program never runs anything the person did not name.
- Every action says what it did and how to undo it.
- Facts, never adjectives.
- Report, never enforce.
- A fact the program does not have is a fact it does not print.
- Prose for a person, data for a machine, neither pretending to be the other.

## Acceptance criteria

- [ ] The rules are in `CLAUDE.md` beside the engineering ones.
- [ ] Each is stated so a reviewer can say a change breaks it.
- [ ] An ADR records why the product gained rules, since it is a structural choice.
