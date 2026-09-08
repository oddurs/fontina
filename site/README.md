# fontina web site

The project site and manual: plain HTML, no JavaScript, one stylesheet. Built with
[Astro](https://astro.build) so the manual can live in Markdown and the architecture
decision records in `../docs/adr` are published without copying them.

```
npm install
npm run dev        # http://localhost:4321/
npm run build      # writes dist/
```

Deployed to GitHub Pages by `.github/workflows/site.yml` on every push to `main` that
touches `site/` or `docs/adr/`. The workflow sets `SITE_URL` and `SITE_BASE`.

Layout:

```
src/content/docs/    the manual, one Markdown file per chapter, ordered by `order`
src/content/news/    dated announcements; also published as RSS at /feed.xml
src/pages/           top-level pages (download, contributing, bugs, security, license ...)
src/layouts/         the single page layout
src/styles/          the single stylesheet
public/              robots.txt, .well-known/security.txt, favicon
```

Rules: no client-side script, no external requests, no web fonts, no analytics. Every
page must read in a text browser.

## License

Two licenses, and which one applies depends on whether a file is a program or a
document. That is the same split the rest of the repository uses; it was simply never
stated here, and every file under `src/` carried no notice at all.

**The code** — the Astro components, the layouts, the build scripts, the stylesheet —
is GPL-3.0-or-later, like the crates. Every file now carries the notice, and
`package.json` declares it. [ADR 0007](../docs/adr/0007-license-gpl-3.md) is why the
project is copyleft.

**The prose** — everything under `src/content/` and the Markdown pages under
`src/pages/` — is under the GNU Free Documentation License 1.3 or later, with no
Invariant Sections, no Front-Cover Texts and no Back-Cover Texts. These pages are the
manual, and the manual is GFDL; a copy is in [`docs/COPYING.DOC`](../docs/COPYING.DOC).
Verbatim copying and distribution of any page are permitted in any medium, provided the
copyright notice and this permission notice are preserved.

The Markdown files carry no per-file notice because a license header in frontmatter
would be rendered onto the page or would have to be taught to the content schema. This
paragraph is the notice for all of them, and `/license/` says the same thing to a
reader.

Nothing here is a new decision. It records what `COPYING`, `docs/COPYING.DOC` and the
site's own [license page](src/pages/license.md) already said, in the one place a tool or
a packager would look.
