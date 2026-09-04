---
layout: ../layouts/Page.astro
title: fontina mission statement
description: "What fontina is for, the nine promises everything in the roadmap serves, and what it will never do."
source: site/src/pages/mission.md
---

fontina is the font manager for the people: a single small binary that indexes,
searches, previews, activates and organises fonts on Linux, macOS and Windows, keeps
everything in plain files you own, never phones home, and will still work in twenty
years.

## The promises

Everything in the [roadmap](../roadmap/) serves one of these. They are checked in
continuous integration or refused in review; they are not aspirations.

<dl>
<dt>Terminal first</dt>
<dd>The command line is the product. Every capability exists as a command that prints
text or JSON before it exists anywhere else. The browser (<code>fontina ui</code>) and
any later graphical shell are clients of the same core and add no capabilities of
their own.</dd>
<dt>Your data stays yours, in formats you can read</dt>
<dd>One SQLite file for the index, JSON with a published schema for every export, TOML
for configuration, fontconfig XML on Linux. Nothing that needs fontina to open.
Delete the binary and your fonts, folders and operating-system font registrations
are exactly as you left them; the index rebuilds from disk in seconds.</dd>
<dt>Nothing leaves the machine</dt>
<dd>No network calls, no accounts, no update checks, no crash reporting, no telemetry,
ever. Online catalogs, if they ever come, are opt-in, separately packaged and off by
default.</dd>
<dt>Light</dt>
<dd>A static binary with no runtime, no bundled browser, no daemon unless you ask for
one. Hard budgets on size, start time and memory are enforced in continuous
integration.</dd>
<dt>Fast</dt>
<dd>Ten thousand files indexed in seconds, any query in milliseconds, the browser
repaints within a frame. Parallel scans, an indexed database, no busywork at
startup.</dd>
<dt>Durable</dt>
<dd>Append-only migrations, stable command output, stable check identifiers, semantic
versioning. A script written against 1.0 runs against 3.0. Dependencies are few,
audited and replaceable; the only hand-written codec is WOFF 1.0.</dd>
<dt>Correct</dt>
<dd>Parsing by Google's fontations, the code Chrome and Skia use. Shaping by HarfBuzz's
Rust port on the same stack. Families from name IDs 16 and 17 and <code>STAT</code>,
never from file names. Standards for every artefact: OpenType, WOFF, CSS Fonts Level
4, SPDX, XDG, fontconfig, JSON Schema, TOML.</dd>
<dt>Every desktop is first class, Linux first</dt>
<dd>Per-user install and temporary activation implemented natively on all three
systems, no elevation, never a write to a system font directory. Linux is the
reference platform: fontconfig integration, distribution packages, and the
terminals people actually use are tested first.</dd>
<dt>Free software, no strings</dt>
<dd>GPL-3.0-or-later, for the library, the command line and the browser, so that
nobody downstream can take these freedoms away from the next person. Contributions
are accepted under the same terms, with no contributor agreement and no copyright
assignment. The manual is under the GNU Free Documentation License. A public
roadmap, a decision record for every structural choice, reproducible release builds
with provenance and a bill of materials. fontina answers the same question about
your fonts: <code>list --free</code> shows the ones you may study, modify and pass
on.</dd>
</dl>

## Non-goals

Font editing. Format conversion or subsetting; that is fonttools' job. Cloud
synchronisation. Accounts. Telemetry. In-process plugins. An Electron shell.

## How decisions are made

There is one maintainer for now. Decisions worth arguing about are written down as
[architecture decision records](../adr/) before the code lands, and a record is never
edited after acceptance; a change of mind is a new record. The
[contributing](../contributing/) page says how to take part.

## The name

fontina: one letter from the thing it manages, free on every package registry, and a
pun of the kind GNU names are usually built from. The working name until 2026-09-04
was "unifont", which collided with GNU Unifont, the bitmap font.
