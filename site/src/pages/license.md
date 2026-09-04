---
layout: ../layouts/Page.astro
title: License
description: unifont is free software under MIT OR Apache-2.0. The fixture fonts are under the SIL Open Font License.
source: site/src/pages/license.md
---

unifont is free software. You may use, study, copy, modify and redistribute it, for
any purpose, under either of the following licenses, at your option:

- the [Apache License, Version 2.0](https://github.com/oddurs/unifont/blob/main/LICENSE-APACHE)
  (SPDX: `Apache-2.0`), or
- the [MIT license](https://github.com/oddurs/unifont/blob/main/LICENSE-MIT)
  (SPDX: `MIT`).

The SPDX expression is `MIT OR Apache-2.0`. Unless you explicitly state otherwise, any
contribution intentionally submitted for inclusion in the work by you, as defined in
the Apache-2.0 license, is dual licensed as above, without any additional terms or
conditions. The reasoning is in [ADR 0004](../adr/0004-license-mit-or-apache/).

## Fixture fonts

The test fonts in `fixtures/` keep their own licenses, all of them open:

| File | License | Source |
|---|---|---|
| `Amiri-Regular.ttf` | OFL-1.1 | google/fonts `ofl/amiri` |
| `BricolageGrotesque[opsz,wdth,wght].ttf` | OFL-1.1 | google/fonts `ofl/bricolagegrotesque` |
| `Nabla[EDPT,EHLT].ttf` | OFL-1.1 | google/fonts `ofl/nabla` |
| `SourceSerif4-Regular.otf` | OFL-1.1 | adobe-fonts/source-serif |
| `inter-latin-400-normal.woff` | OFL-1.1 | Fontsource `@fontsource/inter` |
| `inter-latin-400-normal.woff2` | OFL-1.1 | Fontsource `@fontsource/inter` |

Only fonts under OFL-1.1, Apache-2.0, UFL-1.0 or CC0 may be added.

## Dependencies

Every crate compiled into a release is listed, with its license, in the SPDX software
bill of materials attached to that release. `cargo deny` enforces the allowed license
list in continuous integration.

## This web site

The text and markup of this site are part of the repository and carry the same
`MIT OR Apache-2.0` terms. In addition, verbatim copying and distribution of any
page of this site are permitted in any medium, provided the copyright notice and
this permission notice are preserved.

## Trademarks

"unifont" is a working name and no trademark is claimed on it. GNU Unifont is a
separate project of the Free Software Foundation.
