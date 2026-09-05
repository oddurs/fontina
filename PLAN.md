# fontina — project plan

The font manager for the people. A single small binary that indexes, searches, previews,
activates and organises fonts on GNU/Linux, macOS and Windows, keeps everything in plain
files you own, never phones home, and will still work in twenty years.

> Naming note: the project was called `unifont` until 2026-09-04. That name belonged to
> GNU Unifont, a well-known bitmap font maintained under the GNU project, and taking it
> would have confused users of a package that had it first. `fontina` — the cheese, one
> letter from the thing it manages — is the public name.

---

## 1. Principles

These are the promises. Everything in the roadmap serves one of them.

1. **Terminal first.** The CLI is the product. Every capability exists as a command that
   prints text or JSON before it exists anywhere else. The TUI (`fontina ui`) and any
   later graphical shell are clients of the same core and add no capabilities of their own.
2. **Your data stays yours, in formats you can read.** One SQLite file for the index,
   JSON with a published schema for every export, TOML for config, fontconfig XML on
   GNU/Linux. Nothing that needs fontina to open. Delete the binary and your fonts, folders and
   OS font registrations are exactly as you left them; the index rebuilds from disk in
   seconds.
3. **Nothing leaves the machine.** No network calls, no accounts, no update checks, no
   crash reporting, no telemetry, ever. Online catalogs, if they ever come, are opt-in,
   separately packaged and off by default.
4. **Light.** A static binary with no runtime, no bundled browser, no daemon unless you
   ask for one. Hard budgets (section 6) are enforced in CI, not aspired to.
5. **Fast.** Ten thousand files indexed in seconds, any query in milliseconds, the TUI
   repaints within a frame. Parallel scans, an indexed database, no busywork at startup.
6. **Durable.** Append-only migrations, stable command output, stable check ids, semantic
   versioning. A script written against 1.0 runs against 3.0. Dependencies are few,
   audited, and replaceable; the only hand-written codec is WOFF1.
7. **Correct.** Parsing by Google's `fontations`, the code Chrome and Skia use. Shaping by
   `harfrust`, the HarfBuzz port on the same stack. Families from `name` IDs 16/17 and
   `STAT`, not filename guesses. Standards for every artefact: OpenType, WOFF, CSS Fonts 4,
   SPDX, XDG, fontconfig, JSON Schema, TOML.
8. **Every desktop is first class, GNU/Linux first.** Per-user install and temporary activation
   implemented natively on all three OSes, no elevation, never a write to a system font
   directory. GNU/Linux is the reference platform: fontconfig integration, distro packages,
   and the terminals people actually use (kitty, foot, GNOME Console, Konsole, WezTerm)
   are tested first.
9. **Free software, and it stays free.** GPL-3.0-or-later (ADR 0007): everyone who
   receives the program receives the freedom to run, study, change and share it, and
   nobody can take that away from the next person downstream. A public roadmap, ADRs for
   every structural decision, reproducible release builds with provenance and an SBOM.
   The manual is under the GFDL. And the program tells you the same thing about your
   fonts: `--free` filters the library down to the ones you may actually modify and
   redistribute, and `license` says why for each one.

## 2. Positioning

| Existing tool | Platform | Stack | Weakness we exploit |
|---|---|---|---|
| FontBase | Mac/Win/GNU/Linux | Electron, closed source | 300 MB+ RAM, slow start, proprietary, paid tiers |
| Typeface, RightFont | Mac only | native, paid | not cross-platform, not open |
| Font Manager (GNOME) | GNU/Linux only | GTK/Vala | GNU/Linux only, no library/CLI story |
| NexusFont | Windows only | legacy | unmaintained |
| ZFontManager | Mac/Win/GNU/Linux | Tauri + Rust | early, no CLI/library/schema, ad-hoc data model |
| fontpreview, fnt, fc-* | GNU/Linux | shell scripts, fontconfig | GNU/Linux only, no index, no metadata model, no activation |

**What makes this one the best:**

1. **A library and a CLI, not a window.** `fontina-core` is a reusable crate. The CLI
   emits JSON against published JSON Schemas. Pipes, scripts and editors are the first
   integration surface.
2. **Correctness.** fontations for parsing, harfrust for shaping, `STAT` and typographic
   names for grouping.
