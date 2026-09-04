---
layout: ../layouts/Page.astro
title: License
description: "fontina is free software under the GNU GPL version 3 or later. The manual is under the GFDL. The fixture fonts are under the SIL Open Font License."
source: site/src/pages/license.md
---

fontina is free software. You may run it for any purpose, study how it works, change
it, and redistribute copies with or without your changes, under the terms of the
[GNU General Public License](https://github.com/oddurs/fontina/blob/main/COPYING) as
published by the Free Software Foundation, either version 3 of the License or, at your
option, any later version. The SPDX expression is `GPL-3.0-or-later`.

It is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without
even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See
the GNU General Public License for more details.

Copyleft is a deliberate choice, not a default. A permissive license would let a
proprietary font manager take this code, ship it inside a paid tier, and give its users
none of the freedoms this project is built to protect. The reasoning is set out in
[ADR 0007](../adr/0007-license-gpl-3/), which supersedes
[ADR 0004](../adr/0004-license-mit-or-apache/).

Contributions are accepted under the same terms. There is no contributor licence
agreement and no copyright assignment: contributors keep their copyright, which is what
makes the license hard for any single party to revoke later.

## Documentation

The manual — the man page, the Texinfo manual, and the documents in this repository —
is under the [GNU Free Documentation License](https://www.gnu.org/licenses/fdl-1.3.html)
version 1.3 or later, with no Invariant Sections, no Front-Cover Texts and no Back-Cover
Texts. A copy is in `docs/COPYING.DOC`.

## Fixture fonts

The test fonts in `fixtures/` keep their own licenses, all of them free:

| File | License | Source |
|---|---|---|
| `Amiri-Regular.ttf` | OFL-1.1 | google/fonts `ofl/amiri` |
| `BricolageGrotesque[opsz,wdth,wght].ttf` | OFL-1.1 | google/fonts `ofl/bricolagegrotesque` |
| `Nabla[EDPT,EHLT].ttf` | OFL-1.1 | google/fonts `ofl/nabla` |
| `SourceSerif4-Regular.otf` | OFL-1.1 | adobe-fonts/source-serif |
| `inter-latin-400-normal.woff` | OFL-1.1 | Fontsource `@fontsource/inter` |
| `inter-latin-400-normal.woff2` | OFL-1.1 | Fontsource `@fontsource/inter` |

Only fonts under a free license — OFL-1.1, Apache-2.0 or CC0 — may be added.

## Your fonts

fontina applies the same question to the fonts it indexes. `fontina list --free` shows
only the fonts whose licenses grant you the four freedoms; `fontina license` gives the
verdict and the reason for every font in your library. Where a license has never been
ruled free, the program says so rather than guessing.

`OS/2.fsType` embedding bits are reported and never enforced. They are the font file's
assertion about itself, not a term of any license, and acting on them would mean
restricting you on your own computer.

## Dependencies

Every crate compiled into a release is listed, with its license, in the SPDX software
bill of materials attached to that release. `cargo deny` enforces the allowed license
list in continuous integration; every entry on it is GPLv3-compatible.

## This web site

The text and markup of this site are part of the repository. The prose is under the GNU
Free Documentation License 1.3 or later, as above; verbatim copying and distribution of
any page are permitted in any medium, provided the copyright notice and this permission
notice are preserved.

## Trademarks

No trademark is claimed on the name. GNU Unifont, a bitmap font that shares this
project's former working name, is a separate project of the Free Software Foundation
and is not connected to it.
