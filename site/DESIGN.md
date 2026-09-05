# fontina web design system

The shape of these pages is the GNU one, and deliberately so: a main column, a nav
column, a notice at the foot, and every page readable in a text browser. The setting
is not. Where the old stylesheet took gcc.gnu.org's serif, its filled boxes and its
ad-hoc ems, this one uses the sans the reader's own system already provides, two
scales instead of arbitrary numbers, and hairlines instead of fill.

`src/styles/site.css` implements exactly what is here, section for section. If you add
something, add it to both. `/style/` renders every component once, which is where a
change gets checked by eye.

Rules that hold on every page: light mode only; system fonts only; no web fonts; no
client-side script; no external requests; one stylesheet; every page must read in a
text browser. Colour carries hierarchy, never meaning on its own.

## Families

| Token | Stack |
|---|---|
| `--font-sans` | `ui-sans-serif, system-ui, -apple-system, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif` |
| `--font-mono` | `ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, "Liberation Mono", monospace` |
| `--font-emoji` | `"Apple Color Emoji", "Segoe UI Emoji", "Noto Color Emoji", "Twemoji Mozilla", "EmojiOne Color", "Android Emoji", sans-serif` |

Nothing is fetched. `ui-sans-serif` and `system-ui` resolve to the interface face the
machine already draws its own menus in, which is the face its owner reads fastest;
the rest of the stack is there for the machines that have neither.

`--font-emoji` exists for one character. Left to the body stack, a system sans will
render U+1F9C0 out of whatever fallback it reaches first, which on some machines is a
monochrome symbol face; asking for the colour emoji font by name is the difference
between a cheese and an outline of one.

## Type scale

Seven steps, ratio about 1.25, all in `rem` so they follow the reader's browser setting
rather than overriding it.

| Token | Value | At 16px | Used for |
|---|---|---|---|
| `--text-2xl` | `clamp(1.75rem, …, 2.25rem)` | 28 → 36px | `h1` |
| `--text-xl` | `clamp(1.5rem, …, 1.75rem)` | 24 → 28px | a section opener |
| `--text-lg` | `clamp(1.25rem, …, 1.375rem)` | 20 → 22px | `h2` |
| `--text-md` | `1.125rem` | 18px | `h3`, `.lead` |
| `--text-base` | `1rem` | 16px | body |
| `--text-sm` | `0.875rem` | 14px | nav, meta, tables, code |
| `--text-xs` | `0.75rem` | 12px | labels, badges |

The top three steps interpolate with the viewport rather than snapping at a
breakpoint: a 36px `h1` is right at the top of a desktop column and too loud on a
phone. The scale still names both endpoints, and the four steps below `--text-lg` do
not move — body text should be the size the reader asked for.

Leading is `--leading` 1.6 for prose, `--leading-snug` 1.45 for nav, dense lists and
code, `--leading-tight` 1.25 for headings. Sans needs more air than the serif did;
1.35 was right for Times and is too tight for this.

Weights are `--weight-normal` 400, `--weight-medium` 500 (buttons), `--weight-semi`
600 (headings, terms, the current nav item). Bold 700 is not used: at these sizes a
system sans at 600 is emphatic enough, and 700 shouts.

Headings are set in `--ink`, not a colour of their own, with a top margin larger than
the bottom so they attach to what follows, and negative tracking that grows with the
size. A heading directly under a heading closes up. `h1` is left-aligned; centring it
was the most dated thing on the old page. Body figures are tabular so dates and
versions line up in a column; code overrides that back to normal.

## Spacing scale

One 4px step, doubled and halved. Every margin and padding in the stylesheet is one of
these eight values, which is the whole reason the page looks set rather than emitted.

| Token | Value | | Token | Value |
|---|---|---|---|---|
| `--space-1` | 4px | | `--space-5` | 24px |
| `--space-2` | 8px | | `--space-6` | 32px |
| `--space-3` | 12px | | `--space-7` | 48px |
| `--space-4` | 16px | | `--space-8` | 64px |

