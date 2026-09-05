---
title: Watching, previews and the browser
description: "`watch` keeps the index current; `preview` draws real shaped glyphs in the terminal; `ui` is the keyboard-first browser."
order: 4
---

## Watching

A source registered with `source add` is *watched* by default. `fontina watch`
follows every watched source and keeps the index current until interrupted:

```
$ fontina source add ~/Fonts
$ fontina watch
```

Changes arrive in batches: the watcher waits for a quiet period (`--debounce-ms`,
default 500) and then rescans what changed, so unpacking a hundred files produces one
batch, not a hundred. One line is printed per batch; with `--json`, one JSON object
per line, which makes it a stream another program can follow.

Extra directories can be followed for one run without registering them:

```
$ fontina watch ~/Downloads/fonts-to-sort
```

`watch` is a foreground process on purpose. Run it under whatever supervises long
processes on your system. A systemd user unit on Linux:

```
# ~/.config/systemd/user/fontina-watch.service
[Unit]
Description=Keep the fontina index current

[Service]
ExecStart=%h/.local/bin/fontina watch
Restart=on-failure

[Install]
WantedBy=default.target
```

Turn watching off for a source with `source watch PATH --off`; it is still scanned
by `scan` and `source add`, just not followed.

## Previews

`fontina preview` shows a face as real, shaped glyphs, in the terminal:

```
$ fontina preview 42
$ fontina preview 42 -t "Sphinx of black quartz, judge my vow" -s 64
$ fontina preview family:Amiri -t "بسم الله"
$ fontina preview 42 -a wght=700 -a wdth=85 -f smcp -f liga=0
$ fontina preview 42 -o specimen.png
```

The text is shaped by HarfBuzz's Rust port, so Arabic joins, Indic reorders, emoji
sequences compose, and kerning and ligatures apply. Outlines come from the same
parser as everything else and are rasterised at the requested size and axis
position. `-a` sets a variable axis (repeatable); `-f` turns an OpenType feature on,
or off with `=0`. The default text is a pangram, or the face's own sample text when
it carries one.

The bitmap is drawn with the best protocol the terminal supports, detected
automatically:

| Protocol | Terminals |
|---|---|
| kitty graphics | kitty, Ghostty, WezTerm, Konsole |
| iTerm2 inline images | iTerm2, WezTerm, mintty |
| sixel | foot, xterm, mlterm, Windows Terminal |
| half-block text | everywhere else, including CI logs and `less -R` |

`-p` forces one (`kitty`, `iterm`, `sixel`, `blocks`); `-p png` with `-o` writes a
PNG instead. `--fg` and `--bg` set the ink and, for sixel and blocks, the
background; `--max-width` clips.

## The browser

```
$ fontina ui
```

opens the index with a keyboard on it. Three panes: every facet of the library down
the left, families or faces in the middle, the face itself on the right. It uses the
terminal's own sixteen colours, so it looks like your terminal rather than like a
website, and truecolor only where a preview needs it. The mouse works. The keyboard
is the design.

<!--frame:the_browser_opens_on_the_family_list "families"-->

The left column is the library counted rather than searched: how many faces are Light,
how many are condensed, which scripts they cover, which vendors made them, what
licences they carry. Selecting one filters everything; `x` clears them all. Nothing
here is a saved list you have to build first.

### Opening a family

Enter opens a family into its faces, and Backspace closes it again.

<!--frame:opening_a_family_lists_its_faces_and_says_so_in_the_command "a family, opened"-->

Look at the bottom row. It says `$ fontina list --family Amiri`, which is the command
that would produce exactly this view. That row is the whole relationship between the
two halves of the program: the browser is a way of building a command line by looking
at things, and everything it can do, the command line can do.

### The glyph map

`m` gives the face its whole screen and lays out every character it has, block by
Unicode block, with how many of each block are covered.

<!--frame:the_glyph_map_lists_the_blocks_and_lays_out_the_characters "m — the glyph map"-->

`h` and `l` pick a block, `j` and `k` scroll it, and `/` finds either a codepoint —
`U+0041`, `0x41`, `41` — or a block by name. It is a mode rather than a pane because
reading coverage needs the width: 1,699 codepoints in twenty blocks does not fit
beside anything else.

### The controls

Tab moves to the controls, where a variable font stops being a list of instances and
becomes a family you can move through.

<!--frame:the_controls_pane_offers_the_axes_of_a_variable_face "⇥ — axes and features"-->

`h` and `l` move an axis and `H` and `L` move it by ten; `n` and `p` step through the
named instances the designer drew; Space toggles an OpenType feature; `0` puts
everything back. The preview above redraws as you move, so what you are looking at is
the font at that position rather than an interpolation of a picture.

### Waterfalls, comparisons, specimens

`w` sets the face down the size ladder, `C` compares every face the selection stands
for, and `+` and `-` resize a comparison. `e` changes the sample text everywhere, so
you can put your own words in.

For what a terminal cannot show honestly — colour fonts, fine hinting, the difference
between two weights at 11px — `s` writes a [self-contained HTML specimen](../specimen/)
for the selection and opens it in your browser. The terminal is where you decide what to
look at; the specimen is where you look at it.

### The keys

`?` puts this list over whatever you are looking at.

<!--frame:the_help_overlay_sits_over_the_browser "? — the keys"-->

| Key | Action |
|---|---|
| `j` `k`, arrows, PageUp, PageDown, `g`, `G` | move |
| Tab | cycle the facets, the list and the controls |
| `/` | search; type, then Enter. Esc clears |
| Enter, Space | open a family, or toggle a facet |
| Backspace, Esc | back |
| `x` | clear every filter |
| `t` / `c` | tag the selection / add it to a collection |
| `a` / `A` | activate for the user / until logout |
| `i` / `u` | install a copy / uninstall it |
| `d` | deactivate |
| `e`, `+`, `-` | sample text, preview size |
| `h` `l`, `H` `L` | move an axis, by one or by ten |
| `n` / `p` | step through named instances |
| `0` | reset the axes and features |
| `m` | the glyph map |
| `w` / `C` | waterfall / compare |
| `s` | write an HTML specimen and open it |
| `R` | rescan every source (`fontina scan --prune`) |
| `?` | this list |
| `q`, Ctrl-C | quit |

A conflict on activation is shown in the status line with the `--replace` hint,
exactly as on the command line.

### One program

The status line is not a log. It is the command for what is on screen, and it changes
as you do:

<!--frame:the_status_line_says_what_the_screen_is "the status line, four moments"-->

A filter is a flag. A search is a prompt. An action says what it did, until the next
reload. So the way to learn the command line is to use the browser, and the way to
script what you just did by hand is to copy the row.

Every frame on this page is the program's own output, read from the snapshot files a
test asserts on. They cannot go stale without a test going red first.
