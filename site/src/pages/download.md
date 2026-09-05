---
layout: ../layouts/Page.astro
title: Download
description: "How to get fontina today, what a release archive contains, and how to verify one."
source: site/src/pages/download.md
---

See [Releases](../releases/) for what the current one is and what it contains. Get a release archive below and verify
it, or build from source, which takes one command and a Rust toolchain.

## Release archives

Every release on the [GitHub releases page](https://github.com/oddurs/fontina/releases)
carries one archive per platform:

| Target | Archive |
|---|---|
| GNU/Linux x86_64 | `fontina-vX.Y.Z-x86_64-unknown-linux-musl.tar.gz` |
| GNU/Linux aarch64 | `fontina-vX.Y.Z-aarch64-unknown-linux-musl.tar.gz` |
| macOS Apple silicon | `fontina-vX.Y.Z-aarch64-apple-darwin.tar.gz` |
| macOS Intel | `fontina-vX.Y.Z-x86_64-apple-darwin.tar.gz` |
| Windows x86_64 | `fontina-vX.Y.Z-x86_64-pc-windows-msvc.zip` |

Each archive holds the `fontina` binary (or `fontina.exe`), `COPYING`, the manual
source, a `completions/` directory with bash, zsh, fish and PowerShell completions,
and a `man/` directory with one page per command. Put the binary on your `PATH`;
there is no installer.

The GNU/Linux binaries are statically linked against musl, so one file runs on every
distribution: Debian, Ubuntu, Fedora, Arch, Alpine, and a machine with no fontconfig
installed at all. There is no minimum glibc version to check, because there is no
glibc in it. Each of those is tested on every release.

Linux also gets `.deb` and `.rpm` packages built from the same binary, with the
completions and man pages in their proper places:

```
sudo dpkg -i fontina_X.Y.Z-1_amd64.deb
sudo rpm -i fontina-X.Y.Z-1.x86_64.rpm
```

## Building from source

Requires Rust 1.88 or newer and nothing else: SQLite is bundled, font parsing and
shaping are pure Rust, and the platform backends use only what the operating
system already has.

```
cargo install --git https://github.com/oddurs/fontina fontina-cli
```

Or clone and build:

```
git clone https://github.com/oddurs/fontina
cd fontina
cargo build --release
./target/release/fontina --version
```

The test suite needs nothing beyond the repository; the fixture fonts are checked in:

```
cargo test
```

Completions and man pages come from the binary, so they always match it:

```
fontina completions zsh > ~/.zfunc/_fontina
fontina man --out-dir ~/.local/share/man/man1
```

## Verifying a download

Three things are published beside every archive and package.

1. **A SHA-256 checksum**, in `<file>.sha256`:

   ```
   sha256sum -c fontina-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz.sha256
   ```

2. **A SLSA build provenance attestation**, signed through GitHub's Sigstore
   instance. It proves the archive was built by the release workflow in this
   repository from the tagged commit, not on someone's laptop:

   ```
   gh attestation verify fontina-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz --repo oddurs/fontina
   ```

3. **An SPDX software bill of materials**, `fontina-vX.Y.Z.spdx.json`, listing every
   crate compiled into the binary and its license.

Nothing is signed by hand and nothing is uploaded by hand. If a release artifact does
not verify, do not run it, and [report it](../security/).

## Distribution packages

Planned before 1.0: a Homebrew formula, winget and Scoop manifests, and an AUR
`PKGBUILD`. Packagers are welcome; the build has no network access at build time, no
vendored C libraries except bundled SQLite, and honours `CARGO_TARGET_DIR` and the
usual `cargo` environment.

## Old versions

All releases stay on GitHub. Only the latest release and the `main` branch receive
fixes; see the [security policy](../security/).
