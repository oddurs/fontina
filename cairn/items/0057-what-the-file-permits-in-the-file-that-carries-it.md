---
id: 57
title: What the file permits, in the file that carries it
type: feat
status: backlog
milestone: specimen
created: 2026-09-07
updated: 2026-09-07
priority: p1
effort: s
crate: core
---

## Problem

A specimen embeds font binaries by default and is a file people send to each other. That
makes it the single artefact fontina produces where licence facts matter most, and it is
where they are thinnest: the rail prints an SPDX identifier and the footer prints one
generic sentence.

`main.rs` already takes this seriously at the command line, warning about redistribution
and counting the licensed fonts a specimen would carry. The HTML the command writes says
none of it. The model holds far more: `LicenseInfo` carries the description, the URL, the
reserved font names and a `freedom` verdict, and `Os2Info` carries `fs_type` and a parsed
`EmbeddingRights`.

## Proposal

Give each sheet a licence section carrying the SPDX identifier, the freedom verdict from
`freedom::classify`, the copyright and trademark strings, the licence URL, the reserved
font names, and the embedding rights with the raw `fsType` beside them.

Report restrictions, never enforce them — `freedom.rs` says why, and this is the place the
rule is most easily broken. Embedding rights are data about the font, stated plainly, and
nothing on the page refuses to render, blur, or withhold a face on the strength of them.
A reserved font name is a fact the reader needs when they rename a derivative, not a
prohibition the specimen imposes.

Derive freedom on read, never store it, so the verdict tracks `freedom::FREE` rather than
the day the index was built.

## Acceptance criteria

- [ ] Each sheet carries SPDX, freedom, copyright, trademark, licence URL, reserved font
      names and embedding rights with the raw `fsType`.
- [ ] Nothing is withheld or degraded because of an embedding flag.
- [ ] Freedom is classified on read.
- [ ] A specimen written with `--link` says it references rather than carries the fonts.
