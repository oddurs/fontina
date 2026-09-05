---
id: 15
title: Paste text, see who can set it
type: feat
status: backlog
milestone: tui-discovery
created: 2026-09-05
updated: 2026-09-05
priority: p1
effort: m
crate: ui
---

## Problem

`fontina covering` answers the question a designer actually has — can anything I
own set this sentence — and the browser cannot ask it. Coverage is shown per face,
as scripts and counts, which answers a question about fonts rather than about text.

## Proposal

A text field. Paste or type anything; the list narrows to faces that can set it,
and a face that nearly can shows which codepoints it is missing.

## Acceptance criteria

- [ ] backed by `Index::covering`, not by a new coverage path
- [ ] a face missing a handful of codepoints is offered with them named
- [ ] a mixed-script string reports per script rather than only pass or fail
- [ ] the text survives switching panes, because retyping it is the whole friction
