---
id: 66
title: Nothing checks that macOS or Windows can actually see an activated font
type: test
status: done
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

- [x] `scripts/acceptance` has a macOS branch that asserts an activated font is visible
      to a program that is not fontina (#199)
- [x] the same for Windows (#199)
- [x] both run in CI, and the workflow's gate job stands for them (#199, `desktop.yml`)
- [x] a deliberate break in `macos.rs` — registering under the wrong scope — fails it

## Notes

Found by mapping test coverage across the tree on 2026-09-07. Everything else is well
covered: 28,319 source lines against 14,015 lines of integration tests, fuzzing,
snapshots and property-style invariants. This is the one place where a real user on a
real machine could hit something no test would ever catch.

`m5-ship` rather than unfiled: that milestone is where shipping lives — the packages, the
credentials, the budgets — and shipping a font manager to macOS and Windows without
verifying activation on either is the same class of question.

## Closed

Shipped in #199. `seen()` — the one function the whole script turns on — dispatches per
system now: `fc-list` on GNU/Linux and the BSDs, `system_profiler` on macOS (where Font
Book gets its data), GDI+ through PowerShell on Windows. `desktop.yml` runs it on
`macos-latest` and `windows-latest` with a gate job in the shape the other four
path-filtered workflows use, and `perf, fuzz, linux, site` became five in 0041.

CI on the pull request: **macOS 43 passed 0 failed, Windows 44 passed 0 failed.**

### The fourth criterion, done rather than ticked

Registering under the wrong scope had to fail it, so it was tried. The first attempt was
wrong and is worth recording: swapping `Scope::User` for `SCOPE_SESSION` changed nothing
the test could see, and the test correctly did **not** fail. `kCTFontManagerScopeSession`
is session-wide and visible to other processes; only persistence across logout differs,
which no acceptance test can observe.

The mutation that matters is `kCTFontManagerScopeProcess` — registered for the calling
process alone:

    ok    Bricolage is not visible before activation
    ok    activate family:Bricolage Grotesque          <- fontina reports success
    FAIL  system_profiler does not see Bricolage       <- nothing else can see it

That is exactly the bug this item exists to catch, and before #199 it would have passed
every check in the repository. The pair also shows the test discriminates rather than
merely being sensitive: it stays green for a scope change that does not affect visibility
and goes red for one that does.

Run against a family this machine did not already have, because Amiri is installed here —
which is also why the script now detects that condition and switches the visibility
assertions off with a reason rather than reporting a green line that verifies nothing.

### What cannot be checked, and why

An installed copy on macOS is invisible to `system_profiler` from this harness. `install`
there is a plain copy into `~/Library/Fonts` that fontd watches, and fontd watches the
real home rather than the sandbox's. fontconfig reads `$XDG_DATA_HOME/fonts`, which the
sandbox redirects; Windows registers the file as well as copying it, so GDI answers for
it wherever it lives. The alternative is not sandboxing HOME, which would mean an
acceptance test writing into somebody's actual font directory. Skipped with the reason
printed. Activation is not skipped, and it is the half a sandbox cannot hide.
