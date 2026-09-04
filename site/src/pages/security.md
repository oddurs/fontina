---
layout: ../layouts/Page.astro
title: Security
description: "The unifont security policy and how to report a vulnerability privately."
source: site/src/pages/security.md
---

unifont parses untrusted font files. Parser bugs that lead to crashes, memory
unsafety or out-of-bounds reads are security bugs and are treated as such.

## Reporting a vulnerability

Do not open a public issue. Use GitHub's private vulnerability reporting at
[github.com/oddurs/unifont/security/advisories/new](https://github.com/oddurs/unifont/security/advisories/new),
or email <oddurs@gmail.com> with `[unifont security]` in the subject.

Include the font file that triggers the problem if you can share it, or the
`unifont info --json` output and a description of how the file was produced.

You will get an acknowledgement within 7 days and a fix or a mitigation plan within
90 days. Reporters are credited in the release notes unless they ask not to be.

This policy is also published as `/.well-known/security.txt` on this site.

## Supported versions

Only the latest release and the `main` branch receive fixes.

## Scope

- `unifont-core`: parsing, WOFF and WOFF2 decoding, and the SQLite index
- the `unifont` command line
- release artifacts and the build pipeline

Out of scope: vulnerabilities in fonts themselves, or in operating-system font
rasterisers that unifont does not ship.

## Hardening in place

- All OpenType parsing goes through fontations, which is fuzzed continuously in
  OSS-Fuzz.
- Parsing runs inside a panic boundary. A malformed file is reported, never fatal.
- No network access, no telemetry, no elevation, no writes to system font
  directories.
- Releases carry SLSA build provenance attestations and an SPDX software bill of
  materials; see [Download](../download/) for how to verify them.
- Dependencies are audited against the RustSec advisory database on every change.
