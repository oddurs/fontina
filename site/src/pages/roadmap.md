---
layout: ../layouts/Page.astro
title: Roadmap
description: "What exists, what is next, and what unifont will never do."
source: site/src/pages/roadmap.md
---

The authoritative plan is [`PLAN.md`](https://github.com/oddurs/unifont/blob/main/PLAN.md)
in the repository. This page is the summary.

## Done: foundations

The library and the command line. Every container format (TTF, OTF, TTC, WOFF, WOFF2),
the full metadata model, the SQLite index with full-text search, duplicate detection
across containers, JSON output against published schemas, and a CI matrix on Linux,
macOS and Windows with license and advisory auditing.

Ahead of schedule, the "pro typography" features shipped through the core and the
command line: health checks, text coverage queries, glyph maps by Unicode block, the
license and embedding report with reserved font names, and the HTML specimen with axis
sliders, feature toggles, a waterfall, a glyph map and side-by-side comparison. The
desktop application will reuse those modules rather than reimplement them.

Also done: tags, collections with JSON import and export, registered source
directories, activation state in the index, facet counts and family grouping.

## Next: activation and the terminal

Per-user font activation on all three platforms through one trait with three
implementations: symlinks and a fontconfig fragment on Linux, Core Text registration
on macOS, per-user font registration on Windows. Session and persistent scopes.
Conflict detection on PostScript name and family plus style before activating. A
foreground `watch` command that follows registered sources for scripts and systemd
user units. Terminal previews and a full-screen terminal interface.

## Later: the desktop application

A Tauri 2 shell with a Svelte 5 frontend, following each platform's own conventions:
a virtualised library grid with truthful previews, watched folders, facets, activation
and conflict warnings, family and duplicate views. Packaged as Flatpak, AppImage, dmg
with a Homebrew cask, and MSIX with winget. Signed and notarised. Hard budgets: at
most 15 MB installer, 300 ms to first paint with five thousand faces, 80 MB idle.

## After 1.0: ecosystem

Team sharing through plain folders (collection JSON and relative paths over Syncthing,
git or any synced directory), tag synchronisation with Finder tags and Windows file
properties, and a plugin surface that is only the command line and JSON. No
in-process plugins.

## Never

Font editing. Format conversion or subsetting. Cloud synchronisation. Accounts.
Telemetry.
