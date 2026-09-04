---
layout: ../layouts/Page.astro
title: Download
description: Release binaries, checksums, provenance verification, and building unifont from source.
source: site/src/pages/download.md
---

unifont is pre-release software. Version 0.0.1 has the core library and the command
line; see the [roadmap](../roadmap/) for what is next.

## Release binaries

Every tagged release on
[github.com/oddurs/unifont/releases](https://github.com/oddurs/unifont/releases)
carries an archive per platform:

| Target | Archive |
|---|---|
| Linux x86_64 (glibc) | `unifont-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz` |
| Linux aarch64 (glibc) | `unifont-vX.Y.Z-aarch64-unknown-linux-gnu.tar.gz` |
| macOS Apple silicon | `unifont-vX.Y.Z-aarch64-apple-darwin.tar.gz` |
| macOS Intel | `unifont-vX.Y.Z-x86_64-apple-darwin.tar.gz` |
| Windows x86_64 | `unifont-vX.Y.Z-x86_64-pc-windows-msvc.zip` |

Each archive holds one statically linked binary named `unifont` (or `unifont.exe`).
Put it on your `PATH`. There is no installer and nothing else to install.

## Verifying a download

Three things are published beside every archive.

1. **A SHA-256 checksum**, in `<archive>.sha256`:

   ```
   sha256sum -c unifont-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz.sha256
   ```

2. **A SLSA build provenance attestation**, signed by GitHub's Sigstore instance. It
   proves the archive was built by the release workflow in this repository from the
   tagged commit, not on someone's laptop:

   ```
   gh attestation verify unifont-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz --repo oddurs/unifont
   ```

3. **An SPDX software bill of materials**, `unifont-vX.Y.Z.spdx.json`, listing every
   crate compiled into the binary and its license.

Nothing is signed by hand and nothing is uploaded by hand. If a release artifact does
not verify, do not run it, and [report it](../security/).

## Building from source

Requires Rust 1.88 or newer. No other build dependency: SQLite is bundled, and font
parsing is pure Rust.

Install the command line straight from the repository:

```
cargo install --git https://github.com/oddurs/unifont unifont-cli
```

Or clone and build:

```
git clone https://github.com/oddurs/unifont
cd unifont
cargo build --release
./target/release/unifont --version
```

The test suite needs nothing beyond the repository; the fixture fonts are checked in:

```
cargo test
```

## Distribution packages

None yet. Flathub, Homebrew and winget packaging are planned for the desktop
application milestone. Packagers are welcome; the build has no network access at
build time, no vendored C libraries except bundled SQLite, and honours
`CARGO_TARGET_DIR` and the usual `cargo` environment.

## Old versions

All releases stay on GitHub. Only the latest release and the `main` branch receive
fixes; see the [security policy](../security/).