3. **Truthful previews without a browser.** Real shaped glyph rasters in the terminal
   through the kitty, iTerm2 and sixel image protocols, with a half-block fallback that
   works in every terminal, and a self-contained HTML specimen for when a browser is the
   right tool.
4. **Open standards for every artefact.** OpenType/WOFF in, CSS Fonts 4 style model, SPDX
   licenses, XDG paths, fontconfig for GNU/Linux, JSON Schema for exports, TOML config,
   SBOM + provenance for releases.
5. **Actually lightweight.** One binary, no runtime, budgets enforced in CI.
6. **Offline and private.** No network call, no telemetry, no account.
7. **First class on all three desktops.** Per-user install, temporary activation and
   conflict detection implemented natively per OS.

---

## 3. Architecture

```
fontina/
  Cargo.toml                 # workspace
  crates/
    fontina-core/            # parsing, metadata model, index (SQLite), search, dedupe,
                             # tags/collections/sources, checks, CSS, specimen, render
    fontina-platform/        # activation backends: macos/, windows/, linux/
    fontina-cli/             # `fontina` binary; `src/ui/` is the ratatui TUI
  schemas/                   # JSON Schema: face.json, collection.json, cli-output.json
  fixtures/                  # OFL test fonts, each under ~500 KB
  docs/adr/                  # architecture decision records
```

### 3.1 Core (`fontina-core`, pure Rust, no UI deps)

| Concern | Choice | Why |
|---|---|---|
| Font parsing | `skrifa` + `read-fonts` | zero-copy, fuzzed, standards-tracking, maintained by Google Fonts |
| WOFF2 decode | `woff2-patched` (pure Rust), see ADR 0005 | `read-fonts` does not decode WOFF2 |
| Shaping | `harfrust` | HarfBuzz port on `read-fonts`; one parsing stack |
| Rasterising | `skrifa` outlines + `ab_glyph_rasterizer` | tiny, dependency-free coverage rasteriser |
| Database | SQLite via `rusqlite` (bundled), WAL mode, FTS5 | single file, zero admin, fast facet queries, FTS for names |
| File watching | `notify` | cross-platform, battle-tested |
| Hashing | BLAKE3 of file bytes; plus a "font identity" hash of `name` + outlines | dedupe TTF vs OTF vs WOFF2 of the same face |
| Parallelism | `rayon` for scans | 10k files in seconds |
| Paths | `directories` crate | XDG on GNU/Linux, standard dirs on mac/win |
| Config | TOML | human-editable, Rust-native |
| Errors | `thiserror` in the library, `anyhow` at the edges | good diagnostics in the CLI |

**Metadata extracted per face** (this is the schema in `schemas/face.json`):
- file: path, size, mtime, BLAKE3, container (`ttf`, `otf`, `ttc`, `woff`, `woff2`), index within collection
- names: every `name` record with platform/encoding/language, with preferred family/subfamily (16/17) resolved over legacy (1/2), postscript name (6), version (5), designer/vendor/URLs (8, 9, 11, 12), license text + URL (13, 14) → mapped to an **SPDX identifier** when recognisable; OFL reserved font names
- `OS/2`: weight class, width class, fsSelection, fsType (embedding rights), vendor ID, unicode ranges, codepage ranges, x-height, cap-height
- `head`/`hhea`/`post`: units per em, revision, created/modified, ascender/descender, italic angle, fixed pitch
- variable: `fvar` axes (tag, min/default/max, name), named instances, `STAT` presence; `avar` presence
- features: `GSUB`/`GPOS` feature tags + scripts/languages
- coverage: `cmap` codepoints → Unicode script summary and merged ranges, glyph count
- capabilities: color (`COLR` v0/v1, `SVG `, `sbix`, `CBDT`), hinting, bitmap strikes, `MATH`, `kern`
- derived CSS descriptor: `font-family`, `font-weight` (1–1000), `font-stretch` (%), `font-style`, `unicode-range`

**Index schema (SQLite):** `files`, `faces`, `face_ranges`, `tags`, `face_tags`,
`collections`, `collection_faces`, `sources` (scanned and watched folders), `activations`
(state + scope + install path + timestamp), `faces_fts` (FTS5 over names/designer).
Migrations are append-only; one that needs data from the stored metadata JSON gets a
backfill function.

### 3.2 Platform backends (`fontina-platform`)

