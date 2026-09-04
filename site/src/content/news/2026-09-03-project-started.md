---
title: The core and the command line exist
date: 2026-09-03
---

unifont 0.0.1 is tagged. It is not a release for users yet, but the foundation is
in place: a Rust core that parses TTF, OTF, TTC, WOFF and WOFF2 through fontations,
a SQLite index with full-text search, and a `unifont` command line with `scan`,
`list`, `families`, `facets`, `info`, `dupes`, `css`, `check`, `covers`, `glyphs`,
`license`, `specimen`, tags, collections and sources. Every command takes `--json`
and the output types are published as JSON Schema.

Continuous integration runs on Linux, macOS and Windows, checks the minimum supported
Rust version, audits licenses and advisories, and diffs the schemas. Releases are
built by a workflow with SLSA provenance and an SPDX bill of materials.

Next is font activation on all three platforms, a `watch` command, terminal previews
and a terminal interface. The roadmap page has the details.
