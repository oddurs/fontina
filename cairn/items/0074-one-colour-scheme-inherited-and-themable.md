---
id: 74
title: One colour scheme, inherited and themable
type: feat
status: done
milestone: tui-craft
created: 2026-09-15
updated: 2026-09-15
priority: p1
crate: cli
effort: m
---

## Problem

The six roles had two definitions. `term::sgr` said `Role::Accent => "36"` for the
command line and `ui::Theme::accent` said `Color::Cyan` for the browser: the same
decision, in two files, in two vocabularies, with nothing keeping them in step and
nothing that would have noticed if they stopped.

That is also why the colours could not be configured. A setting can only override one
thing, and there were two.

## Proposal

`scheme.rs` holds the decision once; `term` and `ui::theme` both read it.

`[colours]` in the configuration file — `[colors]` too — names roles, and only the roles
it names move. It joins the precedence chain the file already had (flag, environment,
file, built-in), and `fontina config` prints the origin of each role.

Sixteen colours and no more, optionally with `bold` or `reverse`, or `none`. The limit
is what makes theming work: these are the colours the reader's terminal theme already
defines, so an accent of `cyan` is *their* cyan. No background — a role paints its text
and the ground stays theirs.

## Acceptance criteria

- [x] one definition of the six roles, read by both renderers
- [x] a file that names one role changes that role and no other
- [x] `NO_COLOR` outranks anything the file says
- [x] a value that is not a colour fails, naming what it would have taken
- [x] two roles that would look identical fail, naming both
- [x] `fontina config` reports each role with its source, and round-trips
- [x] the manual documents it

## Notes

Verified against the bytes rather than the source: with no configuration the escapes are
`1`, `90`, `36`, `32` — the literals that used to be in `term::sgr`. With a theme they
become `1;35`, `34`, `92`, and the old three are gone.

Found on the way: `Cli::with_env` in the testkit pushed onto a vector nothing ever read,
so it silently did nothing. Two tests set `COLUMNS` through it and had been running at no
width at all.

Shipped in #230.
