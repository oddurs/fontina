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

### M2 — Typography — delivered 2026-09-05
Laid out as pull requests in §10 and shipped in that order.
1. **Share the judgements** (#41). The opinions a specimen makes — feature labels, the
   waterfall ladder, axis steps, sample text — move out of `specimen.rs` into
   `typography`, so the browser and the HTML specimen cannot drift.
2. **Set the font** (#44). Axis sliders with named-instance snapping and feature toggles
   in the details pane, feeding `render::RenderOptions`.
3. **See the coverage** (#48). A glyph map by Unicode block with a codepoint search, and
   `unicode::cell_for`, which stopped `glyphs` printing U+202E raw into the terminal.
4. **Compare** (#49). Waterfall and comparison as one scrolling sheet, laid out once per
   width rather than per frame.
5. **Say whether it is free** (#55). The freedom verdict and its reason in the details
   pane, and a `freedom` facet, so `--free` is reachable from the browser.
6. **Check more** (#60). `name/empty`, `name/whitespace`, `metrics/line-gap`,
   `metrics/x-height`, `cmap/private-use`, and a test that keeps the published list of
   check ids honest.
7. **Come back after a reboot** (#64). An optional per-user login agent that runs
   `restore`: a systemd user unit, a LaunchAgent, a Startup script.

Reviewed as a whole afterwards (#76), which found what seven separate reviews could not.
The optional Google Fonts offline index is **not** delivered and is still held out; see
§10.

### M3 — Ecosystem — delivered 2026-09-05
Laid out as pull requests in §11 and shipped in that order.
1. **Let a collection travel** (#83). Relative paths in a collection export, which
   refuses to half-succeed: a file claiming its paths are relative while some are
   absolute is worse than one claiming nothing.
2. **Hand it over** (#85). `collection export --bundle <dir>` writes the JSON beside a
   copy of every font, nothing in it naming the machine it was made on. Dropbox,
   Syncthing or git carry it; `collection import <dir>` opens it.
3. **Prove it** (#87). The round trip across two indexes, through the binary, with the
   bundle read from somewhere other than where it was written — and a colleague who
   already owns the fonts keeping their own copy rather than gaining a duplicate.
4. **Tags the desktop can see** (#88). Finder tags through CoreFoundation on macOS,
   `user.xdg.tags` on GNU/Linux, and an honest `Unsupported` on Windows, whose keywords
   are per file format and a font file has none.
5. **Carry them across** (#89). `fontina tag sync --to-files | --from-files`. Directional
   because two tag sets with no common ancestor cannot tell a deletion from an addition,
   and a guess that loses a tag loses it silently. Never writes to an OS font directory.
6. **See it properly** (#90). `s` in the browser writes a specimen for the selection and
   opens it in `$BROWSER`. The whole of the graphical escape hatch, in one keystroke.
7. **Pipe into it** (#91). Targets from standard input, in fontina's own `--json` shape
   or one per line, so a program can write to fontina as well as read from it.
8. **Promise it** (#92, ADR 0008). What the CLI surface guarantees — ids, check ids, JSON
   field names, exit codes — what may be added, and what is not promised. With tests,
   because an ADR nothing checks is a wish.

Two questions M3 opened with were decided rather than deferred, in §11: **no graphical
shell yet** (the gap the TUI leaves is fidelity, and item 6 closes it without a second
interface to maintain), and **no Google Fonts index in this tree** (discovery that cannot
lead to acquisition is half a feature; items 7 and 8 make a catalogue an external program
that pipes candidates in).

Explicit non-goals, unchanged: font editing, format conversion/subsetting (point to
`fonttools`), cloud sync, accounts, telemetry, an Electron shell.

### M4 — Ask (everything the index knows, askable)
The metadata model has been complete since M0. The query surface has not caught up with
it, so four kinds of question the index holds the answer to cannot be put to it.

1. **Variable ranges.** `faces.weight` and `faces.width` hold the *default instance*
   value, and filtering is a point test against it. Bricolage has a `wght` axis of
   200–800 with a default of 800, so `list --weight 400` does not return it — a font
   that can plainly set 400. Every variable font is under-matched by the filter people
   reach for first.
2. **Language.** Language system tags per OpenType script are parsed and stored, and so
   is BCP 47 for every localised name record, and neither is reachable: `FaceFilter` has
   no language field. "Which faces declare Vietnamese" is unanswerable from the index
   that knows.
3. **Script depth.** `--script` is `f.scripts LIKE '%,Arab,%'` against a denormalised,
   unindexed string. It cannot express two scripts at once, cannot threshold on how much
   of a script is covered — though `Coverage.scripts` counts exactly that — and scans.
   A real library makes the cost plain: `--script Bamu` returns 98 faces that hold one
   Bamum codepoint each, ranked exactly like a Bamum font would be.
4. **Spacing.** `Metrics.is_fixed_pitch` is parsed and stored and reaches neither a
   filter nor a facet. On a working developer's library — 149 faces, 126 of them patched
   monospace — "show me the monospace ones" is the most useful single division available
   and there is no way to ask for it.

None of it needs a new parse or a rescan. Every value is already in the stored
`FaceMetadata`, so each item is index work with a backfill keyed on its migration index,
the pattern `face_ranges` established. `SCHEMA_VERSION` does not move: the model is not
changing, only what the index can be asked about it.

There is a fifth gap and it is a different kind. The four above are values the index
stores and cannot be asked about. This one is a relationship the index never derives.

fontina groups faces two ways today: by what the font **declares** — the family in its
`name` table — and by what a person **asserts** — tags and collections. It has no
grouping derived from what a face *is*, so where the declared families are wrong there is
nothing to fall back on. They are wrong often. That same 149-face library reports twenty
families and holds about eight typefaces, because `FiraCode Nerd Font`, `FiraCode Nerd
Font Mono` and `FiraCode Nerd Font Propo` are one typeface with its patch spaced three
ways, and nothing inside the files says so: not one of them declares a WWS family name.
There is no standard to read the answer from, which makes what to build here a narrower
question than it first looks. §12 says what to do, and what not to.

M4 does not depend on M3 and M3 does not depend on M4; the numbering is order of
discovery, not order of work. Item 1 is closer to a bug than a milestone item and should
be pulled out and shipped whenever someone has an hour.


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

Items 1-7 are delivered (#41, #44, #48, #49, #55, #60, #64), and the finished surface was
reviewed as a whole in #76.

The optional Google Fonts offline index was held out here and has since been decided
against; the reasoning is in §11, and the plugin surface is what replaces it.

---

## 11. M3, concretely

One pull request per item, in this order. M3 is not one theme the way M2 was: it is four
separable pieces of ecosystem work, and the two questions it opened with are now decided
below.

Where M2 built on plumbing that was already there, M3 mostly builds on the **collection
export**, which since #14 already carries an identity hash, a BLAKE3 and the tags, and
already matches on import by identity hash, then PostScript name, then path. That is the
hard half of sharing, and it is done.

### Sharing a folder

1. `feat(core)`: relative paths in a collection export. `CollectionFace::path` is absolute
   today, so an export names a directory that means nothing on anyone else's machine —
   the identity hash saves the import, and the path is dead weight that leaks a home
   directory. Add a base the paths are relative to, keep absolute exports working, and
   bump `schemas/collection.json`.
2. `feat(cli)`: `collection export --bundle <dir>`. Writes `collection.json` beside copies
   of the fonts, with every path relative to the bundle, so the folder can go in
   Dropbox, Syncthing or git and be imported anywhere. `collection import <dir>` resolves
   against the bundle and keeps the existing matching, so a teammate who already has the
   font keeps their own copy instead of gaining a duplicate.
3. `test(core)`: a round trip across two indexes — export from one, import into another
   whose fonts live elsewhere, and assert the collection arrives with its order and its
   tags, and that a face already present is matched rather than duplicated. This is the
   test that says "team sharing works"; without it the feature is a claim.

### Tags where the file manager can see them

4. `feat(platform)`: read and write the operating system's own file tags. macOS keeps
   Finder tags in the `com.apple.metadata:_kMDItemUserTags` extended attribute as a binary
   plist; GNU/Linux desktops use `user.xdg.tags`, comma-separated. Windows has no
   equivalent for an arbitrary file — the Property System's keywords are per-format and
   not available for fonts — so it returns `Unsupported`, and the CLI says so rather than
   pretending.
5. `feat(cli)`: `fontina tag sync --to-files | --from-files`. Explicitly directional. A
   two-way merge of two tag sets with no common ancestor cannot tell a deletion from an
   addition, so guessing loses tags silently; the reader says which side is right.

### Seeing type properly, without a second interface

6. `feat(cli)`: a key in the browser that writes a specimen for the selection and opens
   it. The status line already prints `fontina specimen 42 43`; nothing runs it. This is
   the whole of the graphical escape hatch, and it is one keystroke — see the shell
   decision below, which it exists to settle.

### A plugin surface that is a promise, not an accident

7. `feat(cli)`: read targets from standard input where a command takes them, so a program
   can pipe into fontina as well as out of it (`fontina list --json | jq … | fontina tag
   add serif -`). Everything else the surface needs already exists — every command has
   `--json`, every printed type is in `schemas/cli-output.json`, and CI diffs it.
8. `docs`: an ADR stating what the surface guarantees. Which parts are stable (ids,
   check ids, JSON field names, exit codes), what may be added (fields, never removed),
   and what a plugin may assume. Without that written down, "the CLI is the plugin API"
   is a description of today rather than a promise about tomorrow.

Items 1-8 are delivered (#83, #85, #87, #88, #89, #90, #91, #92).

### Two decisions

Both were open questions when §11 was written. Both are answered no, for different
reasons, and both are recorded here so nobody has to re-derive them.

**No graphical shell yet.** The gap the TUI leaves is real but narrow, and it is one
thing: fidelity. A terminal is worst at exactly what choosing a typeface needs — real
antialiasing at text sizes, spacing you can trust, hinting. Half-blocks give two pixels
per cell and even the image protocols are bounded by the cell grid. Everything else a
shell would add — a wall of families at once, drag and drop, reaching people who do not
live in a terminal — is an argument for a *different product*, not evidence that this one
falls short.

And the escape hatch already exists unbuilt: `specimen.rs` renders in a real browser, and
item 6 is the one keystroke that reaches it. Use that for a month and write down what is
still missing. If the answer is "I press the key and it is fine", a second interface has
been avoided; if something specific remains, its shape will be known rather than guessed.

ADR 0006 deferred the desktop because truthful previews were its one real benefit, and
#21 and #44 then delivered those in the terminal. The premise of that deferral got
stronger, not weaker — reversing it now would be acting on evidence that has since moved
the other way.

**No Google Fonts index in this tree**, and items 7 and 8 are why. Discovery that cannot
lead to acquisition is half a feature: search, find, leave for a browser, download, come
back and scan — and the half fontina would own is the half that matters least. It is also
an adjacent product; fontina manages the fonts you have and says what you may do with
them, while a catalogue is a different tool that shares a data model.

Once targets arrive on standard input and the JSON contract is an ADR, a catalogue is an
external program that pipes candidates in. The project gets the capability without the
crate, the dependency, the packaging or the network question — which is the plugin
surface paying for itself on its first real case.

There is a freedom argument too, and it is not about the fonts: Google Fonts is OFL. It
is that the *curation* is one company's list, and building one company's view of what
exists into the tool is what a free program should avoid. Someone should be able to point
the same mechanism at Debian's fonts, or a foundry's own index, and have it work
identically.

### Not M3, but before 1.0

Homebrew, winget, Scoop and AUR manifests (§5, M1 item 6) are still outstanding. They are
independent of everything above and block a 1.0 rather than M3.

---

## 12. M4, concretely

One pull request per item. The first three are one theme — a value the index stores but
cannot be filtered on gets a column or a table and a filter — so they share a shape:
migrate, backfill from the stored `FaceMetadata`, widen `FaceFilter`, expose the flag,
add a facet, test against a fixture that would fail before.

Nothing here parses anything new. That is the point: the cost is a migration and a
backfill, not a rescan of everyone's library.

### The variable range

1. `fix(core)`: index the ranges a variable font actually spans. Four columns —
   `weight_min`, `weight_max`, `width_min`, `width_max` — defaulting to the static value
   for a non-variable face, backfilled from `variable.axes` where a `wght` or `wdth` axis
   is present. The filter becomes an overlap test (`weight_min <= hi AND weight_max >=
   lo`) rather than `weight BETWEEN`. The fixture test is Bricolage: `wght` 200–800,
   default 800, and `list --weight 400` must return it. It does not today, and that
   assertion is the whole reason this item exists.
2. `feat(cli)`: say so in the output. `list` prints the default instance and gives no
   sign that a face reaches further; a variable face should show its range where it has
   one, and `facets` should count a variable face into every weight bucket it covers
   rather than only the bucket its default sits in.

### Script, with depth

3. `feat(core)`: `face_scripts(face_id, script, codepoints)`, indexed on both columns,
   backfilled from `coverage.scripts`. `faces.scripts` stays for now: it is what the
   browser's facet list reads, and one change at a time.
4. `feat(cli)`: `--script` becomes repeatable and gains a companion. Two occurrences mean
   both scripts, which the `LIKE` could never express, and `--script-min <n>` filters on
   coverage depth, so a font with three Arabic codepoints stops ranking with one that has
   three thousand. Sort the facet by codepoints rather than by name while there.

### Language

5. `feat(core)`: `face_languages(face_id, tag, source)`, where `source` distinguishes a
   language system tag declared under an OpenType script from a BCP 47 tag on a name
   record. They are different claims — one says the shaping engine has rules for it, the
   other only says the font names itself in it — and collapsing them would produce a
   filter that lies in one direction.
6. `feat(cli)`: `--lang <tag>`, a `language` facet, and the language list in `info` and
   in the browser's details pane. `--lang vi` is the question a person actually has.

### Spacing

7. `feat(core)` and `feat(cli)` together, because it is one column and two flags:
   `is_fixed_pitch` becomes a column, `--mono` and `--proportional` filter on it, and
   `spacing` becomes a facet. Report what `post.isFixedPitch` says and nothing more; a
   font whose advance widths contradict its own flag is a health check
   (`metrics/fixed-pitch`), not a filter that quietly disagrees with the file.

### Grouping the index can derive

The declared family is often the wrong unit and there is no standard that says so. The
temptation is a stored superfamily: a second grouping the index computes once and keeps.
**Do not build that.** Deciding what belongs together needs a rule, the only rule
available is a naming convention, those conventions belong to other projects and change
without telling anyone, and a stored grouping that is wrong is worse than none at all
because everything downstream inherits the mistake. It is also plainly the invention the
project rules out.

A question is the right shape instead. Asked of one face it is answerable from evidence
already in the index, it costs nothing when nobody asks, and when it is wrong it is wrong
once rather than permanently.

8. `feat(core)`: `Index::related(face_id, min)`. Jaccard over the codepoint sets —
   `|A ∩ B| / |A ∪ B|` — computed straight from `face_ranges`, which already holds both
   as sorted ranges indexed by face, so an intersection is a linear merge and the whole
   query is one pass over the library. No new table, no new parse, nothing stored.
9. `feat(cli)`: `fontina variants <target> [--min 0.9]`. Prints each candidate with its
   overlap and the four numbers that say whether "covers the same characters" means "is
   the same design": units per em, ascender and descender, fixed pitch, glyph count.
   High overlap with identical metrics is a variant of one typeface; high overlap with
   different metrics is two fonts that happen to serve the same languages. The score is
   printed rather than thresholded away, so the reader sees 0.98 and 0.62 and draws the
   line, which is the same discipline `freedom` follows.

   This is why it is not `dupes` and does not become a flag on it. `dupes` sweeps the
   whole library, and can, because exact identity is hash equality: group by
   `identity_hash`, one pass. Similarity has no such trick — it is pairwise, and a sweep
   is quadratic over a library that may hold tens of thousands of faces. Same axis,
   different cost, so a different shape: `dupes` sweeps, `variants` answers about a
   target.

### Keeping it honest

10. `test(core)`: a fixture-backed test per filter, each asserting the case that fails
    today. A filter without one is a claim. For `related`, the fixtures already hold the
    case that matters most: `inter-latin-400-normal.woff` and `.woff2` cover exactly the
    same 230 codepoints in the same 31 ranges, and are built three glyphs apart — 515
    against 518 — so they must score 1.0 while remaining two different files. That is the
    whole argument for printing the metrics beside the score rather than thresholding on
    it: identical coverage is not identity, and the pair that proves it is already in the
    repository. Amiri against Source Serif must score near zero.
11. `docs`: regenerate `schemas/cli-output.json` for the new filter fields, the new
    facets and the `variants` output type. `schemas/face.json` does not move —
    `FaceMetadata` is unchanged, which is worth saying out loud in the pull request so no
    one goes looking for a `SCHEMA_VERSION` bump that should not be there.

### The two questions behind all of it

Items 1 to 7 are one mistake found five times: a value was modelled richly, then flattened
to one scalar on the way into SQL because that was what the first query needed.
`face_ranges` is the counter-example, and it was added late and for one purpose,
`covering`. Worth asking once, before M4 rather than after: what else in `FaceMetadata` is
stored as a document and queried as a string? `features.gsub`, `capabilities.color` and
`os2.codepage_ranges` are the remaining candidates, and none has a filter today.

Items 8 and 9 are a different question, and the more interesting one. Everything fontina
knows about how faces relate to each other, it was told — by the `name` table or by a
person. `related` is the first thing it works out for itself, and it does so from
evidence rather than from convention, which is the only version of that idea this project
can honestly ship. If it earns its place, the question to ask next is what else follows
the same rule: which relationships between faces are measurable rather than declared, and
what could be answered if they were.