## Colour

Three inks, three grounds, two lines, one accent.

| Token | Value | Use |
|---|---|---|
| `--ink` | `#14171c` | body text |
| `--ink-soft` | `#59616e` | secondary: dates, meta, the notice, `dd` |
| `--ink-faint` | `#6b7480` | tertiary: nav labels, disabled |
| `--bg` | `#ffffff` | page |
| `--bg-subtle` | `#f7f8fa` | panels |
| `--bg-inset` | `#f2f4f7` | code and things set into the page |
| `--line` | `#e5e8ec` | the default hairline |
| `--line-strong` | `#ccd2da` | a border that has to hold its own |
| `--accent` | `#1b58d0` | links, primary action, focus ring |
| `--accent-hover` | `#1443a5` | hover and active |
| `--accent-visited` | `#4a3f9e` | visited links |
| `--accent-subtle` | `#eff4fe` | tinted ground: notes, quiet hover, selection |
| `--accent-line` | `#c8d8f8` | the edge of a tinted thing |

Near-black on white rather than `#000` on `#fff`: pure black on pure white is harsher
than any printed page, and these are meant to be read for a long time. One accent does
four jobs, so there is never a question of which blue. The old darkorange hover is
gone; hover deepens the blue and adds the underline, which is the same information
without the costume change.

Radii are `--radius-sm` 3px, `--radius` 5px, `--radius-lg` 8px — small on purpose. The
thing being documented is a command line. There are no shadow tokens: this design draws
with hairlines, and a shadow would be the one soft edge on an otherwise sharp page.

## Layout

| Token | Value | Use |
|---|---|---|
| `--frame` | `70rem` | main column, gutter and nav column together |
| `--nav-w` | `13rem` | the nav column |
| `--gutter` | `--space-6` | between the two |
| `--measure` | `72ch` | prose line length inside the main column |

The frame is a flex row of `.main` and `.navcol`, centred. Prose inside `.main` is
capped at `--measure` so a line stays readable even when a table beside it is wide.
Navigation is three tiers, not one list doing three jobs.

1. **The masthead**, above the content on every page: the wordmark and the five
   destinations most people want. It is the fix for a real defect — with the index living
   only in a right-hand column, a reader met twenty-four links before the first sentence
   and, once the page stacked on a phone, met them *after* the last one, so the way to the
   manual sat at the bottom of the manual.
2. **The nav column**, only where there is something contextual to put in it. Today that
   is the manual's own chapter list; every other page is the full frame. It is sticky, so
   it stays in reach down a long chapter, and scrolls inside itself if it is taller than
   the window.
3. **The sitemap**, at the foot: every group, every link, in as many columns as fit.

The masthead wraps rather than collapsing into a menu. Five items do not need a
disclosure, and a disclosure without script means fighting the user agent stylesheet;
under `30rem` the wordmark takes its own line and the destinations sit beneath it.

Three things happen as the page narrows. Under `60rem` the columns stack, the nav takes
a top hairline and stops being sticky, and — because six groups in a single list is
three screens of scrolling on a phone — its groups flow into as many columns as fit.
Under `30rem` the page takes back the margins it was spending: smaller body padding, a
shorter gap above the frame, tighter heading rhythm. And where the pointer is coarse,
nav links and buttons grow their vertical padding, because a finger is not a mouse.

Nothing may scroll the page sideways. Tables and `pre` scroll inside themselves; prose
breaks a long word rather than widening the column, and code is exempt from that so a
path is never broken mid-token.

Links are not underlined at rest; the colour carries them, and a page dense with links
is not a page of stripes. Hover deepens the colour and underlines. Keyboard focus draws
a 2px `--accent` ring at 2px offset. Those two states are the accessibility budget of
this design and they are not optional. Anchored headings carry `scroll-margin-top` so a
deep link does not land with the heading against the top of the window.

## Components

Each is a class (or an element) in the stylesheet, with the markup it expects.
`/style/` shows every one.

### Button

