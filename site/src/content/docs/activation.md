---
title: Activation and installation
description: "making a font visible to other applications, per user, in place or by copying; scopes, conflicts, and restoring after a reboot."
order: 3
---

Indexing a font tells fontina about it. *Activating* it tells the operating system, so
that every other application can use it. fontina does this per user, without
elevation, and without touching a system font directory. Two commands do it in two
ways:

<dl>
<dt><code>fontina activate TARGETS...</code></dt>
<dd>Registers the font file <em>in place</em>, where it already lives. Nothing is copied.
Persistent for the user across logins unless <code>--session</code>, which lasts until
logout or reboot. Undo with <code>deactivate</code>.</dd>
<dt><code>fontina install TARGETS...</code></dt>
<dd>Copies the file into the per-user font directory, which the operating system
already watches. The copy is an ordinary file you can see and delete. Undo with
<code>uninstall</code>.</dd>
</dl>

Both take the usual targets: face ids, `family:<name>`, or indexed file paths
(with `path#1` for one face of a collection).

```
$ fontina activate family:Amiri
$ fontina activate 42 --session
$ fontina install 42
$ fontina activations
$ fontina deactivate family:Amiri
```

`activations` lists everything fontina has activated or installed, with its state
(`session`, `user` or `installed`) and when. The same state is a filter on `list`
(`--active`, `--activation user`) and a facet. The `A` flag in `list` marks an
active face.

## Which one to use

Activate when the font lives in a folder you keep (a project, a synced directory, a
foundry download) and you want it to stay there. Install when you want the font to
survive the folder going away, or when an application only sees fonts in the user
font directory. On Linux both are symlinks or copies under `$XDG_DATA_HOME/fonts`
and behave identically to fontconfig; the difference is only whether the original
must stay put.

## Conflicts

Two active fonts with the same PostScript name, or the same family and style, fight
over which one an application gets. Before activating, fontina checks the targets
against everything already active and everything in the operating system's font
directories. A clash stops the command with exit status 2 and prints what clashed.

```
$ fontina conflicts 42
$ fontina activate 42
1 conflict(s): Amiri Regular (Amiri-Regular) is already active from /usr/share/fonts/... Use `fontina activate --replace` to override.
$ fontina activate 42 --replace
```

`--replace` deactivates or uninstalls the conflicting faces first, but only the
ones fontina manages. A font in a system directory is reported and left alone;
that is the one case where the answer is "not with this tool".

## After a reboot

Persistent (`user`) activations and installs survive reboots because the operating
system remembers them. Session activations do not, by design. The index records
them, and `fontina restore` re-applies every recorded activation:

```
$ fontina restore
```

Run it from a login agent if you want session activations back automatically. It
is idempotent and quick. A systemd user unit on Linux:

```
# ~/.config/systemd/user/fontina-restore.service
[Unit]
Description=Re-apply fontina font activations

[Service]
Type=oneshot
ExecStart=%h/.local/bin/fontina restore

[Install]
WantedBy=default.target
```

```
$ systemctl --user enable --now fontina-restore.service
```

On macOS a LaunchAgent, on Windows a Run key or a scheduled task, does the same.
Packaged agents are on the [roadmap](../../roadmap/) and will be off by default.

## What each platform does

One trait, three implementations. The mechanism is always the operating system's
own per-user one, so removing fontina leaves everything reversible by hand.

| | `install` | `activate` |
|---|---|---|
| Linux and BSDs | symlink into `$XDG_DATA_HOME/fonts/fontina/` | symlink into `$XDG_DATA_HOME/fonts/fontina-active/`, declared to fontconfig by `~/.config/fontconfig/conf.d/50-fontina.conf`; `fc-cache` is run if present |
| macOS | copy into `~/Library/Fonts` | Core Text registration of the file, `user` or `session` scope |
| Windows | copy into `%LOCALAPPDATA%\Microsoft\Windows\Fonts` plus a value under `HKCU\...\Fonts` | `AddFontResourceEx` for the session, plus the registry value for `user` scope; `WM_FONTCHANGE` broadcast |

Details and caveats per platform are in [Platform notes](../platforms/).

## Exit status

`0` applied. `1` an error. `2` a conflict blocked the change and nothing was
applied; add `--replace` or resolve it by hand.
