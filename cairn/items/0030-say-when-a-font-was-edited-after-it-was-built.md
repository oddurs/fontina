---
id: 30
title: Say when a font was edited after it was built
type: feat
status: dropped
milestone: integrity
created: 2026-09-06
updated: 2026-09-07
priority: p2
effort: m
crate: core
---

## Problem

`head.checkSumAdjustment` is the only integrity value OpenType defines: a font records a
checksum of itself at build time. Nothing here reads it, so fontina cannot answer the one
question the format was designed to answer about its own file.

## Proposal

Compute it at parse time — zero the field, sum the sfnt as big-endian `u32`, compare
against `0xB1B0AFBA` minus that sum — and store whether it matched. `read-fonts` exposes
`head.checksum_adjustment()` for the declared value; the sum is arithmetic over bytes the
spec defines, not a table this would be hand-parsing.

`head/checksum` reports a mismatch as **info**, and the wording has to carry its own
caveat, because the check is weaker than it sounds:

- a mismatch means the file changed after it was built, which is *usually* a subsetter or
  an autohinter doing its job;
- a match means nothing at all, because any tool that edits a font can recompute it.

That asymmetry is the whole finding. It is worth reporting and it is not worth a warning,
and saying so in the message is the difference between information and alarm.

## Acceptance criteria

- [ ] the model carries whether the declared checksum matched the computed one
- [ ] `head/checksum` reports a mismatch as info, with a message that says a match proves
      nothing
- [ ] correct for a WOFF: the checksum covers the sfnt inside, so it is computed after
      unwrapping
- [ ] a mutated fixture triggers it — no honest fixture can

## Notes

Out of scope, and deliberately: comparing a font against a reference copy. fontina has
BLAKE3 for every file and no way to fetch what the file *should* be, and fetching one
would need the network. It can tell you two copies differ. It cannot tell you which is
right, and should not imply that it can.

## Measured, and dropped

Surveyed before writing any Rust: the spec's algorithm over every bare sfnt on a working
machine — 391 files across `/System/Library/Fonts`, `~/Library/Fonts` and `fixtures/`.

    checksum agrees:    341
    checksum disagrees:  50   (12.8%)

The disagreements are Apple's own shipped system fonts — `Monaco`, `Geneva`,
`Apple Symbols`, `LastResort`, `Apple Braille`, the whole `SF*` family. They are not
edited, tampered with, or suspect. Apple's build simply does not fix the field up, and
nothing downstream cares.

`fontina check` shows info findings by default, so this would put a finding on one system
font in eight, every one of them a false alarm in the only sense that matters: nobody can
act on it. A check that flags Monaco is a check people learn to skip, and then they skip
the ones that matter too.

The item's own note already said a match proves nothing and a mismatch is usually a
subsetter doing its job. The survey turns that caveat into a number, and the number says
do not ship it.

Dropped, with the same test 0032 is filed under: if a reader cannot act on the finding,
it should not be in the default output. Reopen only with a use that needs it — comparing
one specific file against one specific reference, where 12.8% base noise is irrelevant
because you are asking about one font.
