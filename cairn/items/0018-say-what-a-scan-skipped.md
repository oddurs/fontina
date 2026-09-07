---
id: 18
title: Say what a scan skipped
type: feat
status: done
milestone: unfiled
created: 2026-09-05
updated: 2026-09-07
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

- [x] the scan report names how many files were skipped and why (#141)
- [x] `--json` carries the same, so a script can see it (#141)
- [x] a test with a Type 1 file and a BDF file in the scanned directory (#141)

## Closed

Shipped in #141. Checked against the built binary rather than the tests alone — a
directory holding three TrueType fixtures and one BDF:

    scanned 3 candidates in 0.04s: 3 parsed (3 faces), 0 unchanged, 0 removed, 0 failed
    skipped 1 font(s) in a format this program does not read: BDF bitmap
      - .../scandir/fixed.bdf (BDF bitmap)

and `--json` carries `skipped: [{path, format}]`, which is how the pinned-types test in
#181 caught `SkippedFile` joining the published set without being pinned.

`scan::walk` sniffs content rather than extension, which is the part that matters: the
fonts that prompted this — macOS's `HelveLTMM` and `TimesLTMM`, datafork Type 1 Multiple
Masters — carry no extension at all, so no extension filter could ever have seen them.
Six signatures are recognised, and `is_resource_fork` checks the header's arithmetic
rather than a four-byte magic, because `00 00 01 00` opens plenty of files that are not
fonts and being wrong here means telling somebody their spreadsheet is a font.

One deliberate limit worth writing down, since it looks like a bug from outside: `sniff`
refuses anything below 64 bytes or above 64 MB before opening it, so a truncated stub is
not reported. That keeps a scan of a source tree from reading every object file and a
scan of a media directory from touching a video, and
`a_file_too_small_to_be_a_font_is_not_opened` holds it.
