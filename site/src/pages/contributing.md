---
layout: ../layouts/Page.astro
title: Contributing
description: "How to hack on unifont: workflow, conventions, and what makes a good change."
source: site/src/pages/contributing.md
---

Thank you. This page is the short form; the repository's
[`CONTRIBUTING.md`](https://github.com/oddurs/unifont/blob/main/CONTRIBUTING.md) holds
the git workflow and [`CLAUDE.md`](https://github.com/oddurs/unifont/blob/main/CLAUDE.md)
the engineering rules that apply to every contributor, human or otherwise.

## Getting the source

```
git clone https://github.com/oddurs/unifont
cd unifont
cargo build
cargo test
cargo clippy --all-targets -- -D warnings
```

Rust 1.88 or newer. Set `UNIFONT_DB=/tmp/u.db` while developing so you do not touch
your real index.

## Repository layout

```
crates/unifont-core       parsing, metadata model, SQLite index, scan, CSS export,
                          health checks, HTML specimen
crates/unifont-platform   per-OS font directories and the FontActivator trait
crates/unifont-cli        the `unifont` binary
schemas/                  JSON Schemas; regenerated with `unifont schema <name>`
fixtures/                 OFL-licensed test fonts, kept small
docs/adr/                 architecture decision records
site/                     this web site
```

## Rules that are not negotiable

- Never modify system font directories or require elevation. Per-user only.
- No network calls in the core or the command line. No telemetry of any kind.
- Parsing goes through fontations. Do not hand-parse a table it already exposes.
- Standards over invention: CSS Fonts Level 4, SPDX, XDG, JSON Schema, TOML.
- Errors are values. The core never panics on font input.
- Every new dependency needs a stated reason.

## Workflow

Trunk-based. `main` is protected and every change is a pull request that GitHub merges
once the seven CI checks pass: format and clippy, tests on three operating systems,
minimum supported Rust version, license and advisory audit, and a check that the JSON
Schemas are current.

```
git checkout -b feat/<topic> main
# make the change; run cargo test and clippy
git commit -m "feat(core): <what and why>"
gh pr create --fill
gh pr merge --auto --squash --delete-branch
```

Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/)
with the crate as scope: `feat(core)`, `fix(cli)`, `docs`, `ci`. The pull request title
becomes the squash commit subject. No `Co-Authored-By` or tool trailers.
[release-please](https://github.com/googleapis/release-please) turns the history into
the changelog and the version bump; nothing is released by hand.

## What a good change looks like

- One logical change per pull request. Split refactors from features.
- A new parsed capability comes with a fixture font (open license, under 500 KB, source
  noted in `fixtures/README.md`) and a snapshot test.
- A new health check has a stable `area/check` identifier that is never renamed, and a
  fixture-backed test that triggers it.
- A change to a public type in the metadata model regenerates the schemas and, if not
  backwards compatible, bumps the schema version.
- Database changes are append-only migrations. Applied migrations are never edited.
- A decision worth arguing about gets an [architecture decision record](../adr/).

## Licensing of contributions

By contributing you agree your work is licensed MIT OR Apache-2.0, matching the
project, without any additional terms. There is no contributor license agreement to
sign.

## Conduct

The project follows the
[Contributor Covenant](https://github.com/oddurs/unifont/blob/main/CODE_OF_CONDUCT.md).
