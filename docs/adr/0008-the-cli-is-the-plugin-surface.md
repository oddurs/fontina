# 0008 — The CLI is the plugin surface, and here is what it promises

**Status:** accepted, 2026-09-05.

## Context

fontina has no plugin API and is not going to grow one. Every command prints
machine-readable output with `--json`, every type it prints has a definition in
`schemas/cli-output.json`, CI diffs that schema on every change, and since M3 every
command that takes faces reads them from standard input as well. A program can already
read fontina, filter, and write back:

```
fontina list --json --free | jq '[.[] | select(.variable)]' | fontina tag add variable -
```

That is a plugin surface. It is also, right now, an accident: it describes what the
program happens to do today rather than what anyone building on it may rely on
tomorrow. Somebody who writes a script against `check` ids, or a tool that reads
`FaceSummary.freedom`, has no way to know which of those is a promise and which is an
implementation detail that will move in the next release.

An in-process plugin API — a dynamic library, a scripting runtime, a WASM host — was the
obvious alternative and is rejected in its own section below. This ADR is the cheaper
half of the same goal: write down what the existing surface guarantees, so that building
on it is a decision rather than a gamble.

## Decision

**The command-line interface is the extension interface.** These things are stable, and
changing any of them is a breaking change requiring a major version:

1. **Face ids** are stable within an index. A face keeps its id across a rescan of the
   same file (`carry_over_*` in `index/library.rs` exists for this). An id is *not*
   portable between indexes; the identity hash is what carries a face between machines,
   which is why the collection export is built on it and not on ids.

2. **Health-check ids** (`area/check`, e.g. `license/nonfree`, `fvar/axis-range`). An id
   is never renamed and never re-pointed at a different condition. A check whose meaning
   changes gets a new id and the old one is retired. `CLAUDE.md` has said this since M1;
   this ADR makes it a promise to people outside the repository, not only a rule inside
   it.

3. **JSON field names and their types**, for every type in `schemas/cli-output.json` and
   `schemas/collection.json`. A field is never renamed, never given a different type, and
   never repurposed.

4. **Exit codes.** `0` success. `1` an error, with a message on stderr. `2` a conflict
   that stopped the operation — `activate` and `conflicts` use it, and it is deliberately
   distinct from `1` so a script can tell "this font clashes with one already active"
   from "something went wrong". `check` uses `1` for a failing check, which is an error
   about the font rather than about fontina.

5. **`--json` on stdout, prose on stderr.** A pipeline reading stdout never has to filter
   progress or warnings out of it. `--json` output is a single value per invocation,
   except `watch --json`, which is one object per line by design.

6. **Reading targets from `-`**, in both shapes documented in the manual: fontina's own
   `--json` output, and one target per line.

**These things may be added, never removed:**

- New fields in a JSON object. A reader must ignore fields it does not know. Every
  optional field is `skip_serializing_if`, so absence is normal and must not be an error.
- New commands, new flags, new health checks, new facets.
- New enum variants — `Freedom`, `ActivationState`, `SourceKind`, `Severity`. A reader
  must not assume it has seen every value.

**These things are not promised:**

- The layout, wording and column order of human-readable output. It is for people. A
  script parsing it is parsing something that will move; that is what `--json` is for.
- Error message text. The exit code is the contract.
- Face ids across different indexes, or across a `--prune` that removed a file.
- The SQLite schema. It is an implementation detail with its own migrations; read it
  through the CLI, not through `sqlite3`.
- The TUI's keys and layout.
- `SCHEMA_VERSION` moving for an additive change. It marks incompatible changes to the
  metadata model, and additive fields do not move it. Read the schema, not the number.

**Where a plugin's own state goes:** nowhere in fontina. Tags and collections are the
places a program is expected to write, and both are first-class, exportable and
documented. A tool that needs storage of its own keeps it in its own file.

## Consequences

- Renaming a JSON field, a check id, or an exit code is now a version decision rather
  than a judgement call in review. `schemas/cli-output.json` in CI already catches the
  accidental cases; this ADR is what makes the deliberate ones visible.
- `--json` output types must stay `Serialize` with stable field names. That constrains
  refactoring inside `fontina-core::model` and `index`, which is the price of the
  promise.
- Adding a health check stays free. Changing one's meaning costs a new id and a retired
  one, which is the right price.
- A catalogue of downloadable fonts, deliberately left out of this tree
  (`PLAN.md` §11), becomes an external program that pipes candidates in. fontina gets the
  capability without the crate, the dependency, the packaging or the network question.
  That case is the first real test of this decision.

## Alternatives considered

- **An in-process plugin API** — a dynamic library with a C ABI, an embedded scripting
  runtime, or a WASM host. Rejected. Each is a large permanent surface with its own
  versioning problem, and each drags the thing this project is most careful about —
  `PLAN.md` §6's size and memory budgets, and the rule that nothing leaves the machine —
  into code fontina did not write and cannot audit. A plugin that crashes in-process
  crashes fontina; a program in a pipe does not. And the CLI surface already exists and
  is already tested.
- **A stable Rust library API.** `fontina-core` is a crate and can be depended on, but
  promising its API would freeze the internals of a program still finding its shape, for
  the benefit of a consumer who must be written in Rust and rebuilt against every
  release. The CLI is language-agnostic and process-isolated. `fontina-core` stays
  usable and stays unpromised; it is versioned with the workspace and will break.
- **Saying nothing and keeping the freedom to change everything.** What the project has
  been doing. It is honest only until somebody builds on the surface, and by then the
  cost of the first breaking change has been paid by them rather than decided by us.
- **A narrower promise — `--json` only.** Tempting, but the exit codes and the check ids
  are exactly what a script written against fontina reaches for first, and leaving them
  out would make the promise true and useless.
