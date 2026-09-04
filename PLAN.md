# unifont — project plan

A lightweight, cross‑platform, open‑source font manager. Rust core, thin native shell, open standards end to end.

> Naming note: "Unifont" is already the name of GNU Unifont, a well‑known bitmap font. Keep `unifont` as the working codename, but pick a distinct public name before the first release to avoid confusion and package‑registry collisions.

---

## 1. Positioning

| Existing tool | Platform | Stack | Weakness we exploit |
|---|---|---|---|
| FontBase | Mac/Win/Linux | Electron, closed source | 300 MB+ RAM, slow start, proprietary, paid tiers |
| Typeface, RightFont | Mac only | native, paid | not cross‑platform, not open |
| Font Manager (GNOME) | Linux only | GTK/Vala | Linux only, no library/CLI story |
| NexusFont | Windows only | legacy | unmaintained |
| ZFontManager | Mac/Win/Linux | Tauri + Rust | early, no CLI/library/schema, ad‑hoc data model |

**What makes this one the best:**

1. **Correctness first.** Parsing via Google's `fontations` (`skrifa`/`read-fonts`), the same code path Chrome and Skia now use. Family grouping from `name` IDs 16/17 and `STAT`, not filename guesses.
2. **A library and a CLI, not just a window.** `unifont-core` is a reusable crate. The CLI emits JSON against a published JSON Schema. The GUI is one client of the core.
3. **Open standards for every artefact.** OpenType/WOFF for input, CSS Fonts Level 4 for the style model, SPDX for licenses, XDG for paths, fontconfig for Linux integration, JSON Schema for exports, TOML for config, Fluent for localisation, SBOM + Sigstore for releases.
4. **Actually lightweight.** Hard budgets (section 6) enforced in CI, not aspirations.
5. **Offline and private.** No network call unless the user opts into a catalog. No telemetry, ever.
6. **First‑class on all three desktops.** Per‑user install, temporary activation and conflict detection implemented natively per OS, packaged through Flathub, Homebrew and winget.

---

## 2. Architecture

```
unifont/
  Cargo.toml                 # workspace
  crates/
    unifont-core/            # parsing, metadata model, index (SQLite), search, dedupe
    unifont-platform/        # activation backends: macos/, windows/, linux/
    unifont-cli/             # `unifont` binary
  apps/
    desktop/                 # Tauri 2 shell + Svelte 5 frontend
      src-tauri/
      src/
  schemas/                   # JSON Schema: face.json, collection.json, cli-output.json
  fixtures/                  # OFL test fonts (Noto, Inter, Roboto Flex, Amiri, Noto Color Emoji)
  docs/                      # mdBook: user guide, CLI reference, data model, ADRs
```

### 2.1 Core (`unifont-core`, pure Rust, no UI deps)

| Concern | Choice | Why |
|---|---|---|
| Font parsing | `skrifa` + `read-fonts` | zero‑copy, fuzzed, standards‑tracking, maintained by Google Fonts |
| WOFF2 decode | `woff2-patched` (pure Rust) or FFI to google/woff2 behind a feature flag | `read-fonts` does not decode WOFF2; evaluate both in M0 |
| Database | SQLite via `rusqlite` (bundled), WAL mode, FTS5 | single file, zero admin, fast facet queries, FTS for names |
| File watching | `notify` + `notify-debouncer-full` | cross‑platform, battle‑tested |
| Hashing | BLAKE3 of file bytes; plus a "font identity" hash of `name` + `glyf`/`CFF` for cross‑format duplicates | dedupe TTF vs OTF vs WOFF2 of the same face |
| Parallelism | `rayon` for scans | 10k files in seconds |
| Paths | `directories` crate (XDG on Linux, standard dirs on mac/win) | standards‑compliant config/data/cache locations |
| Config | TOML | human‑editable, Rust‑native |
| Errors | `thiserror` in the library, `anyhow`/`miette` at the edges | good diagnostics in the CLI |