```html
<p class="btn-row">
  <a class="btn btn--primary" href="...">Download fontina</a>
  <a class="btn" href="...">Read the manual</a>
  <a class="btn btn--quiet" href="...">Source on GitHub</a>
</p>
```

One button, three intents (`--primary`, default, `--quiet`), two sizes (default and
`--sm`). An `<a>` and a `<button>` are drawn identically, so a link that acts like a
button looks like one without pretending to be a form control. `--primary` marks the
one action a page is for and there is never more than one on a page; `--quiet` is for
an action that is available but not being suggested. `aria-disabled="true"` greys it
and removes pointer events. `.btn-row` lays them out with `--space-2` between and wraps
on a narrow screen.

### Badge

```html
<span class="badge">badge</span>
<span class="badge badge--accent">accent</span>
```

Labels a thing in place — a state, a licence, a version — at `--text-xs` inside a
hairline. Two variants, neutral and accent.

### Note

```html
<div class="note">
  <span class="label">Note</span>
  <p>...</p>
</div>
```

An aside that must not be missed: a 2px `--accent-line` bar, `--accent-subtle` ground,
and the optional `.label` above it. Takes any content; the last child loses its margin.

### Panel

```html
<div class="panel">...</div>
```

Groups something that is not prose — a summary, a set of figures. `--bg-subtle`, one
hairline, one radius. The last child loses its margin.

### Hero

```html
<header class="hero">
  <h1>fontina</h1>
  <p class="tagline">A font manager for the terminal.</p>
  <p class="lead">One paragraph of what it is.</p>
  <p class="btn-row">…</p>
</header>
```

The block a landing page opens with. Not a marketing hero: no ornament, no full-bleed
anything, no ornamental type. It exists so that the first screen answers *what is this*
before the page starts answering *what has happened lately*. The old front page led with
news and release status, which is a project's own view of itself rather than a reader's.

### Section

```html
<section class="section"><h2>…</h2>…</section>
```

A rule and a wider gap. A long page of undifferentiated blocks is what makes a page of
this era feel like an archive rather than a document, and this is the cheapest thing that
says "new subject". No card, no shadow, no background: one hairline.

### Grid and tile

```html
<div class="grid">
  <div class="tile"><h3><a href="…">Activates</a></h3><p>… <code>activate</code></p></div>
</div>
```

Scannable units — one claim each, each linking to the chapter that proves it, each naming
the command that does it. `auto-fit` with a `15rem` minimum, so it becomes one column on a
phone without a breakpoint of its own. This replaces a paragraph that carried fifteen
inline links, which nobody reads as a list because it is not one.

### Properties

```html
<dl class="props">
  <dt>It makes no network connections</dt>
  <dd>How you would check that.</dd>
</dl>
```

For claims that have to be checkable: the term is the property, the definition says how
you would verify it. Used for the free-software section, where a vague promise would be
worse than none — "respects your freedom" says nothing, "contains no networking code, and
a catalogue feature would have to live in a separate package to keep it that way" can be
confirmed by reading the source.

### On-page contents

```html
<nav class="toc" aria-label="On this page">
  <span class="label">On this page</span>
  <ul><li><a href="#slug">Section</a></li></ul>
</nav>
```

Built from the chapter's own depth-2 headings, which Astro hands back from `render()`, so
it cannot drift from the page. Two columns on a wide screen, one under `40rem`. It appears
only where there are more than two sections: the command reference runs to eleven, and
arriving at the top of that with no map is why a reader gives up on a manual and goes back
to `--help`.

### Chapter row

```html
<div class="chapter">
  <div class="num">5</div>
  <div class="body"><a href="…">Command reference</a><p>The sentence that says whether this is the chapter you want.</p></div>
</div>
```

The manual index, which was an `<ol>` of ten links. The number keeps the reading order the
manual is written in; the description is what lets someone skip to the chapter they need.

### Lockup

```html
<span class="lockup"><span class="mark" aria-hidden="true">🧀 </span>fontina</span>
```

