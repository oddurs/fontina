---
title: The font manager, in the terminal
date: 2026-09-04
---

Milestone M1 is on `main`: fontina now manages fonts, not just describes them.
`activate` and `install` make a font visible to every application, per user, in
place or by copying, on Linux, macOS and Windows, with conflict detection and
`restore` for after a reboot. `watch` follows your source directories and keeps the
index current. `preview` draws real, shaped glyphs in the terminal over kitty
graphics, iTerm2 images, sixel or half-block text. `fontina ui` is a keyboard-first
browser over all of it. Release archives will carry shell completions and man
pages, and Linux gets `.deb` and `.rpm` packages.

The manual has two new chapters, [Activation and installation](../../docs/activation/)
and [Watching, previews and the browser](../../docs/terminal/), and the
[command reference](../../docs/cli/) covers every new command.
