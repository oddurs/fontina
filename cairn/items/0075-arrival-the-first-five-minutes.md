---
id: 75
key: arrival
title: Arrival — the first five minutes
type: milestone
status: backlog
created: 2026-09-15
updated: 2026-09-15
priority: p2
---

The first thing fontina says to a new person is the word **error**, followed by
thirty-four nouns in a comma-separated list, followed by "For more information, try
'--help'".

```
$ fontina
error: 'fontina' requires a subcommand but one was not provided
  [subcommands: scan, list, families, facets, tag, collection, source, activate, ...]
```

Nothing about that is broken. Every part of it is a reasonable decision that a
command-line program makes, and together they are a product with no opening move. A
person who has just installed a font manager knows exactly one thing — that they have
fonts — and the program's answer is a list of its own internals.

This milestone is the first five minutes. `fontina` alone does the obvious thing.
An empty index is a state the program has a plan for rather than an error it reports.
The words are the ones a person would use. Nobody has to read `--help` to begin.

The restraint that makes it a terminal program and not a wizard: it never asks a
question it can answer, it never takes an action it did not name first, and everything
it does on your behalf has the command that would have done it printed beside it.
