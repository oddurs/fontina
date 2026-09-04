---
layout: ../layouts/Page.astro
title: fontina releases
description: "Release series, their support status, and the release timeline."
source: site/src/pages/releases.md
---

## Download

Releases may be downloaded from the
[GitHub releases page](https://github.com/oddurs/fontina/releases); see
[Download](../download/) for the archives per platform and how to verify them.
There are no mirrors yet.

*Important: 0.0.x releases are pre-releases for people who want to try the library
and the command line. There is no desktop application yet.*

You can also retrieve our sources
[using Git](https://github.com/oddurs/fontina).

## Support

Only the latest release and the `main` branch receive fixes. Releases follow
[semantic versioning](https://semver.org/) and are cut by
[release-please](https://github.com/googleapis/release-please) from the commit
history; the [changelog](https://github.com/oddurs/fontina/blob/main/CHANGELOG.md)
is generated the same way. Nothing is released by hand.

Security fixes are announced in the release notes and, for anything serious, on the
[news](../news/) page.

## Timeline

<table class="padding5">
<tr><th>Release</th><th>Release date</th><th>Notes</th></tr>
<tr><td><a href="https://github.com/oddurs/fontina/releases/tag/v0.0.1">fontina 0.0.1</a></td><td>September 3, 2026</td><td>Core library and command line. Pre-release.</td></tr>
</table>

Planned series, from the [roadmap](../roadmap/):

<table class="padding5">
<tr><th>Series</th><th>Scope</th></tr>
<tr><td>0.1</td><td>Font activation on Linux, macOS and Windows; <code>watch</code>; terminal previews and a terminal interface.</td></tr>
<tr><td>0.2</td><td>The desktop application, packaged for Flathub, AppImage, Homebrew and winget.</td></tr>
<tr><td>1.0</td><td>Stable schemas and command-line output; the public name.</td></tr>
</table>