The mark and the name, set once and used at two sizes — 18px in the masthead, 36px in
the hero. Everything is in `em`, so one rule serves both. fontina is a cheese; the mark
is the joke, and it is the only ornament on the site.

Three decisions worth writing down, because each is the opposite of the obvious one:

- **Centred, not baseline-aligned.** An emoji does not share a baseline with the text
  around it in any predictable way: it is drawn to fill its em box rather than to sit on
  a line.
- **Below `1em`.** Filling the em box is also why it reads a size larger than the
  capitals beside it at nominal parity. It is set at `0.92em`, with `line-height: 1` so
  it cannot stretch the masthead, and a `0.02em` nudge that is optical rather than
  metric.
- **`aria-hidden`, and that is the fallback.** The word carries the name. A machine with
  no emoji font shows a tofu box next to a wordmark that still reads, and a screen reader
  says "fontina" rather than "cheese wedge fontina". There is no way to detect a missing
  emoji font without script, so the honest design is one where its absence costs nothing.
  The trailing space inside the mark collapses under CSS and separates the two in a text
  browser.

The favicon is drawn as SVG rather than set as this character, for the same reason in a
harder form: nothing about a favicon can rely on a font being present at all.

### Masthead

```html
<header class="masthead">
  <div class="inner">
    <p class="wordmark"><a href="/">fontina</a></p>
    <nav aria-label="Main"><a href="…" aria-current="page">Manual</a>…</nav>
  </div>
</header>
```

A masthead in the sense a newspaper has one, not a chrome bar: a rule underneath, no
shadow, nothing sticky, nothing that follows you down the page. The current destination
takes `aria-current="page"` and darkens; everything else sits in `--ink-soft` so the bar
does not compete with the first heading under it.

### Property strip

```html
<ul class="strip">
  <li><span class="badge">No network</span></li>
  <li><span class="badge badge--accent"><a href="/license/">GPL-3.0-or-later</a></span></li>
</ul>
```

What the program refuses to do, in the first screen, as badges. The claim gets made where
a reader is deciding whether to care and argued properly further down in **Properties**;
a licence named in a badge and never explained is a sticker.

### Sitemap

```html
<nav class="sitemap" aria-label="Site"><div class="inner">…nav items…</div></nav>
```

The complete index, at the foot, in as many columns as fit. This is where the twenty-four
links went when the masthead took the five that matter and the nav column became
contextual.

It reuses the nav item's markup but not its rail. The nav column's 2px left border says
*you are here in this list*; a footer index is not a list you are anywhere in, so in the
sitemap the border comes off and the indent with it. A component carried into a place
where its affordance does not apply is worse than a new one, because it makes a promise
the page cannot keep.

### Colophon

```html
<div class="colophon">
  <p><strong>fontina is free software.</strong> …GPL for the program, GFDL for the prose…</p>
</div>
```

Between the index and the small print, because it is neither. What a free software
project's footer owes a reader is which licence covers the program and which covers the
words, named rather than linked past — followed by the three things it refuses to do, so
the claim on the front page is repeated where someone who scrolled to the bottom looking
for it will find it.

### Frame

```html
<div class="frame">
  <main class="main">...</main>
  <nav class="navcol">...</nav>
</div>
<div class="notice">...</div>
```

### Nav item

```html
<div class="navitem">
  <div class="navhead">Documentation</div>
  <div class="navbody">
    <a href="...">Manual</a>
    <a href="..." class="sub">Command reference</a>
    <a href="..." aria-current="page">The current page</a>
  </div>
</div>
```

A tracked-out uppercase label, then a rail of links. This replaces the blue title bar
over a filled box: the same three levels of hierarchy — group, member, current — drawn
with weight, space and one 2px edge instead of two fills and two borders.

Every item carries a transparent 2px left border, so hover can colour it `--line-strong`
and the current page `--accent` without the text shifting by a pixel. The current page
stays a real link marked `aria-current="page"`, which the keyboard can still reach and a
screen reader announces; it is not `<strong>` text. Links are block-level, so the markup
needs no `<br>`; a sub-item takes `.sub` and is indented rather than prefixed with a
middot. This is the whole site navigation; there is no top bar.

