---
layout: ../layouts/Page.astro
title: fontina releases
description: "Release policy, what a release contains, and the timeline."
source: site/src/pages/releases.md
---

## Current

The current release is
[fontina 0.1.0](https://github.com/oddurs/fontina/releases/tag/v0.1.0), 2026-09-04.
It is the first tagged release: the core library and the command line with
organising, activation, watching, previews and the terminal browser. Pre-1.0,
so command output, schemas and check identifiers may still change; every such
change is called out in the changelog.

## What a release contains

Every release on
[github.com/oddurs/fontina/releases](https://github.com/oddurs/fontina/releases)
carries, per platform, one archive with the binary, shell completions for bash,
zsh, fish and PowerShell, and the man pages; a SHA-256 checksum for each archive;
a SLSA build provenance attestation; `.deb` and `.rpm` packages for Linux with their
own checksums; and an SPDX software bill of materials. See
[Download](../download/) for verification.

## Policy

Only the latest release and the `main` branch receive fixes. Versions follow
[semantic versioning](https://semver.org/). Releases are cut by
[release-please](https://github.com/googleapis/release-please) from the commit
history: it keeps a release pull request open with the changelog and the version
bump, and merging that pull request tags the release and starts the build. The
[changelog](https://github.com/oddurs/fontina/blob/main/CHANGELOG.md) is generated
the same way. Nothing is released by hand.

Before 1.0, command output, schemas and check identifiers may still change, and
every such change is called out in the changelog. From 1.0, they are stable.

Security fixes are announced in the release notes and, for anything serious, on the
[news](../news/) page.

## Timeline

<table>
<tr><th>Release</th><th>Date</th><th>Notes</th></tr>
<tr><td><a href="https://github.com/oddurs/fontina/releases/tag/v0.1.0">fontina 0.1.0</a></td><td>September 4, 2026</td><td>First release. Renamed from unifont and relicensed under GPL-3.0-or-later in the same release.</td></tr>
</table>
