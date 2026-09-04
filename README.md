# unifont

[![ci](https://github.com/oddurs/unifont/actions/workflows/ci.yml/badge.svg)](https://github.com/oddurs/unifont/actions/workflows/ci.yml)
[![license](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)
[![msrv](https://img.shields.io/badge/MSRV-1.88-orange.svg)](Cargo.toml)

A lightweight, cross-platform, open-source font manager. Rust core, thin native shell,
open standards end to end.

> Working codename. "Unifont" collides with GNU Unifont and will be renamed before the
> first release.

**Status:** core library and CLI. They parse TTF, OTF, TTC, WOFF and WOFF2, build a
searchable SQLite index, run health checks, answer "which fonts cover this text",
export `@font-face` CSS and interactive HTML specimens. Next (M1): tags and
collections, native activation on all three desktops, watched folders, shaped glyph
previews in the terminal, and a TUI. See [`PLAN.md`](PLAN.md) for the roadmap and
principles.

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
unifont families --script Cyrl   # grouped by typographic family
unifont facets --tag serif       # counts per weight, width, script, license, vendor, ...
unifont tag add serif family:Amiri 42
unifont collection add Editorial family:Amiri 42
unifont collection export Editorial > editorial.json   # schemas/collection.json
unifont source add ~/Fonts       # scan now, follow with `watch` later
unifont dupes                    # same font in several files
unifont css 42 --url-prefix /fonts/ > fonts.css
unifont covers "Þórður át 12 blóðbergsbrauð"   # faces that can set this text
unifont check ~/Fonts/*.ttf      # health checks; exit 1 on errors
unifont license                  # SPDX, embedding rights, reserved font names
unifont glyphs 42 --block arabic # coverage by Unicode block
unifont specimen 42 43 -o specimen.html   # waterfall, axis sliders, feature toggles, compare
unifont schema                   # JSON Schema for the metadata
```

Every command takes `--json`; the output types are published in
`schemas/cli-output.json`. A face target is an index id, a file path, or `family:<name>`.
Set `UNIFONT_DB` to choose the index location. The index is a single SQLite file in the
platform data directory.

## Why

Existing managers are Electron-heavy, single-platform, closed, or all three. unifont is
a reusable Rust crate first, a CLI second, and a terminal UI third. The font manager for
the people: one small binary, your data in plain files, nothing leaves the machine.

- **Correct.** Parsing by Google's [fontations](https://github.com/googlefonts/fontations),
  the code Chrome and Skia use. Families come from the typographic name IDs and `STAT`.
- **Standards.** CSS Fonts Level 4 style model, SPDX license identifiers, XDG paths,
  fontconfig integration, JSON Schema for every export, TOML config.
- **Light.** Hard budgets on install size, start time and idle memory, enforced in CI.
- **Private.** No network calls, no telemetry, no accounts, no elevation, no writes to
  system font directories.
- **Durable.** Append-only migrations, stable output, few audited dependencies. A script
  written today keeps working.

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
