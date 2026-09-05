# fontina — working agreement for coding agents and contributors

fontina is a lightweight, cross-platform font manager, and free software. Rust core, thin
native shell, open standards end to end. Read `PLAN.md` for the architecture and the
milestones; this file is the operating manual.

## Layout

```
crates/fontina-core       parsing (fontations), metadata model, SQLite index, scan, CSS export,
                          health checks (check.rs), HTML specimen (specimen.rs),
                          license freedom classification (freedom.rs)
crates/fontina-platform   per-OS font directories and the FontActivator trait
crates/fontina-cli        the `fontina` binary; `src/ui/` is the ratatui TUI (M1)
schemas/                  JSON Schemas (face, collection, cli-output); regenerate with `fontina schema <name>`
fixtures/                 OFL-licensed test fonts; keep total size small
fuzz/                     cargo-fuzz targets (own workspace, nightly); `scripts/fuzz` drives them,
                          `fuzz/regressions/` keeps the findings and stable replays them
docs/adr/                 architecture decision records, one file per decision
docs/fontina.texi         the manual (GFDL); man pages come from `fontina man`
site/                     project web site and manual (Astro, static, no JS); deploys to GitHub Pages
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
./target/debug/fontina scan fixtures --db /tmp/u.db && ./target/debug/fontina list --db /tmp/u.db

FONTINA=./target/debug/fontina scripts/acceptance    # end-to-end, on this machine
scripts/test-distros                                 # the same, inside six distributions
scripts/test-packages                                # the .deb and .rpm, installed for real
```

Set `FONTINA_DB` to keep an index out of the platform data directory while developing.

`scripts/acceptance` is the test that answers the only question that matters for a font
manager: after `activate`, can another program see the font? On GNU/Linux `fc-list` and
`fc-match` are that other program. `scripts/test-distros` runs it in Debian, Ubuntu,
Fedora, Arch, Alpine and a Debian with no fontconfig installed, using a container
runtime (OrbStack, Podman or Docker); `.github/workflows/linux.yml` runs the same script
on every pull request that touches the crates. `scripts/test-packages` goes one step
further and tests the `.deb` and `.rpm` themselves: it builds them from the manifests in
`crates/fontina-cli/Cargo.toml`, installs them with `apt` and `dnf` in a clean Debian,
Ubuntu and Fedora, and runs `scripts/acceptance` against the binary the package put on
`PATH`. Change a packaging asset path and that is the test that will tell you.

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
- **Free software, and it stays free.** The workspace is GPL-3.0-or-later (ADR 0007);
  the manual is GFDL 1.3+. Every new source file gets the standard GPL notice, copied
  from any existing one. A dependency must be GPLv3-compatible — `deny.toml` is where
  that argument is settled, not the pull request. Never relicense anything permissively.
- **Report restrictions, never enforce them.** `OS/2.fsType` and any similar flag is
  data about the font, not permission to restrict the person running the program.
  `freedom.rs` says why; keep it that way.
- **No `Co-Authored-By` or session trailers** in commit messages.

## Conventions

- Rust 2024 edition, MSRV 1.88 (`rust-version` in `Cargo.toml`). Format with rustfmt,
  zero clippy warnings. Each crate root declares the lints that hold it to its own
  contract — the core forbids `unsafe` and denies every way of panicking; the CLI keeps
  the panic rules and may print. Exemptions are `#[expect(..., reason = "…")]`, never
  `#[allow]`, and the reason names a guard a reader can go and check. `CONTRIBUTING.md`
  §Code has the rest, including what to do about comments and naming.
- Public types in `fontina-core::model` are the schema. Any change to them bumps
  `SCHEMA_VERSION` if it is not backwards-compatible, and regenerates `schemas/face.json`.
  Every type printed with `--json` derives `JsonSchema` and is listed in
  `cli_output_schema()`; regenerate `schemas/cli-output.json` and `schemas/collection.json`
  with `fontina schema cli-output` / `fontina schema collection` (CI diffs all three).
- Snapshot tests in `crates/fontina-core/tests` cover every fixture. Add a fixture when a
  new capability is parsed (a fixture is an OFL/Apache/UFL font under ~500 KB).
- SQL lives in `crates/fontina-core/src/index`. Migrations are append-only entries in
  `schema.rs`; never edit an applied migration. A migration that needs data from the
  stored metadata JSON gets a backfill function keyed on its index (see `face_ranges`).
- Health checks in `check.rs` have stable `area/check` ids; never rename an id, add a
  new one. Every check needs a fixture-backed test that triggers it. A check that no
  fixture can legitimately trigger (`license/nonfree`: we may not redistribute a nonfree
  font) is triggered by mutating a parsed fixture, not by adding one.
- The freedom of a license is derived from its SPDX identifier on every read, never
  stored in the index, so the verdict tracks `freedom::FREE` rather than the day the
  index was built. Keep it that way when adding filters.
- The specimen (`specimen.rs`) is a single self-contained HTML file with no external
  requests; it is the reference implementation the desktop preview will reuse.
- Platform-specific code is `#[cfg(target_os)]`-gated inside `fontina-platform`. Core
  and CLI stay platform-agnostic.
- CLI output: human-readable by default, `--json` for machines, exit code 1 on error.
  Anything printed with `--json` must be a type that serialises stably.

## Git workflow

`main` is protected: every change is a pull request, and GitHub merges it once the
eight CI checks pass. The whole loop is:

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
  `cargo --manifest-path`). `scripts/wt rm feat/<topic>` when the PR has merged. If your
  tooling makes its own worktree elsewhere that is fine — it is ignored — but
  `scripts/wt status` is how everyone else finds out it exists.
- **Stage paths, never `git add -A` or `git add .`.** Untracked files in a checkout may
  belong to someone else's work in progress. Before every push, check
  `git diff --stat origin/main` shows only files your PR is about.
- **Do not touch files outside your PR's scope**, even to tidy them. If something in
  another area is wrong, say so in the PR description or open an issue.
- **Rebase, don't merge**, and expect `main` to move — several PRs an hour on a busy
  day. `scripts/wt status` says how far behind every worktree is; `scripts/wt sync`
  fetches and rebases one onto `origin/main`. Run `sync` before you push, and again
  whenever CI looks wrong. Auto-merge fires the moment CI is green, so never push a
  commit you would not want on `main` a minute later, and fix a bad push with a new
  commit rather than hoping to beat the merge.
- **A conflicted pull request gets no CI at all.** GitHub cannot build
  `refs/pull/<n>/merge` for a PR that conflicts with `main`, so it creates no
  `pull_request` workflow runs: the checks list shows only `enable`, and nothing says
  why. If `ci` never appears, run `gh pr view <n> --json mergeable,mergeStateStatus`
  before suspecting the workflow file, then `scripts/wt sync`.
- **Each PR is one logical change** with the smallest diff that does it. Shared files
  (`CLAUDE.md`, `README.md`, `Cargo.toml`, `main.rs`) conflict most; keep edits to them
  minimal and rebase promptly after another PR touching them merges.
- **Generated files are regenerated, not hand-edited**: `schemas/*.json` with
  `fontina schema <name>`, snapshots with `cargo insta review`.

## When working as an agent

- Read `PLAN.md` before starting anything larger than a bug fix; do not re-plan.
- Run `cargo test` and clippy before declaring work done, and say so with the output.
- Do not create files outside the layout above without a reason stated in the PR.
- Prefer extending `FaceMetadata` over adding parallel data structures.
- If a fontations API is missing something, note it in the PR instead of parsing by hand.
