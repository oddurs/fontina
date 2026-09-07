---
id: 24
title: Nobody knows what the left pane is
type: feat
status: done
milestone: tui-craft
created: 2026-09-06
updated: 2026-09-06
priority: p0
effort: l
crate: ui
---

## Problem

Nobody knows what the left pane is. It is titled with a count — `1998 faces` —
and nothing in it says it is a filter, that Enter toggles a row, or that `x`
clears everything. It reads as a third readout beside the two readouts either
side of it.

What it shows compounds that. Against a real 1,998-font library, in a 40-row
terminal, the order is Weight (nine rows), Width (four), Style, Variable,
Spacing, Script, Language — and the pane ends. Vendor, Tag, Collection,
Licence, Freedom, Source and Container are below the fold and are never seen.
Those are the ways a person actually enters a library; weight is not. Nobody
opens a font manager thinking "show me the 500 Mediums".

The rest, in order of how much they cost:

- The counts are faces; the list beside them is families. `400 Regular 681`
  does not predict what pressing Enter does to a list of 341 families.
- Selecting `300 Light` filters to a band, 250–349, which the row does not say.
- Three of the six busiest scripts are not scripts: `Zyyy` 1996, `Zinh` 1045,
  `Zzzz` 769 — common, inherited, unknown. They crowd out Arab, Hebr, Geor.
- The language section mixes two namespaces without saying so: `en` is BCP 47
  from a name record, `TRK` is an OpenType language system.
- One value per facet. "Light or Regular" cannot be asked.
- Ninety-odd rows, flat, no jump and no collapse.

## Proposal

Two panes and a filter line, with the facets on demand.

    ┌ fontina ─────────────────────────────────────────────────────────┐
    │ filter  script Arab · foundry H&FJ      341 → 12 families   x clear│
    ├───────────────────────────────┬───────────────────────────────────┤
    │ families / faces              │ details                           │
    └───────────────────────────────┴───────────────────────────────────┘

- **The filter line is always visible**, names every active filter in the
  reader's words, says what it did to the count, and says how to clear it. The
  state stops being something you infer from a command at the bottom.
- **`f` opens "Narrow by"** as a panel: sections collapsed to their top three
  with `+9 more`, `/` to jump within it, Enter to toggle, Esc to close. You
  choose a filter in bursts; it should not hold a third of the width the rest
  of the time.
- **Ordered the way a person enters a library**: Script · Foundry · Tag ·
  Collection · Capability (variable, colour, monospace) · Weight and width ·
  Licence and freedom · Source · Format.
- **Counted in the unit of the list**, families with faces in brackets.
- **Several values per facet**, so Light *or* Regular is askable.
- Pseudo-scripts folded into one `common` row that can be expanded, and the two
  language namespaces labelled.

Discoverability is the point of the change. A panel with a name, a verb and a
way out is something a reader opens on purpose; a silent column of numbers is
something they scroll past for a month.

## Alternative considered

Keep the three panes and fix the order, the labels and the counts. Cheaper, and
it leaves the two things that actually confuse people: a control surface that
does not look like one, and ninety rows with no shape.

## Acceptance criteria

- [x] the filter state is visible without opening anything, and names the key
      that clears it
- [x] the facet panel opens on one key, closes on Esc, and is titled with a verb
- [x] sections are collapsed by default and ordered as above
- [x] counts are in the same unit as the list beside them
- [ ] a facet accepts more than one value — deferred to [[0026]]. Everything else
      here is drawing and ordering inside the browser; this one is a change to
      `FaceFilter`, to the SQL that builds the `WHERE`, and to the clap flags that
      expose it, and stapling it on would have made one pull request out of two
      arguments
- [x] the browser at 80 columns still shows a list and a details pane