One trait, three implementations, integration-tested on the CI matrix:

```rust
pub trait FontActivator {
    fn install(&self, file: &Path) -> Result<PathBuf>;      // persistent, per-user copy
    fn uninstall(&self, installed: &Path) -> Result<()>;
    fn activate(&self, file: &Path, scope: Scope) -> Result<()>; // Scope::Session | Scope::User
    fn deactivate(&self, file: &Path) -> Result<()>;
    fn font_dirs(&self) -> Vec<SystemFontDir>;
}
```

| OS | Persistent install | Temporary activation | Crates |
|---|---|---|---|
| GNU/Linux | symlink into `$XDG_DATA_HOME/fonts/fontina/` | symlink into `$XDG_DATA_HOME/fonts/fontina-active/`, declared in `~/.config/fontconfig/conf.d/50-fontina.conf`; deactivate = remove link | none (filesystem + `fc-cache` when present) |
| macOS | copy to `~/Library/Fonts` | `CTFontManagerRegisterFontsForURL` with scope `session` or `user` (persists across login without copying) | `core-foundation` + three CoreText externs |
| Windows | per-user (Win10 1809+): copy to `%LOCALAPPDATA%\Microsoft\Windows\Fonts`, write `HKCU\...\Fonts`, `AddFontResourceW`, broadcast `WM_FONTCHANGE` | `AddFontResourceExW` (no `FR_PRIVATE`), re-applied at login by the optional agent | `windows-sys` |

Design rules:
- Never modify system font directories. Per-user only. No elevation prompts.
- Activation state is persisted in the index so session activations can be restored at
  login by an **opt-in** agent (`fontina restore` from a systemd user unit, LaunchAgent or
  Run key). Default off.
- Conflict detection is a core query: same PostScript name, or same family and style,
  already active or already present under a system font directory from another file.
  The CLI warns and requires `--replace`.
- Apps that only see machine-wide fonts (some legacy Windows apps): document, don't work
  around.

### 3.3 Terminal UI and previews

`fontina preview` and `fontina ui` show real glyphs, not filenames:

1. harfrust shapes the sample text with the face (correct Arabic, Indic, CJK, emoji
   sequences, kerning, ligatures, variable coordinates).
2. skrifa produces outlines at the requested size and axis position; `ab_glyph_rasterizer`
   fills them into an 8-bit coverage bitmap.
3. The bitmap is emitted with the best protocol the terminal supports, detected from
   `TERM`, `TERM_PROGRAM` and a kitty query: **kitty graphics** (kitty, Ghostty, WezTerm,
   Konsole), **iTerm2 inline images** (iTerm2, WezTerm, mintty), **sixel** (foot, xterm,
   mlterm, Windows Terminal), and a **half-block fallback** (`▀`, two rows per cell, 24-bit
   colour) that works everywhere else, including CI logs and `less -R`.

The TUI is ratatui + crossterm: a searchable, filterable browser with facets on the left,
families or faces in the middle, details and preview on the right. Keys, not mice, though
mouse works. Every action in the TUI is one the CLI can do, and the status line shows the
equivalent command. Design follows the terminal's own theme (16-colour palette by default,
truecolor only for previews) so it looks native in any setup.

Frontend rules for any future graphical shell are in ADR 0003 and section 5, milestone M3.

### 3.4 CLI (`fontina`)

```
fontina scan <dir>... [--system] [--prune]      index fonts; directories become sources
fontina list [query] [filters] [--json]        faces; filters below
fontina families [query] [filters] [--json]    grouped by typographic family
fontina facets [filters] [--json]              counts per weight, width, script, license, vendor, tag, ...
fontina info <face|file> [--json]              everything known about a face
fontina preview <face|file>... [--text] [--size] [--axis wght=700] [--feature smcp]
fontina activate|deactivate <face|file>... [--session] [--replace]
fontina install|uninstall <face|file>...
fontina conflicts <face|file>... [--json]
fontina tag add|remove|list|rename|delete
fontina collection list|create|delete|rename|add|remove|show|export|import
fontina source list|add|remove                  scanned folders; `add` scans, `watch` follows
fontina watch                                   foreground watcher (scripts, systemd user units)
fontina restore                                 re-apply session activations (login agent)
fontina dupes [--json]                          cross-format duplicate report
fontina css <face>... [--url-prefix]            @font-face rules
fontina check <face|file>... [--strict]         fontbakery-lite health checks
fontina covers <text>                           faces whose cmap covers every character
fontina glyphs <face> [--block]                 coverage by Unicode block
fontina license [<face>...]                     SPDX, embedding rights, reserved font names
fontina specimen <face>... -o file.html         self-contained HTML specimen
fontina ui                                      the TUI
fontina completions <shell> | fontina man       shell completions, man pages
fontina schema [face|collection|cli-output]     the JSON Schemas
```