**Metadata extracted per face** (this is the schema in `schemas/face.json`):
- file: path, size, mtime, BLAKE3, container (`ttf`, `otf`, `ttc`, `woff`, `woff2`, `dfont`, `pfb`, `bdf`), index within collection
- names: every `name` record with platform/encoding/language (IDs 0–25), with preferred family/subfamily (16/17) resolved over legacy (1/2), postscript name (6), version (5), designer/vendor/URLs (8, 9, 11, 12), license text + URL (13, 14) → mapped to an **SPDX identifier** when recognisable (`OFL-1.1`, `Apache-2.0`, `UFL-1.0`, `GPL-2.0-only WITH Font-exception-2.0`, `LicenseRef-Proprietary`)
- `OS/2`: weight class, width class, fsSelection, fsType (embedding rights), vendor ID, unicode ranges, codepage ranges, x‑height, cap‑height
- `head`/`hhea`/`post`: units per em, revision, created/modified, ascender/descender, italic angle, fixed pitch
- variable: `fvar` axes (tag, min/default/max, name), named instances, `STAT` axis values; `avar` presence
- features: `GSUB`/`GPOS` feature tags + scripts/languages
- coverage: `cmap` codepoints → Unicode block/script summary, glyph count
- capabilities: color (`COLR` v0/v1, `SVG `, `sbix`, `CBDT`), hinting (`fpgm`/`prep`/`cvt `), bitmap strikes, `MATH`, `kern`
- derived CSS descriptor: `font-family`, `font-weight` (1–1000), `font-stretch` (%), `font-style` (`normal` | `italic` | `oblique <angle>`), `unicode-range` — every face is expressible as a CSS `@font-face` rule (CSS Fonts Level 4 is the style model)

**Index schema (SQLite):** `files`, `faces`, `families`, `axes`, `instances`, `features`, `tags`, `face_tags`, `collections`, `collection_faces`, `sources` (watched folders), `activations` (state + scope + timestamp), `licenses`, `faces_fts` (FTS5 over names/designer/tags). Migrations via `rusqlite_migration`.

### 2.2 Platform backends (`unifont-platform`)

One trait, three implementations, integration‑tested on a CI matrix:

```rust
pub trait FontActivator {
    fn install(&self, face: &FaceRef) -> Result<Installed>;     // persistent, per-user
    fn uninstall(&self, face: &FaceRef) -> Result<()>;
    fn activate(&self, face: &FaceRef, scope: Scope) -> Result<()>; // Scope::Session | Scope::User
    fn deactivate(&self, face: &FaceRef) -> Result<()>;
    fn enumerate_system(&self) -> Result<Vec<SystemFont>>;
    fn conflicts(&self, face: &FaceRef) -> Result<Vec<Conflict>>;
    fn subscribe_changes(&self) -> Receiver<SystemFontEvent>;
}
```

| OS | Persistent install | Temporary activation | Enumerate / change events | Crates |
|---|---|---|---|---|
| macOS | copy to `~/Library/Fonts` **or** register in place with `CTFontManagerRegisterFontURLs` scope `user` (persists across login without copying) | scope `session` | `CTFontManagerCopyAvailableFontURLs`, `kCTFontManagerRegisteredFontsChangedNotification` | `objc2`, `objc2-core-text` |
| Windows | per‑user (Win10 1809+): copy to `%LOCALAPPDATA%\Microsoft\Windows\Fonts`, write `HKCU\...\Fonts`, `AddFontResourceW`, broadcast `WM_FONTCHANGE` | `AddFontResourceExW` (no `FR_PRIVATE`), re‑applied at login by the optional agent | DirectWrite `IDWriteFontSet` / registry; `WM_FONTCHANGE` | `windows` |
| Linux | symlink into `$XDG_DATA_HOME/fonts/` | symlink into `$XDG_DATA_HOME/fonts/unifont-active/` declared via `~/.config/fontconfig/conf.d/50-unifont.conf`; deactivate = remove link | `fc-list`/`fontconfig` bindings; inotify on font dirs | `fontconfig` (or `fontconfig-sys`) |

Design rules:
- Never modify system font directories. Per‑user only. No elevation prompts.
- Activation state is persisted in the index so session activations can be restored at login by an **opt‑in** background agent (LaunchAgent / Run key / XDG autostart). Default off.
- Conflict detection: same PostScript name or same family+style already active from another path → warn before activating, with "replace" as an explicit action.
- Fallback for apps that only see machine‑wide fonts (some legacy Windows apps): document, don't work around.

### 2.3 Desktop shell (Tauri 2 + Svelte 5)

Why a webview rather than egui/iced/Slint for *this* app:

- **Previews must be truthful.** The webview renders through CoreText / DirectWrite / FreeType — the same stacks the user's other apps use. Shaping for Arabic, Devanagari, CJK, emoji, `COLRv1`, variable axes (`font-variation-settings`) and OpenType features (`font-feature-settings`) all come free and correct. Rebuilding that in Rust (parley + swash + vello) is a multi‑year project and still lags.
- **Uninstalled fonts preview trivially** via `@font-face { src: url(font://…) }` over a Tauri custom protocol. No activation needed to look.
- **Still light.** System webview means a 5–12 MB installer and 40–80 MB idle RAM (versus 300 MB+ for Electron). Accessibility, IME, RTL, screen readers, and i18n come from the platform.
- Cost acknowledged: WebKitGTK on Linux is the weakest webview. Flatpak bundles a known runtime; AppImage pins it.

