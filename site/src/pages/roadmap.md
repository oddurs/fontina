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

## M4, ask

Everything the index knows, askable. fontina has read the whole of a font since M0 —
every variable axis and its range, the language systems each script declares, how many
codepoints of a script a face actually covers — and stores it all. The filters lag
behind it in three places: a variable font is matched on its default instance, so a
face whose weight axis spans 200 to 800 is not found by a search for 400; there is no
way to ask for a language at all; and a script filter cannot ask for two scripts at
once or for how much of one a face covers.

Spacing is the fourth: whether a face is monospaced is read from the font and stored,
and cannot be filtered or counted, which on a working developer's library is the most
useful single division there is.

None of that needs a new parse or a rescan. The data is already in the index; only the
questions are missing.

The fifth gap is a different kind. fontina groups faces by what the font declares — the
family in its name table — and by what a person asserts, with tags and collections. It
derives no grouping from what a face actually is, so where a declared family is wrong
there is nothing to fall back on. A real library of 149 fonts reports twenty families and
holds about eight typefaces. The answer is not a second kind of family the index invents
and stores, which would mean encoding somebody else's naming convention as fact; it is a
question you can ask of one face — what else here covers nearly the same characters — and
answer from evidence the index already has.

## M5, ship

Four milestones in, you can install fontina with a package manager on Linux and with
none on macOS or Windows. The release builds a `.deb` and an `.rpm`; everywhere else
there is an archive to download and a binary to move onto your `PATH` by hand. Every
desktop is meant to be first class, and on the evidence of how you install it, that is
not yet true.

So: a Homebrew tap, a Scoop bucket, a winget manifest, an AUR package — kept current by
the release itself rather than by somebody remembering, and each one installed for real
in a clean machine and tested there, the way the `.deb` and the `.rpm` already are.

Two performance budgets are written down and not measured, because both need a real
terminal to measure: how much memory the browser holds at rest, and how long it takes to
repaint. A budget nothing measures is a wish with a table row, so either they get a
harness or they stop being called budgets.

Nothing here makes fontina do anything new with a font. It makes fontina something you
can install, which after four milestones of features is the question worth asking.

## Never

Font editing. Format conversion or subsetting. Cloud synchronisation. Accounts.
Telemetry. An Electron shell.
