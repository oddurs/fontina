# 0003 — Tauri 2 for the desktop shell

**Status:** accepted, 2026-09-03

## Context
The app must preview fonts truthfully (shaping, variable axes, OpenType features,
color fonts), be accessible, and stay small. Candidates: Tauri (system webview), egui,
iced, Slint.

## Decision
Tauri 2 with a Svelte 5 frontend. The system webview renders through the platform text
stack (CoreText, DirectWrite, FreeType), so previews match what users see elsewhere and
complex scripts, color fonts and `font-variation-settings` work without us writing a
text engine. Uninstalled fonts load through `@font-face` over a custom protocol.

## Consequences
Installer 5–12 MB, idle RAM 40–80 MB. WebKitGTK on Linux is the weakest webview and is
pinned through Flatpak. A native-rendered fallback is not planned.
