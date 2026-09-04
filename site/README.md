# unifont web site

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
