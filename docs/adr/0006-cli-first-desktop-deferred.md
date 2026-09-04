# 0006 — CLI first; the desktop shell is deferred

**Status:** accepted, 2026-09-03

## Context
M1 was planned as a Tauri 2 + Svelte desktop app. Building it means a Node toolchain,
WebKitGTK on Linux, code signing and notarisation, and a second UI codebase, for one
benefit: truthful font previews. Meanwhile every feature the app needs (activation,
watched folders, tags, collections, facets) has to exist in the core and the CLI anyway.

## Decision
M1 ships as a CLI plus a ratatui TUI (`fontina ui`). Previews are rendered in the
terminal: harfrust for shaping, skrifa for outlines, a coverage rasteriser, and the kitty,
iTerm2 and sixel image protocols with a half-block fallback. ADR 0003 (Tauri for a
graphical shell) stays accepted but moves to M3, and only if the TUI leaves a real gap.

## Consequences
One binary, no runtime, Linux-first testing in real terminals. The core grows a `render`
module and two dependencies (`harfrust`, `ab_glyph_rasterizer`); the CLI grows `ratatui`
and `crossterm`. Complex-script previews are correct because shaping is HarfBuzz-grade,
not because a browser did it. People who want a window get the HTML specimen today and,
possibly, a thin shell later.
