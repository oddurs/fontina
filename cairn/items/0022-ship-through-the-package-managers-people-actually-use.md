---
id: 22
title: Ship through the package managers people actually use
type: chore
status: backlog
milestone: m5-ship
created: 2026-09-05
updated: 2026-09-05
priority: p1
effort: m
crate: workspace
---

## Problem

Releases are archives, a `.deb` and an `.rpm`. Anyone on macOS or Windows, and
anyone on Arch, installs by hand. PLAN.md has carried this under M1 item 6 since
M1 shipped, and it blocks a 1.0 rather than any milestone.

## Proposal

A Homebrew formula, winget and Scoop manifests, and an AUR `PKGBUILD`, each fed by
the existing release workflow so a release publishes them rather than someone
remembering to.

## Acceptance criteria

- [ ] `brew install fontina` works on macOS, Apple silicon and Intel
- [ ] `winget install fontina` and `scoop install fontina` work
- [ ] the AUR package builds from the released source
- [ ] a release updates all four without manual steps
