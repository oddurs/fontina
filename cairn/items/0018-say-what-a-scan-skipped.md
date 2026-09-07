---
id: 18
title: Say what a scan skipped
type: feat
status: backlog
milestone: unfiled
created: 2026-09-05
updated: 2026-09-05
priority: p1
effort: s
crate: core
---

## Problem

`collect_candidates` filters on file extension before reading anything, so a font
in a format fontina does not support is never considered and never mentioned. A
directory holding a Type 1 font, a BDF and a TTF reports:

    scanned 1 candidates in 0.00s: 1 parsed (1 faces), 0 unchanged, 0 removed, 0 failed

Two real fonts vanished and the report says nothing failed. For a font manager
that is the worst failure mode available: a parse error can be acted on, a file
that was never considered leaves someone believing their library is indexed.

## Proposal

Count what was skipped and say so. Optionally sniff the first four bytes of files
whose extension is unknown or absent, which is what `file(1)` does and would have
caught the extensionless PostScript fonts macOS ships.

## Acceptance criteria

- [ ] the scan report names how many files were skipped and why
- [ ] `--json` carries the same, so a script can see it
- [ ] a test with a Type 1 file and a BDF file in the scanned directory