Filters: `--family`, `--weight 600-900`, `--width 75-100`, `--italic`, `--variable`,
`--color`, `--script Arab`, `--license OFL`, `--vendor`, `--tag`, `--collection`,
`--active`, `--container woff2`, `--under <path>`. A face target is an index id, a file
path, or `family:<name>`.

`--json` output validates against `schemas/cli-output.json`. Exit code 1 on error, 2 for
"conflicts, not applied". Shell completions by `clap_complete`, man pages by
`clap_mangen`, both shipped in release archives.

---

## 4. Open standards inventory

| Area | Standard | Where used |
|---|---|---|
| Font formats | OpenType (ISO/IEC 14496-22), TrueType, CFF/CFF2, WOFF 1.0, WOFF 2.0 (W3C) | parser, importer |
| Shaping | OpenType Layout via HarfBuzz semantics (`harfrust`) | previews, TUI |
| Style model | CSS Fonts Level 4 (`font-weight` 1–1000, `font-stretch` %, `font-style oblique`, `unicode-range`) | data model, CSS export |
| Licensing | SPDX License List + expressions; OFL-1.1 reserved font name handling | license facets, compliance |
| Filesystem | XDG Base Directory spec; XDG autostart; Apple & Windows standard dirs | config, data, cache, agent |
| GNU/Linux fonts | fontconfig `fonts.conf` XML, `~/.local/share/fonts` | activation |
| Terminal images | kitty graphics protocol, iTerm2 inline images, DEC sixel | previews |
| Export/import | JSON with published JSON Schema (draft 2020-12) | collections, CLI output |
| Config | TOML 1.0 | settings |
| Unicode | UCD blocks/scripts (`unicode-script`, `unicode-blocks`) | coverage facets, glyph map |
| Packaging | release archives with checksums; `.deb`/`.rpm` via `cargo-deb`/`cargo-generate-rpm`; Homebrew, winget, Scoop, AUR, nixpkgs manifests | releases |
| Supply chain | SBOM (SPDX), SLSA provenance via GitHub attestations, reproducible builds where feasible | CI |
| License of the project | `GPL-3.0-or-later`; GFDL 1.3+ for the manual | repo |
| Versioning | SemVer; Conventional Commits; Keep a Changelog | repo |

---

## 5. Roadmap

### M0 — Foundations (done, 2026-09-03)
- Workspace, CI matrix (ubuntu/macos/windows), fixtures, `cargo-deny`, clippy, release
  binaries with provenance and SBOM, release-please.
- `fontina-core`: parse all formats, full metadata model, SQLite index, FTS, duplicate
  detection, health checks, CSS export, HTML specimen, coverage queries.
- CLI: `scan`, `list`, `info`, `dupes`, `css`, `stats`, `dirs`, `check`, `covers`,
  `glyphs`, `license`, `specimen`, `schema`.
- ADRs for fontations, SQLite, Tauri (deferred, see ADR 0006), license, WOFF decoding.

