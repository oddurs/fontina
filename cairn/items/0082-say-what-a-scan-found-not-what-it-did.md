---
id: 82
title: Say what a scan found, not what it did
type: feat
status: review
milestone: arrival
created: 2026-09-15
updated: 2026-09-15
priority: p1
effort: s
crate: cli
---

## Problem

```
scanned 516 candidates in 1.06s: 515 parsed (932 faces), 0 unchanged, 0 removed, 1 failed
```

Six numbers about the program's work and none about the person's fonts. `candidates`,
`parsed`, `unchanged` and `removed` are the scanner's vocabulary. After the first scan of
their life, what somebody wants to know is what they have.

## Proposal

Keep the ledger — it is the right answer on a rescan, and a script reads it — and lead
with the library on a first scan: how many families, the ones with the most faces, how
many are free, anything that failed and why.

The distinction is first-scan versus rescan, which the index already knows: a rescan has
`unchanged` greater than zero and nothing new to say.

## Acceptance criteria

- [x] A first scan says what was found, in a form a person reads.
- [x] A rescan keeps the current one-line ledger.
- [x] Failures stay visible in both, named with the reason.
- [x] `--json` is unchanged.
