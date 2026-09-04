# unifont — working agreement for coding agents and contributors

unifont is a lightweight, cross-platform, open-source font manager. Rust core, thin
native shell, open standards end to end. Read `PLAN.md` for the architecture and the
milestones; this file is the operating manual.

## Layout

```
crates/unifont-core       parsing (fontations), metadata model, SQLite index, scan, CSS export
crates/unifont-platform   per-OS font directories and the FontActivator trait
crates/unifont-cli        the `unifont` binary
apps/desktop              Tauri 2 + Svelte 5 app (M1, not yet present)
schemas/                  JSON Schema for the metadata model; regenerate with `unifont schema`
fixtures/                 OFL-licensed test fonts; keep total size small
docs/adr/                 architecture decision records, one file per decision
```

## Build, test, lint

```
cargo build                       # workspace
cargo test                        # unit + fixture snapshot tests
cargo insta review                # after intentional metadata changes
cargo clippy --all-targets -- -D warnings
cargo fmt --all
cargo deny check                  # licenses and advisories (needs cargo-deny)
./target/debug/unifont scan fixtures --db /tmp/u.db && ./target/debug/unifont list --db /tmp/u.db
```

Set `UNIFONT_DB` to keep an index out of the platform data directory while developing.

## Rules that are not negotiable

- **Never modify system font directories** or require elevation. Per-user only.
- **No network calls** in core or CLI. Catalog features are opt-in and live in the app.
- **No telemetry.** Do not add analytics or crash reporting of any kind.
- **Parsing goes through fontations** (`read-fonts`/`skrifa`). Do not hand-parse tables
  that fontations already exposes. WOFF1 unwrapping is the one hand-written codec.
- **Standards over invention.** Style model is CSS Fonts Level 4. Licenses are SPDX.
  Paths follow XDG and platform conventions via the `directories` crate. Exports are
  JSON validated against `schemas/`. Config is TOML.
- **Errors are values.** The core never panics on font input; parsing runs inside
  `catch_unwind` in `scan::parse_paths` as a last line of defence, not an excuse.
- **Keep it light.** Adding a dependency needs a reason in the PR. Check binary size and
  idle memory against the budgets in `PLAN.md` §6 for anything touching the app.
- **No `Co-Authored-By` or session trailers** in commit messages.

## Conventions

- Rust 2024 edition, MSRV 1.88 (`rust-version` in `Cargo.toml`). Format with rustfmt,
  zero clippy warnings.
- Public types in `unifont-core::model` are the schema. Any change to them bumps
  `SCHEMA_VERSION` if it is not backwards-compatible, and regenerates `schemas/face.json`.
- Snapshot tests in `crates/unifont-core/tests` cover every fixture. Add a fixture when a
  new capability is parsed (a fixture is an OFL/Apache/UFL font under ~500 KB).
- SQL lives in `crates/unifont-core/src/index`. Migrations are append-only entries in
  `schema.rs`; never edit an applied migration.
- Platform-specific code is `#[cfg(target_os)]`-gated inside `unifont-platform`. Core
  and CLI stay platform-agnostic.
- CLI output: human-readable by default, `--json` for machines, exit code 1 on error.
  Anything printed with `--json` must be a type that serialises stably.

## Git workflow

`main` is protected and always releasable. All work goes through pull requests. See
`CONTRIBUTING.md` for the branch naming, commit format and review checklist. In short:

1. Branch from `main`: `feat/<topic>`, `fix/<topic>`, `chore/<topic>`, `docs/<topic>`.
2. Conventional Commits (`feat(core): parse STAT axis values`). Scope is the crate or area.
3. Open a PR early; CI must be green (fmt, clippy, tests on Linux, macOS, Windows, deny).
4. Squash-merge. The PR title becomes the commit subject, so keep it in Conventional
   Commits form. release-please turns those into the changelog and version bumps.

## When working as an agent

- Read `PLAN.md` before starting anything larger than a bug fix; do not re-plan.
- Run `cargo test` and clippy before declaring work done, and say so with the output.
- Do not create files outside the layout above without a reason stated in the PR.
- Prefer extending `FaceMetadata` over adding parallel data structures.
- If a fontations API is missing something, note it in the PR instead of parsing by hand.
