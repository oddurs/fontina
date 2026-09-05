---
id: 9
title: Select many, act once
type: feat
status: backlog
milestone: tui-depth
created: 2026-09-05
updated: 2026-09-05
priority: p0
effort: m
crate: ui
---

## Problem

Every action in the browser applies to the face under the cursor. Activating a
family of sixteen weights means sixteen keystrokes on the same key, and tagging a
foundry's worth of faces is not something anyone will do twice.

## Proposal

A selection set. Space toggles the face under the cursor, `v` starts a range, `A`
selects everything the current filter matches. Every action that takes a face takes
the selection instead when there is one, and says how many it touched.

## Acceptance criteria

- [ ] activate, deactivate, install, uninstall, tag and collection all act on a selection
- [ ] the count is visible while a selection exists, and clearing it is one key
- [ ] a filter change keeps the selection to faces that still match, and says so
- [ ] a partial failure reports which faces failed and leaves the rest applied
