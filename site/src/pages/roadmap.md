---
layout: ../layouts/Page.astro
title: Roadmap
description: "What is done, what is next, and what fontina will never do."
source: site/src/pages/roadmap.md
---

The authoritative plan is [`PLAN.md`](https://github.com/oddurs/fontina/blob/main/PLAN.md)
in the repository. This page is the summary. Milestones are named for what they let
you do, not for versions; the version number is set by the release process from the
commit history.

## M0, foundations: done, 2026-09-03

The library and the command line. Every container format (TTF, OTF, TTC, WOFF,
WOFF2), the full metadata model, the SQLite index with full-text search, duplicate
detection across containers, health checks, CSS export, the HTML specimen, coverage
queries, JSON output against published schemas, and a CI matrix on Linux, macOS and
Windows with license and advisory auditing. Decision records for fontations,
SQLite, the deferred desktop shell, the license and WOFF decoding.

## M1, manage: done, 2026-09-04

The font manager proper, in the terminal. Re-scoped on 2026-09-03 from a desktop
application to command line plus terminal browser.

1. **Organise.** Tags, collections with JSON import and export, sources, family
   grouping, facet counts, richer filters.
2. **Activate.** Native `activate`, `deactivate`, `install` and `uninstall` on Linux,
   macOS and Windows; conflict detection with `--replace`; activation state in the
   index; `restore` for login agents.
3. **Watch.** `source add` scans immediately; `watch` follows every watched source
   with debounced incremental rescans.
4. **Preview.** Shaped, rasterised previews in the terminal over kitty graphics,
   iTerm2 images, sixel or half-block text, with axis coordinates and feature
   toggles.
5. **Browse.** `fontina ui`: search, facets, families and faces, details, previews,
   tagging and activation from the keyboard.
6. **Ship.** Completions and man pages in the archives; `.deb` and `.rpm` from the
   release workflow.

Still to do before 1.0: a Homebrew formula, winget and Scoop manifests, an AUR
`PKGBUILD`, and the rename.

## M2, typography

In the browser: axis sliders with named-instance snapping, feature toggles, a glyph
map by block with codepoint search, compare and waterfall views, a license viewer.
`check` grows toward fontbakery parity where it is cheap; identifiers never change.
Optional login-agent packaging (systemd user unit, LaunchAgent, Run key), off by
default. An optional offline Google Fonts index, separately packaged, opt-in.

## M3, ecosystem and shells

Team sharing through plain folders (collection JSON with relative paths over
Syncthing, git or any synced directory), tag synchronisation with Finder tags and
Windows file properties, and a plugin surface that is only the command line and
JSON. A graphical shell, as one more client of the core, only if the terminal browser
leaves a real gap; it would have to meet the same budgets and follow each platform's
own design conventions, Linux first.

## Never

Font editing. Format conversion or subsetting. Cloud synchronisation. Accounts.
Telemetry. An Electron shell.