Frontend rules: Svelte 5 + TypeScript, no component framework beyond that; virtualised grid (only visible previews get an `@font-face`), IntersectionObserver‑driven lazy loading, all state flows through typed Tauri commands (`specta`/`tauri-specta` for generated bindings). Keyboard‑first, WAI‑ARIA‑complete, respects `prefers-color-scheme` and `prefers-reduced-motion`.

### 2.4 CLI (`unifont`)

```
unifont scan <dir>...            index fonts (respects .gitignore-style excludes)
unifont list [--family X] [--variable] [--script Arab] [--license OFL-1.1] [--json]
unifont info <file|face-id> [--json]
unifont activate|deactivate|install|uninstall <face-id|file>... [--session]
unifont dupes [--json]           cross-format duplicate report
unifont collection export|import <name> [file.json]
unifont css <face-id>... > fonts.css   emit @font-face rules
unifont check <file>             fontbakery-lite health checks
unifont watch                    foreground watcher (for scripts / systemd user units)
```

`--json` output validates against `schemas/cli-output.json`. Exit codes documented. Shell completions generated by `clap_complete`. Man pages by `clap_mangen`.

---

## 3. Open standards inventory

| Area | Standard | Where used |
|---|---|---|
| Font formats | OpenType (ISO/IEC 14496‑22), TrueType, CFF/CFF2, WOFF 1.0, WOFF 2.0 (W3C) | parser, importer |
| Style model | CSS Fonts Level 4 (`font-weight` 1–1000, `font-stretch` %, `font-style oblique`, `unicode-range`) | data model, CSS export |
| Licensing | SPDX License List + SPDX expressions; OFL‑1.1 reserved font name handling | license facets, compliance view |
| Filesystem | XDG Base Directory spec; XDG autostart; Apple & Windows standard dirs | config, data, cache, agent |
| Linux fonts | fontconfig `fonts.conf` XML, `~/.local/share/fonts` | activation |
| Export/import | JSON with published JSON Schema (draft 2020‑12) | collections, tags, CLI output |
| Config | TOML 1.0 | settings |
| Localisation | Project Fluent (`.ftl`) | UI strings |
| Unicode | UCD blocks/scripts (via `unicode-script`, `unicode-blocks`); ICU segmentation in webview | coverage facets, glyph map |
| Catalog (opt‑in) | Google Fonts `METADATA.pb` / API; Fontsource | v2 online catalog |
| Packaging | Flatpak (Flathub), AppImage, `.dmg` + Homebrew cask, MSIX + winget, `.deb`/`.rpm` via `cargo-deb`/`cargo-generate-rpm` | releases |
| Supply chain | SBOM (CycloneDX + SPDX), Sigstore/cosign signatures, SLSA provenance via GitHub attestations, reproducible builds where feasible | CI |
| Accessibility | WAI‑ARIA 1.2, WCAG 2.2 AA | UI |
| License of the project | `MIT OR Apache-2.0` for crates (Rust convention, patent grant); same for the app | repo |
| Versioning | SemVer; Conventional Commits; Keep a Changelog | repo |

---

## 4. Feature scope by milestone

### M0 — Foundations (weeks 1–3)
- Workspace, CI matrix (ubuntu/macos/windows), fixtures, `cargo-deny`, `cargo-audit`, clippy‑pedantic, `cargo-fuzz` target for the import path.
- `unifont-core`: parse all formats, full metadata model, SQLite index, FTS, duplicate detection.
- CLI: `scan`, `list`, `info`, `dupes`, `css` with `--json` and schemas.
- ADRs written for: fontations, SQLite, Tauri, license.

### M1 — MVP desktop (weeks 4–9)
- Library view: virtualised grid/list, custom preview text, size, sample texts per script, dark/light.
- Watched folders; drag‑and‑drop import; system font view.
- Search + facets: family, weight, width, style, variable, color, script coverage, license, vendor, tags, collection, activation state.
- Activate / deactivate / install / uninstall on all three OSes; conflict warnings.
- Collections and tags; JSON export/import.
- Family grouping (name 16/17, STAT), duplicate view.
- Packaging: `.dmg`, `.msi`/MSIX, AppImage + Flatpak manifest. Signed and notarised.

