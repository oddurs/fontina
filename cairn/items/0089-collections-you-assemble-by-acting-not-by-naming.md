---
id: 89
title: Collections you assemble by acting, not by naming
type: feat
status: backlog
milestone: the-shelf
created: 2026-09-15
updated: 2026-09-15
priority: p1
effort: m
crate: cli
---

## Problem

Making a collection today: `collection create`, then `collection add` with a list of ids
you got from somewhere else. That is the shape of a database, and it means the collection
is made *after* the decision, by retyping it.

The decision happens while looking. The program should be collecting it as it is made.

## Proposal

Acting is how a set is built. Mark faces as you go; the marked set is a collection
waiting for a name. Name it when you have one, or never — an unnamed working set is a
real thing and should not need ceremony.

`collection create` and `collection add` stay exactly as they are, because a script needs
them.

## Acceptance criteria

- [ ] A set can be assembled by marking, without naming it first.
- [ ] Naming it later loses nothing.
- [ ] The existing subcommands are untouched.
- [ ] The command that would have built the same set is shown.
