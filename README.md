# unifont

A lightweight, cross-platform, open-source font manager. Rust core, thin native shell,
open standards end to end.

> Working codename. "Unifont" collides with GNU Unifont and will be renamed before the
> first release.

**Status:** M0. The core library and CLI parse TTF, OTF, TTC, WOFF and WOFF2, build a
searchable SQLite index, and export `@font-face` CSS. See `PLAN.md` for the roadmap.

```
cargo install --path crates/unifont-cli
unifont scan --system            # index the OS font directories
unifont scan ~/Fonts             # and your own
unifont list --script Arab       # faces that cover Arabic
unifont list --variable bold     # variable faces matching "bold"
unifont info 42                  # everything about a face
unifont dupes                    # same font in several files
unifont css 42 --url-prefix /fonts/ > fonts.css
```

Every command takes `--json`. Set `UNIFONT_DB` to choose the index location.

## Why

Existing managers are Electron-heavy, single-platform, closed, or all three. unifont is
a reusable Rust crate first, a CLI second, and a desktop app third, with parsing by
Google's [fontations](https://github.com/googlefonts/fontations), a CSS Fonts Level 4
style model, SPDX license identifiers, and JSON Schema for every export.

## License

MIT OR Apache-2.0. Fixture fonts are OFL-1.1; see `fixtures/README.md`.