### M1 — Manage (the font manager, in the terminal) — delivered 2026-09-04
Re-scoped on 2026-09-03 from a desktop app to CLI + TUI (ADR 0006).
1. **Organise** (#14). Tags, collections, JSON export/import against
   `schemas/collection.json`, sources, family grouping, facet counts, richer filters.
   `families`, `facets`, `tag`, `collection`, `source`.
2. **Activate** (#16). Native `activate`/`deactivate`/`install`/`uninstall` on Linux,
   macOS and Windows; conflict detection with `--replace`; activation state in the
   index; `restore` for login agents.
3. **Watch** (#19). `source add` scans immediately; `watch` follows every watched source
   with debounced incremental rescans.
4. **Preview** (#21). Shaped, rasterised previews in the terminal (kitty, iTerm2, sixel,
   half-block fallback), with axis coordinates and feature toggles.
5. **Browse.** `fontina ui`: search, facets, families and faces, details, previews, tag
   and activate from the keyboard.
6. **Ship.** Completions and man pages in archives; `.deb`/`.rpm` from the release
   workflow. The rename to `fontina` and the move to GPL-3.0-or-later landed in #25
   (ADR 0007). Still to do before 1.0: Homebrew formula, winget and Scoop manifests,
   AUR `PKGBUILD`.

### M2 — Typography
- In the TUI: axis sliders with named-instance snapping, feature toggles, glyph map by
  block with codepoint search, compare and waterfall views, and a license viewer that
  gives the freedom verdict and its reason, not only the SPDX identifier.
- Laid out as pull requests in §10.
- `check` grows toward fontbakery parity where it is cheap; stable ids never change.
- Optional login agent packaging (systemd user unit, LaunchAgent, Run key), off by default.
- Optional Google Fonts offline index, separately packaged, opt-in.

### M3 — Ecosystem and shells
- Team sharing via plain folders (Dropbox/Syncthing/git): collection JSON with relative
  paths, no proprietary cloud.
- Finder-tag / xattr sync on macOS; Windows properties.
- Plugin surface via CLI + JSON only (no in-process plugins).
- A graphical shell (Tauri 2, ADR 0003) as one more client of the core, only if the TUI
  leaves a real gap. It must meet the same budgets and design rules: system webview,
  platform-adjacent design per OS (GNOME HIG on GNU/Linux, macOS HIG, Fluent), GNU/Linux first.

Explicit non-goals: font editing, format conversion/subsetting (point to `fonttools`),
cloud sync, accounts, telemetry, an Electron shell.

---

## 6. Quality and testing

- **Unit + snapshot tests** on metadata extraction for every fixture (`insta`).
- **Fixture-backed tests** for every health check id, every facet, every activation
  state transition (against a temporary index and a temporary font directory).
- **Fuzz**: `cargo-fuzz` (`fuzz/`, its own workspace, nightly) on two targets — `parse`,
  which drives `load_bytes` and so spends its budget in container detection and the
  WOFF/WOFF2 decoders, and `sfnt`, which drives `parse::parse_sfnt` directly so the
  mutator reaches the table readers instead of failing a magic-number check. Every run
  passes `-timeout` and `-rss_limit_mb`, because the `catch_unwind` in `scan::parse_paths`
  catches a panic and neither a hang nor a runaway allocation. The corpus is seeded from
  `fixtures/`; `scripts/fuzz` is the entry point, and `.github/workflows/fuzz.yml` runs a
  minute per target on a pull request that touches the crates, half an hour weekly, and
  whatever a manual dispatch asks for. Findings are kept as inputs in `fuzz/regressions/`
  and replayed on stable by `crates/fontina-core/tests/fuzz_regressions.rs`, which
  requires each of them to return, without panicking, inside a time bound.
- **Platform integration tests** behind `--features platform-tests`, run on the CI matrix
  in a throwaway user profile: install → enumerate → conflict → uninstall round-trips.
- **Render tests**: shaped previews snapshot as PNGs per fixture; half-block output
  snapshot as text.
- **Performance tests** in CI with a synthetic 10k-file corpus; budgets below are hard
  failures.

---

## 7. Performance budgets

Stated for the machine they are enforced on: a GitHub-hosted runner, which is around two
and a half times slower than a developer's laptop. `scripts/bench` measures these against
a corpus of real font files and fails on a miss;
`.github/workflows/perf.yml` runs it. The two marked *not measured* need a terminal, and
a number produced without one would be a number about nothing; they are checked by hand
until there is a harness that can hold a pty.

| Metric | Budget | Measured |
|---|---|---|
| Release binary, stripped | ≤ 12 MB per platform | yes |
| `fontina list` cold start to output | ≤ 80 ms | yes, at 5k faces |
| Initial index, 10k files, SSD | ≤ 10 s | yes |
| Incremental rescan, 1 changed file | ≤ 50 ms | yes |
| Search keystroke → a screenful of results | ≤ 30 ms | yes, at 10k faces |
| Preview render, one face, 64 px, 40 characters | ≤ 30 ms | yes |
| Idle RSS of `fontina ui`, 5k faces | ≤ 40 MB | not measured |
| TUI repaint | ≤ 16 ms | not measured |

The search budget is a screenful, not every match: the unbounded form measures the cost
of printing ten thousand lines rather than of finding them.

---

## 8. Risks and mitigations

| Risk | Mitigation |
|---|---|
| Name collision with GNU Unifont | rename before first public release |
| Terminals without image support | half-block fallback always works; HTML specimen for the browser |
| WOFF2 decoder maturity in pure Rust | ADR 0005; swap behind the `container` module |
| Windows per-user fonts invisible to some legacy apps | document; "copy to user font dir" is the install path |
| macOS in-place registration breaks if files move | store canonical paths + BLAKE3; detect moves on rescan; `activate` re-registers |
| Family grouping heuristics | prefer `STAT`/name 16/17; "split/merge family" override stored in the index (M2) |
| Font-file parsing attack surface | fontations is fuzzed; import runs in a `catch_unwind` boundary with size limits |
| Dependency drift | few deps, `cargo-deny`, Dependabot minor/patch auto-merged, majors by hand |

---

## 9. M1, concretely

Each item was one pull request against `main`, in this order.

1. `feat(core)` #14: tags, collections, sources and activation state APIs; facets and
   families; new filters; `schemas/collection.json` and `schemas/cli-output.json`;
   `fontina tag|collection|source|families|facets`.
2. `feat(platform)` #16: Linux, macOS and Windows activation backends; core conflict
   query; `fontina activate|deactivate|install|uninstall|conflicts|restore`.
3. `feat(core)` #19: `notify`-based watcher; `fontina watch`; `source add` scans.
4. `feat(core)` #21: `render` module (harfrust + skrifa + rasteriser), PNG encoder on
   `flate2`; `fontina preview` with kitty/iTerm2/sixel/half-block output.
5. `feat(cli)`: `fontina ui` (ratatui).
6. `chore(release)`: completions, man pages, `.deb`/`.rpm` in the release workflow.

What M2 starts from: the TUI has the plumbing for axis sliders and feature toggles
(`render::RenderOptions`), `check` has stable ids, and `restore` is ready for a login
agent. The rename landed in #25; package-manager manifests are the remaining "Ship" item.

---

## 10. M2, concretely

One pull request per item, in this order. Items 1–4 are the typography work and share a
dependency: 1 gives 2–4 their helpers.

Most of M2 is not new logic. `specimen.rs` already implements every one of these
features in HTML, and it is the reference implementation (CLAUDE.md); the work is
wiring core APIs the specimen already exercises into ratatui panes.

1. `refactor(core)`: lift the specimen's typography helpers into the core — OpenType
   feature labels, the waterfall size ladder, axis step calculation, named-instance
   snapping. All four are private to `specimen.rs` today and the TUI needs the same
   four; sharing them is what stops the two clients drifting apart.
2. `feat(cli)`: axis sliders and feature toggles in the details pane, feeding
   `render::RenderOptions`, which already carries `variations` and `features` and has
   nothing setting them. Note the preview cache key is `(face, text, size, width)`; it
   has to grow to include axes and features or a moved slider will not repaint.
3. `feat(cli)`: glyph map by Unicode block with codepoint search, over the public
   `unicode::glyph_map`, reusing the details pane's existing input mechanism.
4. `feat(cli)`: waterfall and compare views — the size ladder for one face, the sample
   text across several. `current_face_ids` already returns the selection.
5. `feat(cli)`: the license viewer. The freedom verdict and its reason from
   `freedom::assess`, embedding rights (reported, never enforced), reserved font names
   and copyright; plus a `freedom` facet, so `--free` is reachable from the TUI.
6. `feat(core)`: `check` toward fontbakery parity where it is cheap. Ids are additive —
   the ones that exist keep their names — and every new check needs a fixture-backed
   test that triggers it.
7. `chore(platform)`: optional login agent packaging, a systemd user unit, a LaunchAgent
   and a Run key entry, off by default, wrapping the `restore` M1 shipped.

Held out until it is decided: the optional Google Fonts offline index. It cannot live in
the core or the CLI, because neither makes network calls, so it wants its own crate, its
own binary and its own package, with the index shipped as a file rather than fetched.
Buildable, but a different kind of work from 1–7 and about as large as all of them.
