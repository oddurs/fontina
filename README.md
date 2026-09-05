# fontina

[![ci](https://github.com/oddurs/fontina/actions/workflows/ci.yml/badge.svg)](https://github.com/oddurs/fontina/actions/workflows/ci.yml)
[![license](https://img.shields.io/badge/license-GPL--3.0--or--later-blue.svg)](#license)
[![msrv](https://img.shields.io/badge/MSRV-1.88-orange.svg)](Cargo.toml)

A lightweight, cross-platform font manager, and free software: you may run it, study it,
change it and pass it on. Rust core, thin native shell, open standards end to end.

> Named for the cheese, one letter from the thing it manages. The project was called
> `fontina` until 2026-09-04; that name collided with GNU Unifont, a bitmap font
> maintained under the GNU project, and was dropped for it.

**Status:** M1 done. The core library and CLI parse TTF, OTF, TTC, WOFF and WOFF2,
build a searchable SQLite index with tags and collections, activate and install fonts
per user on Linux, macOS and Windows, follow watched folders, run health checks, answer
"which fonts cover this text", show shaped glyph previews in the terminal, export
`@font-face` CSS and HTML specimens, and come with a keyboard-first TUI. See
[`PLAN.md`](PLAN.md) for the roadmap and principles.

Documentation, the manual and the roadmap are at
[oddurs.github.io/fontina](https://oddurs.github.io/fontina/).

## Install

From source (Rust 1.88 or newer):

```
cargo install --git https://github.com/oddurs/fontina fontina-cli
```

Release binaries for Linux, macOS and Windows are attached to each
[GitHub release](https://github.com/oddurs/fontina/releases) with SHA-256 checksums,
SLSA provenance attestations and an SPDX SBOM, plus `.deb` and `.rpm` packages for
Linux. Every archive carries shell completions and man pages; `fontina completions
<shell>` and `fontina man` print them too. Verify with
`gh attestation verify <archive> --repo oddurs/fontina`.

## Use

```
fontina scan --system            # index the OS font directories
fontina scan ~/Fonts             # and your own
fontina list --script Arab       # faces that cover Arabic
fontina list --variable bold     # variable faces matching "bold"
fontina list --license OFL       # by SPDX identifier
fontina list --free              # only fonts you may study, change and pass on
fontina list --freedom unknown   # licenses nobody has ruled free
fontina info 42                  # everything about a face
fontina families --script Cyrl   # grouped by typographic family
fontina facets --tag serif       # counts per weight, width, script, license, vendor, ...
fontina tag add serif family:Amiri 42
fontina collection add Editorial family:Amiri 42
fontina collection export Editorial > editorial.json   # schemas/collection.json
fontina source add ~/Fonts       # scan now, follow with `watch` later
fontina activate family:Amiri    # visible to every app, in place, per user; --session until logout
fontina conflicts 42             # same name already active or in an OS font directory? exit 2
fontina install 42 --replace     # copy into the per-user font directory
fontina restore                  # re-apply activations after a reboot
fontina agent install            # have the OS run restore at login; off until asked for
fontina preview 42 -t "Sphinx of black quartz" -a wght=700 -f smcp   # shaped glyphs, in the terminal
fontina preview 42 -o specimen.png            # or as a PNG
fontina ui                       # browse: facets, families, previews, tag and activate
fontina dupes                    # same font in several files
fontina css 42 --url-prefix /fonts/ > fonts.css
fontina covers "Þórður át 12 blóðbergsbrauð"   # faces that can set this text
fontina check ~/Fonts/*.ttf      # health checks; exit 1 on errors
fontina license                  # SPDX, embedding rights, reserved font names
fontina glyphs 42 --block arabic # coverage by Unicode block
fontina specimen 42 43 -o specimen.html   # waterfall, axis sliders, feature toggles, compare
fontina schema                   # JSON Schema for the metadata
```

Every command takes `--json`; the output types are published in
`schemas/cli-output.json`. A face target is an index id, a file path, or `family:<name>`.
Set `FONTINA_DB` to choose the index location. The index is a single SQLite file in the
platform data directory.

In `list`, the `flags` column reads `V` variable, `C` color, `I` italic, then the
activation state (`s` session, `u` user, `i` installed), then the freedom of the
license: `F` free, `N` nonfree, `?` a license nobody has ruled on, `-` none stated.

`fontina man` writes the man pages and `fontina completions` the shell completions;
the full manual is [`docs/fontina.texi`](docs/fontina.texi) (`info fontina`), which is
where the freedom rules and the file layout are written out.

## Why

Existing managers are Electron-heavy, single-platform, closed, or all three. fontina is
a reusable Rust crate first, a CLI second, and a terminal UI third. The font manager for
the people: one small binary, your data in plain files, nothing leaves the machine.

- **Correct.** Parsing by Google's [fontations](https://github.com/googlefonts/fontations),
  the code Chrome and Skia use. Families come from the typographic name IDs and `STAT`.
- **Standards.** CSS Fonts Level 4 style model, SPDX license identifiers, XDG paths,
  fontconfig integration, JSON Schema for every export, TOML config.
- **Light.** Hard budgets on install size, start time and idle memory, enforced in CI.
- **Private.** No network calls, no telemetry, no accounts, no elevation, no writes to
  system font directories.
- **Free, and it says which of your fonts are.** GPL-3.0-or-later, so nobody downstream
  can take these freedoms away from the next person. `--free` filters your library to
  the fonts you may actually study, modify and pass on; `fontina license` gives the
  verdict and the reason for each one. `OS/2.fsType` embedding bits are reported and
  never enforced — they are the font file's assertion, not a term of any license.
- **Durable.** Append-only migrations, stable output, few audited dependencies. A script
  written today keeps working.

## Contributing

Read [`CONTRIBUTING.md`](CONTRIBUTING.md) for the branch and commit conventions and
[`CLAUDE.md`](CLAUDE.md) for the engineering rules. Architecture decisions are recorded
in [`docs/adr`](docs/adr). Security issues go through [`SECURITY.md`](SECURITY.md).
This project follows the [Contributor Covenant](CODE_OF_CONDUCT.md).

## License

fontina is free software: you can redistribute it and/or modify it under the terms of
the GNU General Public License as published by the Free Software Foundation, either
version 3 of the License, or (at your option) any later version. The full text is in
[`COPYING`](COPYING); the reasoning is in [ADR 0007](docs/adr/0007-license-gpl-3.md).

It is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without
even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.

Contributions are accepted under the same terms. There is no CLA and no copyright
assignment: you keep your copyright.

The documentation — this file, `PLAN.md`, `docs/`, the man page and the Texinfo manual —
is under the GNU Free Documentation License 1.3 or later, with no invariant sections and
no cover texts; see [`docs/COPYING.DOC`](docs/COPYING.DOC).

Fixture fonts are under the SIL Open Font License 1.1; see
[`fixtures/README.md`](fixtures/README.md).
