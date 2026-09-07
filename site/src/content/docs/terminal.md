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

### Many at once

Every action starts out meaning the row under the cursor. Space marks a row instead,
`v` starts a range and `v` again ends it, and `*` marks everything the current filter
matches — so tagging a foundry's worth of faces is `*` and then `t`, rather than two
hundred keystrokes on the same key.

Once anything is marked, every action below applies to the marked faces: activate,
deactivate, install, uninstall, tag, collection. The pane title says how many are
marked so it is readable at the moment you reach for the key, a filter that stops
matching a marked face drops it and says so, and an action that fails on some of them
reports which and leaves the rest applied. Esc clears the marks.

A family row stands for its faces, so marking a family marks all of them — the mark
survives opening that family and looking at the faces one at a time.

### Taking it back

`U` undoes the last thing that changed the index and `Ctrl-R` does it again. A whole
selection is one undo: marking two hundred faces and pressing `a` is one entry, so one
`U` puts all two hundred back. The status line names what came back rather than saying
"undone", because pressing `U` twice should tell you which of two changes you just
reversed.

What comes back is what was *there*, which is not always the opposite of what you
asked for. Tagging a hundred faces of which forty already carried the tag records the
sixty that gained it, so undo takes the tag off those sixty and leaves the forty
carrying what they came with. An activation records the state each face was in, one by
one, so a selection that held three states goes back to three states.

What cannot be put back exactly is not offered. A rescan changes what the index knows
about the disk, and there is no earlier state to restore, so it records nothing and `U`
says there is nothing to undo. The history is in memory, for the session: one that
outlived the process would be a claim about a filesystem several other programs can
also write to.

### Every command, from inside the browser

`:` lists every command `fontina --help` lists — the nested ones too, `tag add` and
`collection export` and the rest — with the same one-line description, filtered as you
type. Above the list is the command line it would run, carrying the filter or the
selection you already have, so the palette teaches the command line rather than
replacing it.

The names come out of the program's own argument parser rather than a list somebody
maintains, so a new subcommand appears here the day it is added. What is written down
is the other half — whether the browser can *do* each one — and a test fails the build
when a command exists that nobody has decided about. That is what stops the two
surfaces drifting.

Enter runs the ones the browser implements, by pressing the browser's own key, so
there is one implementation of activating a font and not two. The ones that print get
written into the status line instead, ready to paste into another window: the browser
is using the screen they would print to. Anything that writes to the disk asks first,
and anything but `y` is a no.

### Choosing between two typefaces

Looking at two faces together is the whole of the decision, and a browser that shows
one at a time makes you do it from memory, which is where comparisons go wrong.

`.` pins the face under the cursor, up to four; `.` again unpins it. Pinned rows are
numbered in the list, in the order you pinned them, because that is the order the
comparison puts them in. `C` then sets them one above another at a single size, in a
single sample text, with a single set of controls — the ones in front of you when you
press it — so what differs between the rows is the design rather than the settings.

Pins survive filtering and searching, and that is the point of them rather than an
accident: marks are pruned when the filter stops matching, because a mark says what
the next action will touch and acting on something invisible is a bad idea. A pin says
"I am deciding between these", and searching for the next one is exactly how you find
it. Losing the first pin the moment you typed would make the feature useless at the
one moment it is needed.

With nothing pinned, `C` still compares whatever the listing holds, the way it always
did.

### The keys

`?` puts this list over whatever you are looking at.

<!--frame:the_help_overlay_sits_over_the_browser "? — the keys"-->

| Key | Action |
|---|---|
| `j` `k`, arrows, PageUp, PageDown, `g`, `G` | move |
| Tab | cycle the panes this width has |
| `/` | search; type, then Enter. Esc clears |
| Enter | open a family, or toggle a facet |
| Space | mark the row under the cursor |
| `v` | start a range; `v` again ends it |
| `*` | mark everything the filter matches, again to unmark |
| Backspace, Esc | back, or clear the marks |
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
| `.` | pin a face, up to four; again to unpin |
| `w` / `C` | waterfall / compare the pins, or the listing |
| `s` | write an HTML specimen and open it |
| `U` / Ctrl-R | undo the last change to the index / do it again |
| `:` | every command, filtered as you type |
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
