---
title: Platform notes
description: "font directories, how activation works, and caveats on Linux, macOS and Windows."
order: 10
---

Core and command line are platform-agnostic. Everything that differs by operating
system is in one crate, `fontina-platform`, behind one trait with three
implementations, so `activate`, `install` and their inverses mean the same thing
everywhere and differ only in mechanism. `fontina dirs` prints what the running
binary believes the font directories are.

The rules that hold on every platform: system font directories are never modified;
nothing requires elevation; everything is per-user; and every mechanism is the
operating system's own, so if you delete fontina, the links, copies and
registrations it made are ordinary things you can undo by hand.

## Linux and the BSDs

Font directories scanned with `--system`: `/usr/share/fonts`,
`/usr/local/share/fonts`, `$XDG_DATA_HOME/fonts` (normally `~/.local/share/fonts`)
and the legacy `~/.fonts`. The per-user directory is `$XDG_DATA_HOME/fonts`.

fontconfig reads `$XDG_DATA_HOME/fonts` recursively, so both operations are
symlinks into it:

- `install` links the file into `$XDG_DATA_HOME/fonts/fontina/`;
- `activate` links it into `$XDG_DATA_HOME/fonts/fontina-active/`, and writes a
  fragment at `~/.config/fontconfig/conf.d/50-fontina.conf` declaring that
  directory, so activation works even for users whose `fonts.conf` does not include
  the XDG directory.

`fc-cache` is run on the directory if it is installed, so applications that do not
watch the directory pick the change up promptly; its absence is not an error.
Session and user activations look the same to fontconfig; the index records which
is which, and `restore` re-creates session links after a reboot.

Linux is the reference platform. The terminals people actually use (kitty, foot,
GNOME Console, Konsole, WezTerm) are tested first for `preview` and `ui`, and
`.deb` and `.rpm` packages are built by the release workflow.

## macOS

Font directories: `~/Library/Fonts` (per-user), `/Library/Fonts`, `/System/Library/Fonts`
and `/Network/Library/Fonts` if present.

`activate` registers the file in place through Core Text's font manager, with the
`user` scope, which persists across logins without copying, or the `session` scope
with `--session`. `install` copies into `~/Library/Fonts`, which the font daemon
watches. Registering in place means the file must stay where it is; the index
stores the canonical path and the content hash, so a moved file shows up as a
failed activation on rescan rather than a silent gap.

## Windows

Font directories: `%LOCALAPPDATA%\Microsoft\Windows\Fonts` (per-user, Windows 10
1809 and later) and `%WINDIR%\Fonts`.

`activate` calls `AddFontResourceEx` to make the file visible to every process for
the session, and for `user` scope also writes a value under
`HKCU\Software\Microsoft\Windows NT\CurrentVersion\Fonts`, which Windows reads at
logon for per-user fonts from any path. `install` copies into the per-user font
directory and writes the same value. `WM_FONTCHANGE` is broadcast after every
change so running applications refresh.

Caveat: some legacy applications only see machine-wide fonts. fontina documents
this rather than working around it, since the workaround is elevation.

## Previews per terminal

`preview` and `ui` pick the best image protocol the terminal supports: kitty
graphics (kitty, Ghostty, WezTerm, Konsole), iTerm2 inline images (iTerm2, WezTerm,
mintty), sixel (foot, xterm, mlterm, Windows Terminal), and half-block text
everywhere else. Detection reads `TERM` and `TERM_PROGRAM` and sends a kitty query;
`-p` overrides it.

## Paths in the index

The index stores absolute, canonical paths. On Windows they use the drive-letter
form. On all platforms, `--under PREFIX` matches on the stored string, so use the
same form the index shows in `list`.
