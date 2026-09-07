---
id: 42
title: Two thirds of the published JSON fields have no description
type: docs
status: backlog
milestone: unfiled
created: 2026-09-07
updated: 2026-09-07
priority: p2
effort: m
crate: workspace
---

## Problem

ADR 0008 makes `schemas/cli-output.json` the plugin contract, and schemars publishes each
field's `///` as its `description`. So a doc comment on a published type is interface
text, read by whoever is building against fontina.

An audit counted **222 of 296 published fields with no description at all** across the
three schemas. A plugin author gets a name and a type and nothing else for three fields
in four.

## Proposal

Not all 222 want prose. `CheckReport.family` and `Source.path` say what they are. The ones
worth writing are those whose name does not carry the meaning:

- counts whose unit is ambiguous — `Facets.faces` against `Facets.families`, the `Stats`
  fields, `Related.shared` and `Related.union`
- anything where the value's *domain* matters and is not in the type: which units,
  which scale, what an empty list means
- anything where absence and zero mean different things

Work through the list, write the ones that need it, leave the rest.

## Acceptance criteria

- [ ] every field whose name does not carry its meaning has a description
- [ ] the count is recorded here afterwards, so the next audit can see whether it moved

## Notes

Found by the schema audit that also produced the `PUBLISHED_TYPES` guard. Two of ADR
0008's claims held under that audit — every `Option` field really does
`skip_serializing_if`, and no description leaks a private identifier, a source path or a
pull request number. This is the one that did not.

Filed rather than fixed because a 222-field diff would be unreviewable and most of it
would be restating the field name in a sentence.
