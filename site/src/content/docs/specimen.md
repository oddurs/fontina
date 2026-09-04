---
title: Specimens and CSS
description: "the self-contained HTML specimen and the `@font-face` export."
order: 8
---

## The HTML specimen

```
fontina specimen 1 2 3 -o specimen.html
```

writes one HTML file. It contains, for each face:

- a **waterfall** of the sample text at a range of sizes;
- **script samples**, one paragraph per script the face covers, in that script;
- **axis sliders** for a variable font, with the named instances as snap points, driving
  `font-variation-settings` live;
- **feature toggles**, one per `GSUB`/`GPOS` feature the face declares, driving
  `font-feature-settings`;
- a **glyph map** by Unicode block;
- and, when several faces are given, a **compare** view with them side by side on
  the same text.

The page makes no external requests. The font files are embedded as `data:` URLs so
the file opens from disk, from an email attachment, or from a USB stick, in any
browser, with no server. The one cost is size: the file is at least as big as the
fonts it contains. `--link` references the fonts by path instead, which is small but
needs an HTTP server or a browser that permits `file://` font loads.

`--text` sets the sample text; `--title` the page title.

The specimen renders through whatever browser opens it, so shaping, colour fonts
and variation are as truthful as that browser. This is the same reason the desktop
application will render previews in a webview, and the specimen module is the
reference implementation it will reuse
([ADR 0003](../../adr/0003-tauri-for-the-desktop-shell/)).

## `@font-face` export

```
fontina css 1 2 --url-prefix /fonts/ > fonts.css
```

emits one rule per face:

```
@font-face {
  font-family: "Amiri";
  font-style: normal;
  font-weight: 400;
  font-stretch: 100%;
  font-display: swap;
  src: url("/fonts/Amiri-Regular.ttf") format("truetype");
  unicode-range: U+0020-007E, U+00A0-017F, U+0600-0604, ...;
}
```

The descriptors are the face's CSS Fonts Level 4 descriptor, the same one the index
stores and `info` prints: a numeric `font-weight`, `font-stretch` as a percentage,
`font-style` with an oblique angle when the font declares one, and a
`unicode-range` computed from the character map so the browser can skip the
download for text the font cannot set. A variable font gets a `font-weight` and
`font-stretch` range instead of a single value.

The URL is the file name under `--url-prefix`. Without a prefix, `src` is a
`file://` path, which is what you want for a local specimen and nothing else. The
`format()` hint follows the container: `truetype`, `opentype`, `woff` or `woff2`.

A file path as target emits rules for every face in that file, including every
face of a collection, so an unindexed font can be exported directly.
