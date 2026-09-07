---
id: 22
title: Ship through the package managers people actually use
type: chore
status: doing
milestone: m5-ship
depends_on:
- 39
created: 2026-09-05
updated: 2026-09-06
priority: p1
effort: m
crate: workspace
---

## Problem

Releases are archives, a `.deb` and an `.rpm`. Anyone on macOS or Windows, and
anyone on Arch, installs by hand. PLAN.md has carried this under M1 item 6 since
M1 shipped, and it blocks a 1.0 rather than any milestone.

## Proposal

A Homebrew formula, winget and Scoop manifests, and an AUR `PKGBUILD`, each fed by
the existing release workflow so a release publishes them rather than someone
remembering to.

## Acceptance criteria

- [x] manifests for all four, generated rather than hand-kept (#116)
- [ ] `brew install fontina` works on macOS, Apple silicon and Intel
- [ ] `winget install fontina` and `scoop install fontina` work
- [ ] the AUR package builds from the released source
- [ ] a release updates all four without manual steps
- [ ] installed through each manager in a clean machine, `scripts/acceptance` run against
      whatever it put on `PATH`

## Progress

`packaging/` holds all seven manifests, written by `scripts/package-manifests` from the
`.sha256` sidecars the release already publishes — generated for the same reason
`schemas/` is, since each one repeats the version and a checksum and a hand-kept set is
a version behind by the second release.

Validated rather than eyeballed: `ruby -c`, `bash -n` on both PKGBUILDs, JSON and YAML
parses, two checksums recomputed from the downloaded archives, all five traced to what
v0.1.1 published, and generation idempotent.

`oddurs/homebrew-fontina` and `oddurs/scoop-fontina` exist and are empty.

What remains is publishing, and it is blocked on 0039 — every step needs a credential the
repository does not have.

## Notes

The install test cannot follow the container pattern for all four. AUR fits (`makepkg` in
an `arch` image); Homebrew and winget need `macos-latest` and `windows-latest` runners, so
it adds CI matrix jobs rather than cases to `scripts/test-distros`.
