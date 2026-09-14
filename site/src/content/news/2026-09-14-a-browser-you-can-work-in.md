---
title: "0.2.0, and a browser you can work in"
description: "Multi-select, undo, a command palette and filters you build while watching the count; questions about language, spacing and variable ranges; and a WOFF2 decoder that no longer panics."
date: 2026-09-14
---

fontina 0.2.0 is out, ten days after 0.1.1: forty-two features, thirty-eight
fixes and five performance changes. Most of it is the browser, a much larger set
of questions you can ask of your library, and a long tail of hardening that came
out of reviewing the last two milestones rather than out of bug reports.

## The browser

`fontina ui` was a way of looking at the index. It is now a way of working on it.

Mark many faces and act on them once. Take back the last thing that changed the
index, whatever it was. Open a command palette over every command the program has,
listed where you are rather than in a manual. Build a filter while watching the
count move, using the same flags the command line takes — so the row at the bottom
of the browser is the command that would have done the same thing, and copying it
is how you script what you just did by hand.

Three of the additions answer questions rather than run commands. **What else in
this library is nearly this font** ranks the rest of your faces against the one you
are looking at and shows the numbers behind the ranking. **Who can set this text**
takes a paste and tells you which faces cover it. **What pairs with this** reads
what a font says it is — its classification, not a guess from the name — and ranks
on that.

The layout follows the terminal now: as many panes as the window can carry and no
more, one palette resolved against what the terminal can actually show, one box,
one bar, one left edge.

It is also measurably quicker. A pane builds the rows it draws rather than the
rows it holds, the listing queries left the thread that draws, and there is a
repaint budget in `scripts/bench` that fails the build if a keystroke stops being
instant on ten thousand faces. The preview cache now says what it costs as well as
what it saves.

Two of the four browser milestones are finished and two are not: what it looks like
is three quarters done, and the comparison work — pinning faces side by side —
is still ahead.

## Better questions

The index has held all of this since M0. The filters had not caught up.

- `--lang` and a language facet, with a font's two different claims about a
  language kept apart rather than merged.
- `--mono` and `--proportional`, and a spacing facet. On a developer's library this
  is the most useful single division there is, and it was read from every font and
  then thrown away.
- `--script` repeats, so you can ask for two scripts at once, and `--script-min`
  asks how much of one a face actually covers.
- A variable font is found at every weight it spans, not only at its default. A
  face whose weight axis runs 200 to 800 now answers a search for 400.

## Handing fonts over

`collection export --bundle` produces a collection you can give to somebody else,
fonts and all. `fontina tag sync` reads and writes the operating system's own file
tags, in one direction at a time so it cannot quietly lose anyone's work. Every
command that takes faces reads them from standard input too, so programs can pipe
in. A specimen now says how many licensed fonts it is carrying, and is laid out so
the type can be judged rather than merely displayed.

## Hardening

Most of this was found by looking rather than by breaking.

The WOFF2 decoder panicked on a file it was handed; it is contained now. An empty
`cmap` no longer hides the rest of a font's report. Two processes creating the same
index at the same instant no longer race. The watcher keeps up with what people
actually do to a font directory. The rasteriser clipped a preview instead of
smearing it, and stopped overflowing. A scan says which files it walked past and
why, rather than silently ignoring them.

The login agent that puts your activations back after a reboot touches only files
fontina wrote, which is the sort of thing worth checking before it ships rather
than after.

## Packaging

Manifests for Homebrew, Scoop, winget and the AUR are in the repository. They are
not published yet — that needs credentials only a person can create — but the
release now also builds an arm64 `.deb` and an aarch64 `.rpm` alongside the x86_64
ones, and every package is installed for real with `apt` and `dnf` in a clean
Debian, Ubuntu and Fedora before it goes out.

[Download](../../download/) has the archives and how to check them: a SHA-256 for
every file, a SLSA build provenance attestation that proves the archive came from
the release workflow rather than somebody's laptop, and an SPDX bill of materials.

## Why 0.2.0 and not 0.1.2

Because forty-two features is a minor release. release-please was configured to
move the patch number for a feature until 1.0, which is allowed before 1.0 and
still wrong: somebody running 0.1.1 who saw 0.1.2 would have read it as bug fixes.
That setting is off, here and from now on.

Pre-1.0 still means what it said: command output, the JSON schemas and the
health-check identifiers may change, and the
[changelog](https://github.com/oddurs/fontina/blob/main/CHANGELOG.md) says when one
does. [Releases](../../releases/) lists every version.
