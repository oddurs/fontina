---
id: 85
title: Say it in the words a person would use
type: chore
status: review
milestone: one-verb
created: 2026-09-15
updated: 2026-09-15
priority: p1
effort: m
crate: cli
---

## Problem

`facets` is a database word. `covers` is a verb with no object. `dupes` is an
abbreviation of a word nobody abbreviates. `--script-min` is a query language wearing a
flag's clothes, and `--lang-source` needs a paragraph before it means anything.

The names are what a person has to think in, and several of them are the schema's names
rather than theirs.

## Proposal

Go through every command and flag and ask one question: would somebody who knows fonts
and not this program guess it? Where the answer is no, add the name they would guess as
the primary and keep the current one as an alias, for ever.

Aliases rather than renames, because the rule that a script written against 1.0 runs
against 3.0 is worth more than tidiness. The manual leads with the new name.

## Acceptance criteria

- [x] Every renamed command and flag keeps its old spelling working.
- [x] A test asserts the aliases resolve, so removing one is a deliberate act.
- [x] `the_manual_matches_the_help` covers both spellings.
- [x] The changelog names each pair.
