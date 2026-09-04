---
layout: ../layouts/Page.astro
title: Reporting bugs
description: "Where and how to report a bug in unifont, and what to include."
source: site/src/pages/bugs.md
---

Bugs are tracked at
[github.com/oddurs/unifont/issues](https://github.com/oddurs/unifont/issues). Search
before filing; the issue templates ask for what is listed here.

If the bug is a crash, a hang, or wrong memory behaviour while reading a font file,
it is a security bug. Do not file it publicly; follow the [security policy](../security/).

## What to include

1. The output of `unifont --version` and your operating system and version.
2. The exact command and its complete output. Add `--json` if the human-readable
   output hides something.
3. For a parsing or metadata problem: the font file, if its license allows sharing it.
   If not, the output of `unifont info --json <file>` and the font's origin.
4. What you expected instead, and why. A reference to the OpenType specification or
   to what another tool reports is ideal.

## Wrong metadata is a bug

If unifont reports a family, weight, script coverage, license or feature that is not
what the font says, that is a bug even when the font itself is unusual. Please report
it. If a fixture under an open license reproduces it, say which.

## Missing fontations coverage

Parsing goes through [fontations](https://github.com/googlefonts/fontations). If
something is missing because fontations does not expose it, the report should say so;
the fix is upstream, and unifont will not hand-parse around it.

## Feature requests

Welcome, in the same tracker. Read the [roadmap](../roadmap/) first, including the list
of things unifont will never do.
