---
title: Watching, previews and the browser
description: "`watch` keeps the index current; `preview` draws real shaped glyphs in the terminal; `ui` is the keyboard-first browser over what is in your fonts."
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

opens the index with a keyboard on it. A filter line across the top, families or faces
on the left, the face itself on the right. It uses the terminal's own sixteen colours,
so it looks like your terminal rather than like a website. The mouse works. The
keyboard is the design.

It shows what is *in* a font rather than a picture of one. A terminal cell is about one
pixel wide and two tall, so type drawn with block characters arrives through a filter
that changes its weight, loses its spacing and destroys its detail — a judgement made
on something that is not the font. `s` writes a real specimen and opens it in a
browser; `fontina preview` draws a true image where the terminal has a protocol for
one. The browser is where you decide what to look at.

<!--frame:the_browser_opens_on_the_family_list "families"-->

The top line is the filter, and it is there whether or not anything is filtered. With
nothing on it says so and says which key would change that. With something on it names
every filter in your own words, says what they did to the count — `341 → 12 families`
— and names the key that clears them.

### Narrow by

`f` opens the library counted rather than searched.

<!--frame:narrow_by_opens_over_the_list_and_says_how_to_leave "f — narrow by"-->

Script first, then language, foundry, your own tags and collections, then what a face
can do, then weight, width and style, licence, source and format. That is the order a
person walks into a font library; nobody has ever opened a font manager thinking "show
me the 500 Mediums". Each section shows its top three and ends with `+N more`, which
Enter opens. Enter on a value filters everything; `x` clears them all; Esc closes the
panel. Nothing here is a saved list you have to build first.

Every count is a pair: families, and faces in brackets. The list beside the panel is a
list of families, so the number that leads is the one that says what pressing the row
will do to it. And a value you have selected is drawn at every count, including zero —
narrowing to nothing must not take away the row that undoes it.

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
everything back. The status line carries the command that draws what you have set —
`fontina preview 12 --axes wght=600,opsz=14` — so a position you found by feel is a
position you can hand to something that renders.

### The measurements

Under the controls are the numbers, which is what a terminal is good for: units per em,
ascender, descender, line gap, cap height, x-height and the x-height ratio, then
coverage per script with the count beside a bar.

Two faces compared this way are actually comparable. `x/em 0.43` against `x/em 0.52`
says which will look larger at the same size, and no rendering at terminal resolution
would have told you that.

### Looking at the type

`s` writes a [self-contained HTML specimen](../specimen/) for the selection and opens
it in your browser: a waterfall, the script samples, axis sliders, feature toggles and
a glyph map, with real antialiasing and real spacing. On the command line,
[`fontina preview`](#previews) draws a true image over kitty graphics, iTerm2 or sixel.

Neither is the browser's job. It finds and organises; those show.

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

### Filters the facets cannot express

The facet pane shows what the library contains and lets one value be chosen from each
heading. A weight *range*, two scripts at once, a coverage threshold, "not variable" —
none of those fit that shape, and the only way to find out how many faces a filter
would leave was to run it in another window.

`F` opens a filter bar that takes the flags `fontina list` takes, and takes them by
handing them to `fontina list`'s own parser. Nothing lists the fields twice: a flag
added to the command line is a flag the browser understands the same day.

```
--weight 300-500 --script Cyrl --script Grek --script-min 200 --variable=false
```

It is applied as you type, so the panes behind the prompt are already showing the
answer and the count sits beside the line. A half-typed flag is not a filter, so the
panes stay on the last line that parsed and the prompt says which word is wrong, in the
command line's own words. Enter keeps it, Esc puts back what was there, and `Ctrl-S`
saves everything it matched as a collection.

The bar opens pre-filled with the flags for the screen you are already on, so it starts
as an editable copy of what you can see rather than an empty box. Toggling a facet
afterwards hands control back to the facets and says so: a typed filter and the facet
pane are two ways of saying the same thing, and there is no sensible way to add one
facet to `--variable=false`.

### The keys

`?` puts this list over whatever you are looking at.

<!--frame:the_help_overlay_sits_over_the_browser "? — the keys"-->

| Key | Action |
|---|---|
| `j` `k`, arrows, PageUp, PageDown, `g`, `G` | move |
| Tab | cycle the panes this width has |
| `/` | search; type, then Enter. Esc clears |
| `f` | Narrow by: the facets, as a panel. Esc closes it |
| `F` | the filter bar: `fontina list` flags, applied as you type |
| Enter | open a family, or pick a value in Narrow by |
| Space | mark the row under the cursor |
| `v` | start a range; `v` again ends it |
| `*` | mark everything the filter matches, again to unmark |
| Backspace, Esc | back, or clear the marks |
| `x` | clear every filter |
| `t` / `c` | tag the selection / add it to a collection |
| `a` / `A` | activate for the user / until logout |
| `i` / `u` | install a copy / uninstall it |
| `d` | deactivate |
| `h` `l`, `H` `L` | move an axis, by one or by ten |
| `n` / `p` | step through named instances |
| `0` | reset the axes and features |
| `m` | the glyph map |
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
