# unifont web design system

Derived from the gcc.gnu.org stylesheet, kept as-is where it is good and cleaned
where it is only old. The stylesheet `src/styles/site.css` implements exactly what is
here, section for section; if you add something, add it to both.

Rules that hold on every page: light mode only; the browser's default serif; no web
fonts; no client-side script; no external requests; one stylesheet; every page must
read in a text browser. Colour carries hierarchy, never meaning on its own.

## Tokens

| Token | Value | Use |
|---|---|---|
| `--bg` | `#ffffff` | page background |
| `--fg` | `#000000` | body text |
| `--heading` | `darkslategray` (`#2f4f4f`) | h1, h2, h3, `.highlight`, news titles |
| `--link` | `#0066bb` | links |
| `--link-visited` | `#003399` | visited links |
| `--link-hover` | `darkorange` | hovered links |
| `--accent` | `#0066dd` | nav item header background |
| `--accent-fg` | `#f2f2f9` | nav item header text |
| `--rule` | `#3366cc` | thin borders: nav header, status column, copyright box |
| `--panel` | `#f2f2f9` | panel backgrounds: nav body, copyright box, examples, node nav |
| `--border` | `gray` | table borders |
| `--gap` | `32px` | gap between the main column and the nav column |
| `--pad` | `4px` | inner padding of small boxes |
| `--pad-2` | `8px` | inner padding of nav bodies and table cells |

Type: `font-family: serif`, base size 100%, `line-height` left to the browser. Two
smaller steps: `.small` 90% (dates) and `.smaller` 80% (nav bodies, copyright,
regress lines). Headings are bold in `--heading`; `h1` is centred, `h2` inside a
column panel is 1.2em with no top margin.

Links are not underlined. This is the one thing the browser default gets wrong for a
page this link-dense; the colour and hover carry it.

## Components

Each one is a class (or element) in the stylesheet, with the markup it expects.

### Frame

```
<div class="frame">
  <main class="main">...</main>
  <nav class="navcol">...</nav>
</div>
<div class="copyright">...</div>
```

Two columns: main content and a narrow nav column on the right, separated by
`--gap`, at most 72em wide, centred. Under 50em the nav column moves below the
content. The copyright box spans both.

### Nav item

```
<div class="navitem">
  <div class="navhead">Documentation</div>
  <div class="navbody">
    <a href="...">Manual</a><br>
    &middot;&nbsp;<a href="...">Command reference</a><br>
  </div>
</div>
```

A bold header bar in `--accent` on `--accent-fg` with a thin `--rule` border, and a
body in `--panel` at `.smaller`. One link per line. A `&middot;&nbsp;` prefix marks a
sub-item. This is the whole site navigation; there is no top bar.

### Columns

```
<div class="columns">
  <section class="col news">...</section>
  <section class="col status">...</section>
</div>
```

Two equal columns for the front page. `.status` has a thin `--rule` left border and
inner left padding. Each column starts with an `h2` at 1.2em.

### News list

```
<dl class="news">
  <dt><a href="...">Title</a> <span class="date">[2026-09-03]</span></dt>
  <dd>One line of detail, or nothing.</dd>
</dl>
```

`dt` bold in `--heading`; the date in `.small` `--heading` inside square brackets;
`dd` indented 3ex with tight vertical margins.

### Status list

```
<dl class="status">
  <dt><span class="version"><a href="...">unifont 0.0.1</a></span> (<a href="...">changes</a>)</dt>
  <dd>Status: <a href="...">2026-09-03</a> (pre-release). <div class="regress">...</div></dd>
</dl>
```

`.version` bold; `.regress` at `.smaller`.

### Node nav

```
<p class="node-nav">Next: <a>...</a>, Previous: <a>...</a>, Up: <a>Manual</a></p>
```

Texinfo-style navigation line above and below manual chapters, in a `--panel`
box at `.smaller`.

### Copyright box

```
<div class="copyright">
  <address>Where to ask for help.</address>
  <p>Copyright and the verbatim-copying notice.</p>
  <p>Who maintains the pages and when they were last modified.</p>
</div>
```

`--panel` background, thin `--rule` border, `.smaller`, `--pad` padding, clears
both columns. The `address` is not italic.

### Padded table

```
<table class="padding5">
```

The table style for reference material: `1px solid --border` on every cell, 5px
padding, header row in `--panel`. Every table in the manual is one of these; the
stylesheet applies it to all tables so Markdown gets it for free.

### Example

```
<pre>...</pre>
```

`--panel` background, `--pad` padding, horizontal scroll, no border. Inline
`code` gets the same background.

### Highlight and helpers

`.highlight` bold in `--heading`. `.small`, `.smaller`, `.center`, `.right`,
`.no-margin-top`. Nothing else.
