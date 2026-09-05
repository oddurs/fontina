---
id: 10
title: Undo anything that changed the index
type: feat
status: backlog
milestone: tui-depth
created: 2026-09-05
updated: 2026-09-05
priority: p0
effort: l
crate: ui
---

## Problem

Activation, tagging and collection edits are immediate and permanent. With the
selection set above, one keystroke will change a hundred rows. A tool that makes a
hundred changes at once and offers no way back is a tool people use carefully
rather than fluently, which is the opposite of the point.

## Proposal

An undo stack of inverse operations, in memory, for the session. `u` undoes,
`ctrl-r` redoes, and the status line names what it just undid.

Scoped deliberately: this undoes index state, not the filesystem. Uninstall
removes a file the tool copied, and putting it back is `install`, so it is
undoable; anything that could not be exactly reversed is not offered.

## Acceptance criteria

- [ ] every mutating action pushes an inverse
- [ ] undo of a batch is one undo, not a hundred
- [ ] what cannot be reversed exactly is refused rather than half-undone
- [ ] the status line names the action, not "undone"
