---
id: 26
title: Several values in one facet, in the browser and on the command line
type: feat
status: backlog
milestone: tui-discovery
created: 2026-09-06
updated: 2026-09-06
priority: p1
effort: l
crate: core
---

## Problem

One value per facet. "Light *or* Regular" cannot be asked, and neither can "OFL or
Apache", or "these three foundries". The browser holds its selection as one value per
facet — `BTreeMap<Facet, String>` — because that is all `FaceFilter` can carry: every
field but `scripts` is an `Option`, and `scripts` is an AND (a face covering both), not
an OR.

This was the one acceptance criterion of [[0024]] that was left out of the Narrow-by
panel, deliberately and on its own: everything else in that item is drawing and
ordering inside `crates/fontina-cli/src/ui`, and this is a change to the core filter,
the SQL that builds the `WHERE`, and the clap flags that expose it. Stapling it to a
TUI redesign would have made one pull request out of two arguments.

## Proposal

Turn the single-valued filters into OR-groups in `FaceFilter`, and let the flags repeat
on the command line — `--tag serif --tag display` reads as "either", which is what a
person means when they say a flag twice:

- `weight`, `width` — `Vec<(u16, u16)>`, any band matching
- `lang`, `license`, `vendor`, `tag`, `collection`, `container`, `path_prefix` — `Vec`
- `freedom`, `activation` — `Vec` of the enum
- `scripts` stays an AND. Two scripts means a face that covers both, which is the
  useful question about coverage and is documented that way.

Each group is one `(a OR b OR c)` clause; the groups are still ANDed together, so
"Light or Regular, and Arabic" is one filter.

In the browser, `selected` becomes `BTreeMap<Facet, BTreeSet<String>>`, Enter adds and
removes rather than replaces, and the filter line says `weight Light, Regular` in the
reader's words.

## Acceptance criteria

- [ ] a facet in the Narrow-by panel holds more than one value, and the filter line
      names all of them
- [ ] a repeated flag on the command line means "either", for every flag listed above
- [ ] `command_line()` still emits a command that returns exactly what the screen shows,
      with several values in a facet
- [ ] one test per flag that repeats it and asserts the union, not the intersection

## Notes

`Index::where_for` is where the clause is built; `facets.rs` needs nothing, because a
facet count is already computed over whatever filter it is handed.
