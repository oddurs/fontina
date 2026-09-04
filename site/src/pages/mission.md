---
layout: ../layouts/Page.astro
title: unifont mission statement
description: "What unifont is for, the principles that are enforced rather than aspired to, and what it will never do."
source: site/src/pages/mission.md
---

unifont exists to give people a font manager that is correct, small, private and
free, on every desktop, built from open standards so that nothing it produces
locks anyone in.

## Principles

These are checked in continuous integration or refused in review. They are not
aspirations.

<dl>
<dt>Correct</dt>
<dd>Every byte of OpenType is parsed by fontations, the parser Chrome and Skia use.
Families come from name IDs 16 and 17 and from <code>STAT</code>, never from file
names. Wrong metadata is a bug.</dd>
<dt>Standard</dt>
<dd>The style model is CSS Fonts Level 4. Licenses are SPDX identifiers. Paths follow
the XDG Base Directory specification and each platform's conventions. Every export
is JSON with a published JSON Schema. Configuration is TOML.</dd>
<dt>Light</dt>
<dd>Install size, start time and idle memory have hard budgets. A build that exceeds
them fails.</dd>
<dt>Private</dt>
<dd>No network calls in the core or the command line. No telemetry, ever. Catalog
features, when they come, are opt-in and live only in the desktop application.</dd>
<dt>Per-user</dt>
<dd>System font directories are never modified. Nothing requires elevation.</dd>
<dt>Verifiable</dt>
<dd>Releases carry SHA-256 checksums, SLSA build provenance attestations and an SPDX
software bill of materials. Nothing is built or uploaded by hand.</dd>
<dt>Free</dt>
<dd>MIT OR Apache-2.0, for the library, the command line and the application, with
contributions accepted under the same terms and no contributor agreement.</dd>
</dl>

## Non-goals

Font editing. Format conversion or subsetting; that is fonttools' job. Cloud
synchronisation. Accounts. Telemetry. In-process plugins.

## How decisions are made

There is one maintainer for now. Decisions worth arguing about are written down as
[architecture decision records](../adr/) before the code lands, and a record is never
edited after acceptance; a change of mind is a new record. The
[roadmap](../roadmap/) says what is next and the [contributing](../contributing/)
page how to take part.

## The name

"unifont" is a working name. It collides with GNU Unifont, the bitmap font, and will
change before the first release.
