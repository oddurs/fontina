# unifont web design system

Derived from the gcc.gnu.org stylesheet, kept where it is good and corrected where it
is only old. The stylesheet `src/styles/site.css` implements exactly what is here,
section for section; if you add something, add it to both. `/style/` renders every
component once.

Rules that hold on every page: light mode only; the browser's default serif; no web
fonts; no client-side script; no external requests; one stylesheet; every page must
read in a text browser. Colour carries hierarchy, never meaning on its own.

## Tokens

| Token | Value | Use |
|---|---|---|
| `--bg` | `#ffffff` | page background |
| `--fg` | `#000000` | body text |
| `--fg-soft` | `#333333` | secondary text: dates, regress lines, the copyright box, quotations |
| `--heading` | `darkslategray` (`#2f4f4f`) | h1, h2, h3, `.highlight`, news titles |
| `--link` | `#0066bb` | links |
| `--link-visited` | `#003399` | visited links |
| `--link-hover` | `darkorange` | hovered links |
| `--accent` | `#0066dd` | nav item header background |
| `--accent-fg` | `#f2f2f9` | nav item header text |
| `--rule` | `#3366cc` | 1px borders that mean something: nav header, status column, copyright box, focus ring, quotation bar |
| `--panel` | `#f2f2f9` | panel backgrounds: nav body, copyright box, examples, node nav, table headers |
| `--panel-edge` | `#d9d9e6` | 1px edge on panels, so they hold on a white page without a heavy border |
| `--border` | `gray` | table cell borders |
| `--measure` | `66em` | width of the frame; the main column comes out near 50em, about 90 characters |
| `--nav-width` | `11em` | the nav column |
| `--gap` | `32px` | gap between the main column and the nav column |
| `--pad` | `4px` | inner padding of small boxes |
| `--pad-2` | `8px` | inner padding of nav bodies, examples and table cells |
| `--leading` | `1.45` | body line height |

The colours are gcc.gnu.org's, unchanged. What is new is the measure, the leading,
the soft foreground and the panel edge: the four things that separate a page that
was set from a page that was merely emitted.

## Type

`font-family: serif`, base size 100%, `line-height` `--leading`. Two smaller steps:
`.small` 90% (dates) and `.smaller` 80% (nav bodies, copyright, regress lines).
Headings are bold in `--heading` with `line-height` 1.2 and a top margin larger than
the bottom one so they attach to what follows; `h1` is centred at 1.9em. A heading
directly under a heading closes up. Dates are set with tabular figures.

Links are not underlined at rest; the colour carries them. On hover and active they
turn `--link-hover` and underline, and keyboard focus draws a 2px `--rule` ring.
Those two states are the accessibility budget of this design, and they are not
optional.

Tables are set at 95% with `4px 8px` cells; code in a cell does not wrap. Examples
are set at 88% with `line-height` 1.4 and a `--panel-edge` border. Inline `code`
gets the panel background except inside a heading, a definition term or a link,
where it would only add noise.

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

Two columns: main content and a nav column on the right, separated by `--gap`, at
most `--measure` wide, centred. The nav column is sticky, so on a long manual
chapter it stays in reach; it scrolls inside itself if it is taller than the window.
Under 50em the nav column drops below the content and stops being sticky. The
copyright box spans both columns.

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

A bold header bar in `--accent` on `--accent-fg` with a 1px `--rule` border, and a
body in `--panel` at `.smaller` with a `--panel-edge` border on three sides so the
box reads as one piece. One link per line at `line-height` 1.6. A `&middot;&nbsp;`
prefix marks a sub-item. This is the whole site navigation; there is no top bar.

### Columns

```
<div class="columns">
  <section class="col news">...</section>
  <section class="col status">...</section>
</div>
```

Two equal columns for the front page, 16px of inner padding each side of a 1px
`--rule` between them. Each column starts with an `h2` at 1.2em with no top margin;
later `h2`s in a column get 1.4em above.

### News list

```
<dl class="news">
  <dt><a href="...">Title</a> <span class="date">[2026-09-03]</span></dt>
  <dd>One line of detail, or nothing.</dd>
</dl>
```

`dt` bold in `--heading`; the date in `.small` `--fg-soft` tabular figures inside
square brackets; `dd` indented 3ex with tight vertical margins.

### Status list

```
<dl class="status">
  <dt><span class="version"><a href="...">unifont 0.0.1</a></span> (<a href="...">changes</a>)</dt>
  <dd>Status: <a href="...">2026-09-03</a> (pre-release). <div class="regress">...</div></dd>
</dl>
```

`.version` bold; `.regress` at `.smaller` in `--fg-soft`.

### Node nav

```
<p class="node-nav">Next: <a>...</a>, Previous: <a>...</a>, Up: <a>Manual</a></p>
```

Texinfo-style navigation line above and below manual chapters, in a `--panel` box
with a `--panel-edge` border at `.smaller`. The closing one gets 2em of air above it.

### Copyright box

```
<div class="copyright">
  <address>Where to ask for help.</address>
  <p>Copyright and the verbatim-copying notice.</p>
  <p>Who maintains the pages and when they were last modified.</p>
</div>
```

`--panel` background, 1px `--rule` border, `.smaller` in `--fg-soft`, `6px 8px`
padding, clears both columns, 2.5em above. The `address` is not italic and its
paragraphs are spaced like the rest.

### Padded table

```
<table>
```

The table style for reference material: `1px solid --border` on every cell, `4px
8px` padding, header row in `--panel`, 95% size. Every table in the manual is one
of these; the stylesheet applies it to all tables so Markdown gets it for free.

### Example

```
<pre>...</pre>
```

`--panel` background, `--panel-edge` border, `6px 8px` padding, horizontal scroll,
88% size, tab width 4. Inline `code` gets the same background.

### Highlight and helpers

`.highlight` bold in `--heading`. `.small`, `.smaller`, `.center`, `.right`,
`.no-margin-top`. Nothing else.

## Print

The nav column and node navs are dropped, the frame goes full width, links keep
their text colour, and external links print their URL after the text.
