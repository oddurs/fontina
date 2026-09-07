---
id: 39
title: Get the credentials the package managers need
type: chore
status: ready
milestone: m5-ship
assignee: oddurs
created: 2026-09-06
updated: 2026-09-06
priority: p0
effort: s
crate: workspace
---

## Problem

The manifests exist and are correct (#116). Nothing publishes them, and nothing can:
every publishing step needs a credential this repository does not have. This is the
only thing standing between a reviewed manifest and `brew install fontina` working.

It is filed as an item because it is work, it is blocking, and the person who has to
do it is not the person writing the code — which is precisely the state that gets lost
in a chat log.

## Proposal

Three secrets, in descending order of what they unblock.

**1. `PACKAGING_TOKEN` — the tap and the bucket.**

A fine-grained token at <https://github.com/settings/personal-access-tokens/new>:

- Repository access: *Only select repositories* → `homebrew-fontina`, `scoop-fontina`
- Permissions → Repository permissions → **Contents: Read and write**

```sh
gh secret set PACKAGING_TOKEN --repo oddurs/fontina
```

Scoped to those two repositories and nothing else.

**2. `AUR_SSH_KEY` — the AUR.**

Both names are free; the AUR API reports no `fontina` and no `fontina-bin`. The package
is created by its first push, so there is nothing to register beyond the account.

```sh
ssh-keygen -t ed25519 -C "aur@fontina" -f ~/.ssh/aur_fontina -N ""
cat ~/.ssh/aur_fontina.pub          # paste into aur.archlinux.org → My Account
gh secret set AUR_SSH_KEY --repo oddurs/fontina < ~/.ssh/aur_fontina
```

No passphrase, because CI has to use it unattended.

**3. winget — deliberately not automated.**

`wingetcreate submit` forks `microsoft/winget-pkgs`, which a fine-grained token cannot
do; it needs a *classic* token with `public_repo`, and that grants write to every public
repository on the account. For a package that updates a few times a year, submitting by
hand is the better trade. Revisit if the cadence changes.

## Acceptance criteria

- [ ] `PACKAGING_TOKEN` exists as a repository secret
- [ ] `AUR_SSH_KEY` exists as a repository secret, and the public half is on the AUR account
- [ ] a decision recorded on winget: automated with a classic token, or submitted by hand