### Columns

```html
<div class="columns">
  <section class="col">...</section>
  <section class="col">...</section>
</div>
```

Two equal columns for the front page, separated by `--space-7` of gutter. The vertical
rule between them is gone: the gutter is enough separation, and the rule was the last
piece of table-era furniture on the page.

### News list

```html
<dl class="news">
  <dt><a href="...">Title</a> <span class="date">2026-09-03</span></dt>
  <dd>One line of detail, or nothing.</dd>
</dl>
```

`dt` at `--weight-medium`; the date in `--ink-faint` at `--text-sm`, tabular, with the
square brackets dropped — the colour and size already say it is a date. An empty `dd`
collapses.

### Status list

```html
<dl class="status">
  <dt><span class="version"><a href="...">fontina 0.1.1</a></span> (<a href="...">changes</a>)</dt>
  <dd>Released 2026-09-04. <div class="regress">...</div></dd>
</dl>
```

`.version` semibold; `.regress` at `--text-sm` in `--ink-soft`.

### Node nav

```html
<p class="node-nav">Next: <a>...</a> · Previous: <a>...</a> · Up: <a>Manual</a></p>
```

Texinfo-style navigation above and below a manual chapter. A hairline under it at the
top of the page, a hairline over it at the foot, and no box.

### Notice

```html
<div class="notice">
  <address>Where to ask for help.</address>
  <p>Copyright and the verbatim-copying notice.</p>
  <p>Who maintains the pages and when they were last modified.</p>
</div>
```

The GNU copyright box, unboxed: a rule across the frame and then the small print in
`--ink-soft`. The `address` is not italic.

### Table

```html
<table>...</table>
```

Rules between rows, not around cells — the data is the grid, and a border on every cell
was drawing the same information twice. The header row is semibold `--ink-soft` over a
`--line-strong` rule; the last row drops its rule. Code in a cell does not wrap.

Every table carries its own horizontal scroll, as `display: block` with
`width: max-content` and `max-width: 100%`: a narrow table stays its own size, a wide
one scrolls inside the column. That is not decoration. Markdown emits a bare `<table>`
and cannot be handed a wrapper, so the nine tables in the manual previously had nothing
stopping them pushing the whole page sideways on a phone. There is no `.wide` helper
any more; it existed only to do this by hand, and by hand it was never applied to the
Markdown that needed it most.

### Code

```html
<pre>...</pre>
```

`--bg-inset` inside a hairline at `--radius`, `--text-sm`, `--leading-snug`, tab width
4, its own horizontal scroll. Inline `code` takes the same ground and a small radius,
except inside a heading, a term or a link, where the tint is only noise. `kbd` is drawn
as a key.

The manual is not syntax-coloured, and that is a finding rather than a preference:
all 110 of its fenced blocks are terminal transcripts, none carries a language, and
Shiki's `console` grammar does not separate the prompt from the output under this
theme. It therefore tokenises nothing and contributes only an inline background,
which the stylesheet overrides back to the palette.

`pre.term` does the one distinction a transcript actually has: the command in
`--accent`, marked up as `<span class="cmd">`, against the output in `--ink-soft`.
It is applied by hand where the markup is ours. Applying it to the manual's 110
blocks would need either `@astrojs/markdown-remark` as a dependency or a build-time
pass over the emitted HTML; neither is done here.

### Helpers

`.lead` (an opening paragraph at `--text-md` in `--ink-soft`), `.label` (the uppercase
micro label), `.small`, `.smaller`, `.soft`, `.faint`, `.highlight`, and
`.skip`, the skip link, clipped until it is tabbed to. Nothing else.

## Print

The nav column, node navs and button rows are dropped, the frame goes full width,
links keep their text colour, panels and code lose their ground, and external links
print their URL after the text.