### M2 — Pro typography (weeks 10–16)

*Delivered through core and CLI ahead of the shell (2026-09-04): `check`, `covers`,
`glyphs`, `license` with reserved font names, and the HTML `specimen` with axis
sliders, feature toggles, waterfall, glyph map and compare. The in-app versions reuse
these modules.*
- Variable axis sliders with named‑instance snapping; OpenType feature toggles with live preview.
- Glyph map by Unicode block with codepoint search; "which fonts cover this text?".
- Compare view (side‑by‑side and overlay), waterfall, paragraph/body test.
- Specimen export (HTML/SVG/PDF via webview print).
- License viewer with SPDX, embedding rights (`fsType`), reserved font names, expiry reminders for licensed fonts.
- `unifont check`: fontbakery‑lite (missing names, bad `OS/2`, broken cmap, license mismatch).
- Optional login agent to restore session activations.
- Optional Google Fonts offline index (opt‑in, no calls until enabled).

### M3 — Ecosystem (after 1.0)
- Team sharing via plain folders (Dropbox/Syncthing/git): collection JSON + relative paths, no proprietary cloud.
- Finder‑tag / xattr sync on macOS; Windows properties.
- Minimal plugin surface via CLI + JSON (no in‑process plugins).
- Auto‑activation for Adobe apps via UXP is *evaluated*, not promised.

Explicit non‑goals: font editing, format conversion/subsetting (point to `fonttools`), cloud sync, accounts, telemetry.

---

## 5. Quality and testing

- **Unit + snapshot tests** on metadata extraction for every fixture (`insta`).
- **Property/fuzz tests**: `cargo-fuzz` on the import wrapper; corrupt‑font corpus from `fontations` and OSS‑Fuzz.
- **Platform integration tests** behind `--features platform-tests`, run on the CI matrix in a throwaway user profile: install → enumerate → conflict → uninstall round‑trips.
- **UI tests**: `tauri-driver` + WebdriverIO smoke flows; Playwright component tests for the Svelte side.
- **Performance tests** in CI with a synthetic 10k‑file corpus; budgets below are hard failures.
- **Accessibility**: axe‑core in CI; manual VoiceOver/NVDA/Orca pass per release.

---

## 6. Performance budgets (CI‑enforced)

| Metric | Budget |
|---|---|
| Installer size | ≤ 15 MB (macOS/Windows), ≤ 25 MB AppImage |
| Cold start to first paint, 5k faces | ≤ 300 ms |
| Idle RSS | ≤ 80 MB |
| Initial index, 10k files, SSD | ≤ 10 s |
| Incremental rescan, 1 changed file | ≤ 50 ms |
| Search keystroke → results | ≤ 30 ms for 50k faces |
| Grid scroll | 60 fps, no more than ~40 `@font-face` loads in flight |

---

## 7. Risks and mitigations

| Risk | Mitigation |
|---|---|
| Name collision with GNU Unifont | rename before first public release |
| WOFF2 decoder maturity in pure Rust | evaluate in M0; FFI to google/woff2 behind a feature flag as fallback |
| WebKitGTK rendering/deps on Linux | Flatpak runtime pinning; document; egui fallback never planned (see 2.3) |
| Windows per‑user fonts invisible to some legacy apps | document; offer "copy to user font dir" as default install path |
| macOS in‑place `user` scope registration breaks if files move | store canonical paths + BLAKE3; detect moves on rescan; offer "repair" |
| Family grouping heuristics | prefer `STAT`/name 16/17; expose "split/merge family" override stored in DB |
| Font‑file parsing attack surface | fontations is fuzzed; import runs in a `catch_unwind` boundary with size limits |

---

## 8. First two weeks, concretely

1. `cargo new --lib crates/unifont-core`, add `skrifa`, `read-fonts`, `rusqlite`, `blake3`, `rayon`, `notify`, `directories`, `serde`, `thiserror`.
2. Write `face.json` schema first; make `FaceMetadata: Serialize` match it; add a schema‑validation test.
3. Implement `Importer::from_path` for TTF/OTF/TTC; snapshot‑test against fixtures.
4. Add WOFF/WOFF2; decide decoder (ADR‑002).
5. SQLite schema + migrations; `Index::scan(dir)` with rayon; FTS5.
6. CLI `scan`/`list`/`info`/`dupes` with `--json`.
7. Platform crate skeleton with the trait and a `enumerate_system` impl per OS.
8. CI matrix + `cargo-deny` + release‑please. Tag `v0.0.1` when `unifont scan ~/Library/Fonts | unifont list --json` works on all three OSes.
