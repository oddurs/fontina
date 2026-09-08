---
id: 66
title: Nothing checks that macOS or Windows can actually see an activated font
type: test
status: doing
milestone: m5-ship
created: 2026-09-07
updated: 2026-09-07
priority: p1
effort: l
crate: platform
---

## Problem

`scripts/acceptance` is the test that answers the only question a font manager has to
answer: after `activate`, can some *other* program see the font? On GNU/Linux it answers
that with `fc-list` and `fc-match`, and `linux.yml` runs it on every pull request that
touches the crates, in six distributions and again against the installed `.deb` and
`.rpm`.

On macOS and Windows nothing answers it at all. The script branches on `uname -s` and
only reaches the fontconfig assertions on `Linux | *BSD | DragonFly`; there is no
CoreText branch and no DirectWrite branch. What those platforms have is the
`platform-tests` unit tests in `macos.rs` (4) and `windows.rs` (2), and the script itself
says what that is worth:

    # The unit tests can only prove fontina wrote the symlink it meant to write.

So the two platforms where a user is most likely to already have Font Book or the
Windows font folder open are the two where activation is verified least. A change to
`macos.rs` that registers a font CoreText will not load passes every check this
repository has.

## Proposal

Give `acceptance` the branch it is missing, using each system's own font API as the
"other program", the way `fc-list` already is:

- **macOS** — `system_profiler SPFontsDataType` is present on every install and needs no
  compilation. A CoreText probe via `CTFontManagerCopyAvailableFontFamilyNames` is more
  precise if the shell route proves flaky.
- **Windows** — enumerate through DirectWrite, or read back what `AddFontResourceEx`
  registered. PowerShell's `[System.Drawing.Text.InstalledFontCollection]` is the cheap
  first cut.

Then run it in CI on `macos-latest` and `windows-latest`. The runners are already in the
`test` matrix, so this is a job, not a new dependency.

## Acceptance criteria

- [ ] `scripts/acceptance` has a macOS branch that asserts an activated font is visible
      to a program that is not fontina
- [ ] the same for Windows
- [ ] both run in CI, and the workflow's gate job stands for them
- [ ] a deliberate break in `macos.rs` — registering under the wrong scope — fails it

## Notes

Found by mapping test coverage across the tree on 2026-09-07. Everything else is well
covered: 28,319 source lines against 14,015 lines of integration tests, fuzzing,
snapshots and property-style invariants. This is the one place where a real user on a
real machine could hit something no test would ever catch.

`m5-ship` rather than unfiled: that milestone is where shipping lives — the packages, the
credentials, the budgets — and shipping a font manager to macOS and Windows without
verifying activation on either is the same class of question.
