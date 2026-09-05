# Contributing

Thanks for helping build fontina. This page is the git workflow; `CLAUDE.md` holds the
engineering conventions and `PLAN.md` the roadmap.

## Branching model

Trunk-based. `main` is protected: no direct pushes, linear history, every change via a
pull request with green CI.

| Branch prefix | Use |
|---|---|
| `feat/<topic>` | new capability |
| `fix/<topic>` | bug fix |
| `chore/<topic>` | tooling, deps, CI |
| `docs/<topic>` | documentation only |
| `refactor/<topic>` | no behaviour change |

Keep branches short-lived (days, not weeks). Rebase on `main` rather than merging it in.
Use a worktree per branch when you have more than one in flight (`scripts/wt new
feat/<topic>` creates `.worktrees/feat-<topic>`); `CLAUDE.md` has the etiquette for
working next to other people and agents in the same repository.

## Commits

[Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/):

```
<type>(<scope>): <subject>

<body: what and why, not how>
```

Types: `feat`, `fix`, `perf`, `refactor`, `docs`, `test`, `chore`, `build`, `ci`.
Scopes: `core`, `platform`, `cli`, `desktop`, `schema`, `deps`, or omit.
A `!` after the type/scope, or a `BREAKING CHANGE:` footer, marks a breaking change.

No `Co-Authored-By`, `Signed-off-by` or tool-session trailers.

## Pull requests

- Title in Conventional Commits form; it becomes the squash commit subject.
- Fill in the template: what changed, why, how it was tested, screenshots for UI.
- One logical change per PR. Split refactors from features.
- CI (`.github/workflows/ci.yml`) must pass on GNU/Linux, macOS and Windows.
- Self-review the diff before requesting review.
- Squash-merge only. Delete the branch after merge (automatic).

## Merging

`main` requires the seven CI checks, linear history and resolved conversations. After
opening a PR, enable auto-merge and walk away:

```
gh pr merge --auto --squash --delete-branch
```

Adding the `automerge` label does the same thing through a workflow. Dependabot minor
and patch updates are auto-merged automatically; major updates wait for a person.

## Releases

