---
id: 83
title: A spine of verbs, with the rest reachable rather than in the way
type: feat
status: backlog
milestone: one-verb
created: 2026-09-15
updated: 2026-09-15
priority: p0
effort: l
crate: cli
---

## Problem

Thirty-four top-level commands, presented flat and alphabetically-ish. `scan` and
`schema` sit at the same level; so do `activate` (what people came for) and `completions`
(what a shell install script runs once).

Nothing should be removed — the command line is the product and the project's own rule is
that nothing is locked away from it. But a flat list of thirty-four is a list nobody
reads, and the ones that matter are buried among the ones that do not.

## Proposal

Group the help by what a person is trying to do, with a short spine first:

```
  Find      list, families, info
  Use       activate, install, deactivate, uninstall
  Judge     compare, related, check
  Keep      tag, collection, source
  Export    css, specimen, preview
  Set up    scan, watch, config, dirs
  Machinery completions, man, schema, agent, restore
```

The names stay; only the presentation changes, so no script breaks. `fontina help <group>`
lists a group.

## Acceptance criteria

- [ ] `fontina --help` groups commands under headings a person would recognise.
- [ ] Every existing command still runs, spelled exactly as it is today.
- [ ] `the_manual_matches_the_help` still passes, or is updated deliberately.
- [ ] The grouping lives in one place, not spread across thirty-four doc comments.

## Notes

Groups, not a new dispatch layer. The temptation is `fontina font activate`; that is a
second vocabulary on top of a working one and it would break every script in the world
for tidiness.
