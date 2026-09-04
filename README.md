# unifont

[![ci](https://github.com/oddurs/unifont/actions/workflows/ci.yml/badge.svg)](https://github.com/oddurs/unifont/actions/workflows/ci.yml)
[![license](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)
[![msrv](https://img.shields.io/badge/MSRV-1.88-orange.svg)](Cargo.toml)

A lightweight, cross-platform, open-source font manager. Rust core, thin native shell,
open standards end to end.

> Working codename. "Unifont" collides with GNU Unifont and will be renamed before the
> first release.

**Status:** M0. The core library and CLI parse TTF, OTF, TTC, WOFF and WOFF2, build a
searchable SQLite index, and export `@font-face` CSS. The desktop app and native
activation backends are next; see [`PLAN.md`](PLAN.md) for the roadmap.

## Install

From source (Rust 1.88 or newer):

```
cargo install --git https://github.com/oddurs/unifont unifont-cli
```

Release binaries for Linux, macOS and Windows are attached to each
[GitHub release](https://github.com/oddurs/unifont/releases) with SHA-256 checksums,
SLSA provenance attestations and an SPDX SBOM. Verify with
`gh attestation verify <archive> --repo oddurs/unifont`.

## Use

```
unifont scan --system            # index the OS font directories
unifont scan ~/Fonts             # and your own
unifont list --script Arab       # faces that cover Arabic
unifont list --variable bold     # variable faces matching "bold"
unifont list --license OFL       # by SPDX identifier
unifont info 42                  # everything about a face
unifont dupes                    # same font in several files
unifont css 42 --url-prefix /fonts/ > fonts.css
unifont schema                   # JSON Schema for the metadata
```

Every command takes `--json`. Set `UNIFONT_DB` to choose the index location. The
index is a single SQLite file in the platform data directory.

## Why

Existing managers are Electron-heavy, single-platform, closed, or all three. unifont is
a reusable Rust crate first, a CLI second, and a desktop app third.

- **Correct.** Parsing by Google's [fontations](https://github.com/googlefonts/fontations),
  the code Chrome and Skia use. Families come from the typographic name IDs and `STAT`.
- **Standards.** CSS Fonts Level 4 style model, SPDX license identifiers, XDG paths,
  fontconfig integration, JSON Schema for every export, TOML config.
- **Light.** Hard budgets on install size, start time and idle memory, enforced in CI.
- **Private.** No network calls, no telemetry, no elevation, no writes to system font
  directories.

## Contributing

Read [`CONTRIBUTING.md`](CONTRIBUTING.md) for the branch and commit conventions and
[`CLAUDE.md`](CLAUDE.md) for the engineering rules. Architecture decisions are recorded
in [`docs/adr`](docs/adr). Security issues go through [`SECURITY.md`](SECURITY.md).
This project follows the [Contributor Covenant](CODE_OF_CONDUCT.md).

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option. Unless you explicitly state otherwise, any
contribution intentionally submitted for inclusion in the work by you, as defined in
the Apache-2.0 license, shall be dual licensed as above, without any additional terms
or conditions.

Fixture fonts are under the SIL Open Font License 1.1; see
[`fixtures/README.md`](fixtures/README.md).