[release-please](https://github.com/googleapis/release-please) reads the commit history
on `main`, maintains `CHANGELOG.md`, and opens a release PR that bumps the workspace
version in `Cargo.toml` (the crates inherit it). The workflow then syncs `Cargo.lock`
on the release branch and approves the CI runs GitHub holds for bot-authored PRs, so
the required checks attach to the release PR. Merge it like any other PR. Merging that PR tags `vX.Y.Z`, and the release workflow
builds the archives (with completions and man pages), `.deb` and `.rpm` packages,
checksums, provenance attestations and the SBOM. Nothing is published by hand.

## Code

Anything a tool can check is checked by a tool. What follows is the part that is left.

### What the compiler enforces

Each crate root declares the lints that hold it to its own contract, so the rule and the
code that must obey it are in the same file. `cargo clippy --workspace --all-targets --
-D warnings` is the gate, and CI runs exactly that.

- **`fontina-core`** forbids `unsafe_code` — it parses hostile input and has never needed
  a raw pointer to do it — and denies `unwrap_used`, `expect_used`, `panic`, `todo`,
  `unimplemented` and `dbg_macro`, which are the ways "errors are values" gets broken by
  accident. It also denies `print_stdout` and `print_stderr`: a library that writes to a
  terminal cannot be embedded by anything.
- **`fontina-platform`** allows `unsafe_code`, because it is the FFI boundary and
  CoreFoundation and the Win32 API are not reachable without it. Every other rule holds.
- **`fontina-cli`** keeps the panic rules and drops the printing ones, printing being
  what it is for.

Tests are exempt from the panic lints. A test that cannot `unwrap` is a test written
around the lint rather than around its subject.

### Exemptions

Use `#[expect(lint, reason = "…")]`, never `#[allow]`. `#[expect]` warns when the lint
stops firing, so an exemption that is no longer needed shows up as a warning instead of
sitting there as paperwork nobody rereads.

The reason names the invariant, and the invariant has to be real. Every exemption in the
tree today points at a guard you can go and read: `split()` returned `Some` only when it
found more than one part; `io::Write` on a `Vec<u8>` cannot fail; `--protocol png`
without `--output` already bailed earlier in the function. If you cannot name the guard,
the code should return an error rather than carry an exemption.

### Comments

Sixteen lines in every hundred are comments, and they earn it by saying why.

- A comment says **why**, not what. The code already says what it does; if that is
  unclear, the fix is the code.
- Where a decision had a plausible alternative, name it and say why it lost. A reader
  who does not know what was rejected will re-propose it.
- Every module opens with `//!` saying what it is for, and where the module encodes a
  judgement rather than a fact, it says so — see `typography.rs`, which states outright
  that all of it is opinion and has to be the same opinion in every client.
- A doc comment on a public item says where the value comes from, not just what it is:
  "weight on the CSS 1–1000 scale (from `OS/2.usWeightClass` or the `wght` default)"
  answers the question the reader is about to ask.

### Naming

- Types are nouns: `FaceMetadata`, `ScriptCoverage`, `EmbeddingRights`. Modules are
  singular nouns for the subject they cover: `model`, `parse`, `index`, `freedom`.
- Functions are verbs, or the noun they produce: `collect_candidates`, `parse_paths`,
  `feature_label`. Predicates begin `is_` or `has_`.
- Conversions read `from_`: `Container::detect`, `EmbeddingRights::from_fs_type`,
  `coverage_from_codepoints`.
- Spell it out in anything public. `codepoints`, not `cps`; `subfamily`, not `subfam`. A
  short local binding inside a ten-line function is fine, because its whole life is
  visible at once.
- Where a name comes from a specification, keep the specification's spelling, even when
  it is ugly: `fs_type`, `usWeightClass`, `wght`, `GSUB`. Renaming it to something nicer
  costs every reader the lookup.

## Testing

Four layers, cheapest first.

1. **Unit and integration tests**, `cargo test`. Hermetic, no system font directory
   touched. Snapshot tests cover every fixture, and every health check id has a case
   that triggers it.
2. **Platform integration tests**, `cargo test --workspace --features
   fontina-platform/platform-tests`. These register a font with the running operating
   system, per user and for the session, and undo it. CI runs them on GNU/Linux, macOS
   and Windows.
3. **Acceptance**, `FONTINA=./target/release/fontina scripts/acceptance`. The whole
   command-line surface end to end, asserted the way a user would: after `activate`,
   `fc-list` and `fc-match` have to see the font, because a font manager that only
   convinces itself has done nothing. Everything it touches is inside one temporary
   XDG home, which it removes on the way out.
4. **Packaging**, `scripts/test-packages`. A release is not a binary, it is a `.deb` and
   an `.rpm` built from `[package.metadata.deb]` and `[package.metadata.generate-rpm]`
   in `crates/fontina-cli/Cargo.toml`, and everything those manifests promise — the
   binary on `PATH`, the man pages where `man` looks, the completions where bash, zsh
   and fish look, the licence where the distribution keeps one, dependencies the archive
   can satisfy — is only a promise until somebody installs one. This script builds the
   packages the way the release workflow does, installs them with `apt-get install` and
   `dnf install` in a clean Debian, Ubuntu and Fedora, runs `scripts/acceptance` against
   the `fontina` the package put on `PATH` (not a binary mounted in: the package is what
   is under test), then removes the package and asserts nothing of ours is left.
   `scripts/package-acceptance` is the half of it that runs inside the container.

GNU/Linux is the reference platform and is not one system, so `scripts/test-distros`
runs the acceptance script inside Debian, Ubuntu, Fedora, Arch, Alpine (musl) and a
Debian with no fontconfig installed. It needs a container runtime; on macOS that is

```
brew install --cask orbstack
```

and Podman or Docker work as well. `.github/workflows/linux.yml` runs the same script
on every pull request that touches the crates, weekly on a schedule, and on `main`.
`scripts/test-packages` needs the same runtime, and the same workflow runs it too.

Adding a capability means adding to whichever layer can prove it: a new metadata field
gets a fixture snapshot, a new health check gets a case in `tests/checks.rs`, a new
activation behaviour gets a platform test, anything a user would type gets a line in
`scripts/acceptance`, and a new file a package installs gets an assertion in
`scripts/package-acceptance`.

## Fixtures

Only fonts under a free license (OFL-1.1, Apache-2.0, CC0) may be added to
`fixtures/`, each under ~500 KB, with the source URL noted in `fixtures/README.md`.

## Licensing

By contributing you agree your work is licensed under `GPL-3.0-or-later`, matching the
project.
