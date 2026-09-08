---
id: 69
title: The specimen has two tests for ten sections
type: test
status: backlog
milestone: unfiled
created: 2026-09-07
updated: 2026-09-07
priority: p2
effort: m
crate: core
---

## Problem

`specimen.rs` is 669 lines and ten section builders — `index`, `compare`, `sheet`,
`spec`, `controls`, `waterfall`, `scripts`, `glyph_map` and two more. Two tests cover it,
both in `tests/fixtures.rs`: `specimen_is_self_contained_html` and
`a_hostile_path_cannot_escape_the_specimen_style_element`.

CLAUDE.md calls the specimen "the reference implementation the desktop preview will
reuse". It is also the artefact a person is most likely to send to somebody who does not
have fontina, which makes its escaping a security property rather than a tidiness one —
and exactly one field is currently tested for it.

## Proposal

A snapshot per section, so a layout change has to be looked at rather than noticed later;
escaping asserted for every field that reaches the HTML rather than the one path that
prompted the existing test; and a face with every optional field empty, since a specimen
of a sparse font is the case least likely to have been tried by hand.

## Acceptance criteria

- [ ] a snapshot per section builder
- [ ] every field that reaches the document is escape-tested, not only the path
- [ ] a face with no optional metadata renders without panicking or emitting empty markup

## Notes

**Do not start before the specimen work in flight lands.** As of 2026-09-07 there is an
open pull request laying the specimen out afresh; snapshots written against today's
markup would be thrown away. See
[[0064-a-specimen-worth-printing]] and [[0061-a-specimen-you-can-send-someone]].
