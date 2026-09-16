---
id: 88
title: Pin faces and set them side by side
type: feat
status: dropped
milestone: judgement
created: 2026-09-15
updated: 2026-09-15
priority: p2
effort: m
crate: ui
---

## Problem

0013 asks for this and is still in the backlog. It belongs here: pinning is how a person
holds a shortlist while they keep looking, and a shortlist is what choosing *is*.

The browser can show one face at a time. Every comparison therefore happens in the
person's memory, which is the one place the program cannot help.

## Proposal

Pin a face; pin another. The pinned set is visible while you keep browsing, comes with
you into `compare`, and becomes a collection in one key when you have decided.

## Acceptance criteria

- [ ] A face can be pinned and unpinned from anywhere in the browser.
- [ ] The pinned set survives navigation and is always visible.
- [ ] It feeds `compare` and becomes a collection without retyping anything.
- [ ] The command that would have made the collection is shown, per the one-verb rule.

## Notes

Supersedes the intent of 0013, which should be closed or pointed here when this starts.

## 2026-09-15

Dropped as a duplicate of 0013, which is older and says the same thing. The work is still wanted — it is 0013, now filed under `judgement`.
