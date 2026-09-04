# Contributing

Thanks for helping build unifont. This page is the git workflow; `CLAUDE.md` holds the
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
- CI (`.github/workflows/ci.yml`) must pass on Linux, macOS and Windows.
- Self-review the diff before requesting review.
- Squash-merge only. Delete the branch after merge (automatic).

## Automated merges

The `automerge` workflow squash-merges a PR once every other check on its head
commit has passed. It applies to Dependabot minor and patch updates automatically
and to any PR a maintainer labels `automerge`. Major dependency updates and release
PRs are always merged by a person.

## Releases

[release-please](https://github.com/googleapis/release-please) reads the commit history
on `main`, maintains `CHANGELOG.md`, and opens a release PR that bumps the workspace
version in `Cargo.toml` (the crates inherit it). The workflow then syncs `Cargo.lock`
on the release branch and dispatches CI there so the required checks exist. Merging that PR tags `vX.Y.Z`. A release workflow that builds signed binaries for the tag is planned for M1.
Nothing is published by hand.

## Fixtures

Only fonts under an open license (OFL-1.1, Apache-2.0, UFL-1.0, CC0) may be added to
`fixtures/`, each under ~500 KB, with the source URL noted in `fixtures/README.md`.

## Licensing

By contributing you agree your work is licensed under `MIT OR Apache-2.0`, matching the
project.
