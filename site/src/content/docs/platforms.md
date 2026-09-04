---
title: Platform notes
description: "font directories, activation plans and caveats on Linux, macOS and Windows."
order: 8
---

Core and command line are platform-agnostic. Everything that differs by operating
system is in one crate, `fontina-platform`, behind one trait, so that the behaviour
described here is the same trait with three implementations. `fontina dirs` prints
what the running binary believes the font directories are.

The rules that hold on every platform: system font directories are never modified;
nothing requires elevation; everything is per-user.

## Linux and the BSDs

Font directories scanned with `--system`: `/usr/share/fonts`,
`/usr/local/share/fonts`, `$XDG_DATA_HOME/fonts` (normally `~/.local/share/fonts`)
and the legacy `~/.fonts`. The per-user directory is `$XDG_DATA_HOME/fonts`.

Planned activation: a persistent install is a symlink into `$XDG_DATA_HOME/fonts`.
A session activation is a symlink into `$XDG_DATA_HOME/fonts/fontina-active`,
declared to fontconfig by a fragment in `~/.config/fontconfig/conf.d/50-fontina.conf`,
and removed at logout. Deactivation removes the link. No fontconfig cache is
rebuilt by hand; fontconfig notices the directory change itself.

Change events will come from inotify on the font directories. An optional XDG
autostart entry, off by default, will restore session activations at login.

Linux is the primary target. The desktop application will be packaged for Flathub
first and pin its WebKitGTK runtime there.

## macOS

Font directories: `~/Library/Fonts` (per-user), `/Library/Fonts`, `/System/Library/Fonts`
and `/Network/Library/Fonts` if present.

Planned activation through Core Text: `CTFontManagerRegisterFontURLs` with the
`user` scope for a persistent install without copying, and the `session` scope for
temporary activation. Enumeration and change events from
`CTFontManagerCopyAvailableFontURLs` and the registered-fonts-changed notification.
Registering in place means the font file must stay where it is; the index stores the
canonical path and the BLAKE3 hash, detects a move on rescan, and will offer a repair.

## Windows

Font directories: `%LOCALAPPDATA%\Microsoft\Windows\Fonts` (per-user, Windows 10
1809 and later) and `%WINDIR%\Fonts`.

Planned activation: a persistent install copies to the per-user font directory,
writes the `HKCU` fonts key and broadcasts `WM_FONTCHANGE`. A session activation
uses `AddFontResourceExW` without the private flag, re-applied at login by the
optional agent. Enumeration through DirectWrite's font set.

Caveat: some legacy applications only see machine-wide fonts. fontina will document
this rather than work around it, since the workaround is elevation.

## Paths in the index

The index stores absolute, canonical paths. On Windows they use the drive-letter
form. On all platforms, `--under PREFIX` matches on the stored string, so use the
same form the index shows in `list`.
