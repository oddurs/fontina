---
id: 55
title: Say whether the font is sound, not only what it looks like
type: feat
status: backlog
milestone: specimen
created: 2026-09-07
updated: 2026-09-07
priority: p1
effort: m
crate: core
---

## Problem

`check.rs` holds about forty checks with stable ids across `cmap/`, `metrics/`, `fvar/`,
`layout/`, `license/` and `name/`. It is the thing fontina has that a specimen generator
does not: anyone can draw a waterfall, and nobody else can tell you that a font's
`post.isFixedPitch` disagrees with its advance widths, or that it declares no kerning at
all, or that its typo and hhea metrics contradict each other.

The specimen writes a page that embeds a font binary and says nothing whatever about
whether the font is sound. The one view a person actually looks at is the one view that
withholds what fontina knows.

## Proposal

Run `check_face` and put the findings on the sheet, in the rail under the specifications:
the count by severity, and the findings themselves with their stable ids.

Report, never enforce — the house rule, and it matters here more than anywhere, because a
finding is information for a person deciding whether to use a font, not a verdict that the
font is unusable. A font with findings is still set at full size on the page above them.

Where a finding is about something the sheet already draws, say so next to it: a
`cmap/basic-latin` finding belongs beside the glyph map, `layout/kerning` beside the
feature toggles, `metrics/typo-vs-hhea` beside the metrics.

## Acceptance criteria

- [ ] The sheet carries the findings for its face, by stable id, with severity.
- [ ] A clean font says so rather than showing an empty area.
- [ ] Nothing is hidden, greyed or refused on the strength of a finding.
- [ ] Findings print with the sheet.

## Notes

Check ids are stable and are never renamed, so a printed specimen citing one stays
readable against a later version.
