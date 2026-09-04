---
layout: ../layouts/Page.astro
title: Frequently asked questions
description: "Answers to common questions about fontina."
source: site/src/pages/faq.md
---

1. [Is this GNU Unifont?](#is-this-gnu-fontina)
2. [What does it do that `fc-list` does not?](#what-does-it-do-that-fc-list-does-not)
3. [Does it need root, or write to `/usr/share/fonts`?](#does-it-need-root-or-write-to-usrsharefonts)
4. [Does it phone home?](#does-it-phone-home)
5. [Where is the database?](#where-is-the-database)
6. [Can it edit or convert fonts?](#can-it-edit-or-convert-fonts)
7. [Why a webview for the desktop app instead of a native toolkit?](#why-a-webview-for-the-desktop-app-instead-of-a-native-toolkit)
8. [Why Rust?](#why-rust)
9. [Why is the license MIT OR Apache-2.0 and not the GPL?](#why-is-the-license-mit-or-apache-20-and-not-the-gpl)
10. [Which fonts ship with it?](#which-fonts-ship-with-it)
11. [The variable font's style shows as "96pt ExtraBold". Is that right?](#the-variable-fonts-style-shows-as-96pt-extrabold-is-that-right)
12. [A WOFF2 file fails with an `hmtx` transform error.](#a-woff2-file-fails-with-an-hmtx-transform-error)
13. [How do I report a bug?](#how-do-i-report-a-bug)

---

### Is this GNU Unifont?

No. GNU Unifont is a bitmap font covering the Basic Multilingual Plane. This is a font
manager that happens to have picked the same word as a working name. The name will
change before the first release. We apologise for the confusion in the meantime.

### What does it do that `fc-list` does not?

`fc-list` tells you what fontconfig sees. fontina reads the files themselves and keeps
what it finds in a database: every name record, the `OS/2` classification and
embedding rights, variable axes and named instances, the OpenType features and
scripts, the full character map, colour tables, hinting, license text mapped to an
SPDX identifier, and a content hash so the same font in two containers is recognised
as one. Then it answers queries against that, exports it, and checks it.

### Does it need root, or write to `/usr/share/fonts`?

Never. Everything is per-user. On Linux the index lives under
`$XDG_DATA_HOME/fontina` and activation, when it lands, will use
`$XDG_DATA_HOME/fonts` and a fontconfig fragment in `~/.config/fontconfig/conf.d`.

### Does it phone home?

No. There is no network code in the core or the command line. The desktop application
will have an opt-in font catalog; until you turn it on it makes no connections either.
There is no telemetry, no crash reporting, and no update check.

### Where is the database?

One SQLite file in the platform data directory: `$XDG_DATA_HOME/fontina/index.db` on
Linux, `~/Library/Application Support/fontina` on macOS, `%APPDATA%\fontina` on
Windows. Set `FONTINA_DB` or pass `--db` to put it somewhere else. You can open it
with any SQLite client. Deleting it loses nothing that a rescan will not recreate,
except tags, collections and source registrations.

### Can it edit or convert fonts?

No, and it will not. Subsetting, format conversion and editing are
[fonttools](https://github.com/fonttools/fonttools)' job and it does it well. fontina
reads.

### Why a webview for the desktop app instead of a native toolkit?

Because previews must be truthful. The system webview shapes Arabic and Devanagari,
renders `COLRv1` colour fonts, applies `font-variation-settings` and
`font-feature-settings`, and does it with the same text stack every other program on
the machine uses. Rebuilding that in a Rust GUI toolkit would take years and still
lag. The trade-off is recorded in [ADR 0003](../adr/0003-tauri-for-the-desktop-shell/).

### Why Rust?

Font files are untrusted input. The parser, fontations, is memory-safe and fuzzed
continuously. The whole tool is one static binary with no runtime.

### Why is the license MIT OR Apache-2.0 and not the GPL?

It is the Rust ecosystem convention, it lets the core crate be used by anyone, and
Apache-2.0 carries an explicit patent grant. See
[ADR 0004](../adr/0004-license-mit-or-apache/). The project is free software either way,
and contributions are accepted under the same terms.

### Which fonts ship with it?

None. The repository carries six small fixture fonts for the test suite, all under the
SIL Open Font License or Apache-2.0, listed with their sources in `fixtures/README.md`.

### The variable font's style shows as "96pt ExtraBold". Is that right?

Yes. For a variable font the listed style is the default instance, resolved from the
`fvar` default coordinates and the named instances. Named instances and axis ranges
are all in `fontina info`.

### A WOFF2 file fails with an `hmtx` transform error.

The pure-Rust WOFF2 decoder does not implement the optional `hmtx` transform. Such
files are rare in practice. They are counted under `failed` in `fontina stats`. If they
turn out to be common, a fallback to Google's reference decoder is the planned
mitigation ([ADR 0005](../adr/0005-woff-decoding/)).

### How do I report a bug?

See [Bugs](../bugs/). Parser crashes are security bugs; see [Security](../security/).
