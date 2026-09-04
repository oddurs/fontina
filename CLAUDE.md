# unifont — working agreement for coding agents and contributors

unifont is a lightweight, cross-platform, open-source font manager. Rust core, thin
native shell, open standards end to end. Read `PLAN.md` for the architecture and the
milestones; this file is the operating manual.

## Layout

```
crates/unifont-core       parsing (fontations), metadata model, SQLite index, scan, CSS export,
                          health checks (check.rs), HTML specimen (specimen.rs)
crates/unifont-platform   per-OS font directories and the FontActivator trait
crates/unifont-cli        the `unifont` binary; `src/ui/` is the ratatui TUI (M1)
schemas/                  JSON Schemas (face, collection, cli-output); regenerate with `unifont schema <name>`
fixtures/                 OFL-licensed test fonts; keep total size small
docs/adr/                 architecture decision records, one file per decision
scripts/wt                worktree helper for parallel branches (see below)
.worktrees/               one checkout per branch in flight; git-ignored
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
  Every type printed with `--json` derives `JsonSchema` and is listed in
  `cli_output_schema()`; regenerate `schemas/cli-output.json` and `schemas/collection.json`
  with `unifont schema cli-output` / `unifont schema collection` (CI diffs all three).
- Snapshot tests in `crates/unifont-core/tests` cover every fixture. Add a fixture when a
  new capability is parsed (a fixture is an OFL/Apache/UFL font under ~500 KB).
- SQL lives in `crates/unifont-core/src/index`. Migrations are append-only entries in
  `schema.rs`; never edit an applied migration. A migration that needs data from the
  stored metadata JSON gets a backfill function keyed on its index (see `face_ranges`).
- Health checks in `check.rs` have stable `area/check` ids; never rename an id, add a
  new one. Every check needs a fixture-backed test that triggers it.
- The specimen (`specimen.rs`) is a single self-contained HTML file with no external
  requests; it is the reference implementation the desktop preview will reuse.
- Platform-specific code is `#[cfg(target_os)]`-gated inside `unifont-platform`. Core
  and CLI stay platform-agnostic.
- CLI output: human-readable by default, `--json` for machines, exit code 1 on error.
  Anything printed with `--json` must be a type that serialises stably.

## Git workflow

`main` is protected: every change is a pull request, and GitHub merges it once the
seven CI checks pass. The whole loop is:

```
git checkout -b feat/<topic> main
# ...make the change, run cargo test and clippy...
git commit -m "feat(core): <what and why>"
gh pr create --fill
gh pr merge --auto --squash --delete-branch
```

That last command enables GitHub's native auto-merge; nothing else to do. Details:

- Branch prefixes: `feat/`, `fix/`, `chore/`, `docs/`, `refactor/`, `ci/`.
- Conventional Commits, scope is the crate or area. The PR title becomes the squash
  commit subject, so keep it in that form too.
- If a check fails, push a fix to the same branch; auto-merge stays armed.
- Dependabot minor/patch PRs auto-merge on their own. Major bumps wait for a person.
- release-please opens a `chore(main): release x.y.z` PR with the changelog and version
  bump. Merging it (same `gh pr merge --squash`) tags the release and builds binaries.

See `CONTRIBUTING.md` for the longer form.

## Working alongside other agents

Several agents (and the maintainer) work in this repository at the same time. The rules
that keep them out of each other's way:

- **One worktree per branch.** Never switch branches in a checkout someone else may be
  using. `scripts/wt new feat/<topic>` gives you `.worktrees/feat-<topic>` with its own
  working tree and index; do everything there (`cd` into it, or use `git -C` and
  `cargo --manifest-path`). `scripts/wt rm feat/<topic>` when the PR has merged.
- **Stage paths, never `git add -A` or `git add .`.** Untracked files in a checkout may
  belong to someone else's work in progress. Before every push, check
  `git diff --stat origin/main` shows only files your PR is about.
- **Do not touch files outside your PR's scope**, even to tidy them. If something in
  another area is wrong, say so in the PR description or open an issue.
- **Rebase, don't merge**, and expect `main` to move: `git rebase origin/main` before
  pushing. Auto-merge fires the moment CI is green, so never push a commit you would
  not want on `main` a minute later, and fix a bad push with a new commit rather than
  hoping to beat the merge.
- **Each PR is one logical change** with the smallest diff that does it. Shared files
  (`CLAUDE.md`, `README.md`, `Cargo.toml`, `main.rs`) conflict most; keep edits to them
  minimal and rebase promptly after another PR touching them merges.
- **Generated files are regenerated, not hand-edited**: `schemas/*.json` with
  `unifont schema <name>`, snapshots with `cargo insta review`.

## When working as an agent

- Read `PLAN.md` before starting anything larger than a bug fix; do not re-plan.
- Run `cargo test` and clippy before declaring work done, and say so with the output.
- Do not create files outside the layout above without a reason stated in the PR.
- Prefer extending `FaceMetadata` over adding parallel data structures.
- If a fontations API is missing something, note it in the PR instead of parsing by hand.
