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

opens a full-screen browser over the index: facets on the left, families or faces
in the middle, details and a preview on the right. It uses the terminal's own
sixteen colours so it looks native in any theme, and truecolor only for the
preview. The mouse works, but the keyboard is the design.

Every action in the browser is one the command line can do, and the status line
shows the equivalent command. The keys:

| Key | Action |
|---|---|
| `/` | search; type, then Enter |
| Tab | move between the facet column and the list |
| Enter, Space, Right, `l` | open a family, or a face's details |
| Backspace, Left, `h` | back |
| `j` `k`, arrows, PageUp, PageDown, `g`, `G` | move |
| `t` | tag the current face |
| `c` | add it to a collection |
| `x` | clear filters |
| `e` | change the preview text |
| `+` `-` | preview size |
| `a` / `A` | activate for the user / for the session |
| `d` | deactivate |
| `i` / `u` | install / uninstall |
| `R` | rescan the sources |
| `?` | help |
| `q`, Esc, Ctrl-C | quit |

A conflict on activation is shown in the status line with the `--replace` hint,
exactly as on the command line.
