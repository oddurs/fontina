---
id: 91
title: A shelf that opens without fontina
type: feat
status: backlog
milestone: the-shelf
created: 2026-09-15
updated: 2026-09-15
priority: p2
effort: m
crate: core
---

## Problem

`collection export --bundle` already produces a directory of fonts and a JSON manifest,
which is most of the way there. What it does not produce is something a person who does
not have fontina can open and understand.

The freedom clause is load-bearing here: a library you can only read with one program is
not yours, whatever the licence on the program says.

## Proposal

A bundle carries a specimen. Open the directory, open the HTML, see the typefaces, read
the licences, know what you have. The manifest stays exactly as it is for the machine.

`specimen.rs` already renders a self-contained page with no external requests, so this is
mostly wiring — and it is the point at which "your data in formats you own" stops being
a promise in a mission statement and becomes a file somebody can double-click.

## Acceptance criteria

- [ ] A bundle contains a specimen of its own faces, self-contained.
- [ ] It opens with no network and no fontina.
- [ ] The manifest is unchanged.
- [ ] `--link` bundles say why they cannot do this rather than shipping a broken page.
