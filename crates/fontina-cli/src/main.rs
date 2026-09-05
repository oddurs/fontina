// SPDX-License-Identifier: GPL-3.0-or-later
//
// fontina — a font manager.
// Copyright (C) 2026 Oddur Sigurdsson
//
// This program is free software: you can redistribute it and/or modify it under the
// terms of the GNU General Public License as published by the Free Software Foundation,
// either version 3 of the License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful, but WITHOUT ANY
// WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A
// PARTICULAR PURPOSE. See the GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with this
// program. If not, see <https://www.gnu.org/licenses/>.

mod ui;

use anyhow::{Context, Result, bail};
use clap::{Args, CommandFactory, Parser, Subcommand};
use fontina_core::{
    ActivationState, FaceFilter, FaceSummary, Freedom, Index, ScanOptions, SourceKind,
};
use std::io::Write as _;
use std::path::PathBuf;

// See the note in Cargo.toml. musl's own allocator makes a parallel scan four times
// slower, and the scan is the thing a user waits for.
#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

// The GNU Coding Standards ask `--version` to say who holds the copyright, under what
// licence the program is distributed, and that it comes with no warranty, so that a
// person who has only the binary can still find out what their rights are. `-V` keeps
// the short form for scripts that only want the number.
const LONG_VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    "\n",
    "Copyright (C) 2026 Oddur Sigurdsson\n",
    "License GPLv3+: GNU GPL version 3 or later <https://gnu.org/licenses/gpl.html>.\n",
    "This is free software: you are free to change and redistribute it.\n",
    "There is NO WARRANTY, to the extent permitted by law.",
);

/// fontina: a lightweight, standards-based font manager.
#[derive(Parser)]
#[command(
    name = "fontina",
    version,
    long_version = LONG_VERSION,
    about,
    propagate_version = true
)]
struct Cli {
    /// Path to the index database (default: the platform data directory).
    #[arg(long, global = true, env = "FONTINA_DB")]
    db: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Index fonts under one or more directories (or files).
    Scan {
        /// Directories or files to scan.
        paths: Vec<PathBuf>,
        /// Also scan the operating system's font directories.
        #[arg(long)]
        system: bool,
        /// Re-parse files even when size and mtime are unchanged.
        #[arg(long)]
        force: bool,
        /// Follow symlinks while walking.
        #[arg(long)]
        follow_symlinks: bool,
        /// Drop index entries under the scanned roots whose files no longer exist.
        #[arg(long)]
        prune: bool,
        #[arg(long)]
        json: bool,
    },
    /// List indexed faces, optionally filtered.
    List(ListArgs),
    /// List families (faces grouped by typographic family name), optionally filtered.
    Families(ListArgs),
    /// Count faces per weight, width, style, script, license, vendor, tag, collection,
    /// activation state and source, for the faces matching the filters.
    Facets(ListArgs),
    /// Tag faces. A tag is a free-form label; a face can carry many.
    #[command(subcommand)]
    Tag(TagCmd),
    /// Collections: ordered, named sets of faces that export to and import from JSON.
    #[command(subcommand)]
    Collection(CollectionCmd),
    /// Sources: the directories the index was built from; `watch` follows the watched ones.
    #[command(subcommand)]
    Source(SourceCmd),
    /// Make faces visible to other applications, in place. Persistent for the user unless
    /// `--session`. Exit code 2 when a conflict blocks it (see `conflicts`).
    Activate {
        /// Face ids, `family:<name>`, or indexed file paths.
        #[arg(required = true)]
        targets: Vec<String>,
        /// Until logout or reboot instead of persistently.
        #[arg(long)]
        session: bool,
        /// Deactivate or uninstall conflicting faces that fontina manages first.
        #[arg(long)]
        replace: bool,
        #[arg(long)]
        json: bool,
    },
    /// Undo `activate`.
    Deactivate {
        #[arg(required = true)]
        targets: Vec<String>,
        #[arg(long)]
        json: bool,
    },
    /// Copy faces into the per-user font directory. Exit code 2 on an unresolved conflict.
    Install {
        #[arg(required = true)]
        targets: Vec<String>,
        #[arg(long)]
        replace: bool,
        #[arg(long)]
        json: bool,
    },
    /// Remove the per-user copies made by `install`.
    Uninstall {
        #[arg(required = true)]
        targets: Vec<String>,
        #[arg(long)]
        json: bool,
    },
    /// Faces that would clash with these once active: same PostScript name or same
    /// family and style, already active or living in an OS font directory.
    Conflicts {
        #[arg(required = true)]
        targets: Vec<String>,
        #[arg(long)]
        json: bool,
    },
    /// Everything fontina has activated or installed.
    Activations {
        #[arg(long)]
        json: bool,
    },
    /// Re-apply recorded activations, for a login agent or after a reboot.
    Restore {
        #[arg(long)]
        json: bool,
    },
    /// The optional login agent that runs `restore` when you log in. Off until you
    /// install it, per-user, and removable with one command.
    #[command(subcommand)]
    Agent(AgentCmd),
    /// Follow the watched sources (and any extra directories) and keep the index
    /// current until interrupted. One line per batch of changes; `--json` for one
    /// JSON object per line.
    Watch {
        /// Extra directories to follow for this run.
        paths: Vec<PathBuf>,
        /// Quiet period in milliseconds before a batch is applied.
        #[arg(long, default_value_t = 500)]
        debounce_ms: u64,
        #[arg(long)]
        json: bool,
    },
    /// Show everything known about a face, by index id or by file path (parses the file when not indexed).
    Info {
        /// Face id from `list`, or a path to a font file.
        target: String,
        #[arg(long)]
        json: bool,
    },
    /// Report faces that are duplicates across containers or share a PostScript name.
    Dupes {
        #[arg(long)]
        json: bool,
    },
    /// Emit `@font-face` rules for faces by id, or for every face in a file.
    Css {
        /// Face ids or font file paths.
        targets: Vec<String>,
        /// Use this URL prefix instead of file:// paths (e.g. `/fonts/`).
        #[arg(long)]
        url_prefix: Option<String>,
    },
    /// Index statistics and recent failures.
    Stats {
        #[arg(long)]
        json: bool,
    },
    /// Print the operating system's font directories.
    Dirs {
        #[arg(long)]
        json: bool,
    },
    /// Run health checks (fontbakery-lite) on faces. Exit code 1 if any check errors.
    Check {
        /// Face ids or font file paths.
        targets: Vec<String>,
        /// Also fail on warnings.
        #[arg(long)]
        strict: bool,
        /// Hide findings below this level: info, warn or error.
        #[arg(long, default_value = "info")]
        min: String,
        #[arg(long)]
        json: bool,
    },
    /// Find indexed faces whose character map covers every character of a text.
    Covers {
        text: String,
        /// Only variable fonts.
        #[arg(long)]
        variable: bool,
        #[arg(long)]
        under: Option<String>,
        #[arg(long, short = 'n')]
        limit: Option<usize>,
        #[arg(long)]
        json: bool,
    },
    /// Show a face's character coverage by Unicode block.
    Glyphs {
        target: String,
        /// Print the characters of one block (case-insensitive substring of the block name).
        #[arg(long)]
        block: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// License and embedding report: SPDX identifier, embedding rights, reserved font names.
    License {
        /// Face ids or font file paths; every indexed face when omitted.
        targets: Vec<String>,
        #[arg(long)]
        json: bool,
    },
    /// Write a self-contained HTML specimen: waterfall, script samples, axis sliders,
    /// feature toggles, glyph map, and side-by-side comparison for several faces.
    Specimen {
        /// Face ids or font file paths.
        targets: Vec<String>,
        /// Output file; `-` for stdout.
        #[arg(long, short = 'o', default_value = "-")]
        output: PathBuf,
        /// Sample text.
        #[arg(long)]
        text: Option<String>,
        /// Reference font files by path instead of embedding them (smaller, but only
        /// works when served over HTTP or in browsers that allow file:// font loads).
        #[arg(long)]
        link: bool,
        #[arg(long)]
        title: Option<String>,
    },
    /// Show faces as real, shaped glyphs in the terminal (kitty, iTerm2 or sixel
    /// images, or half-block text anywhere), or write a PNG.
    Preview(PreviewArgs),
    /// Browse the index: facets, families, faces, details and previews, keyboard first.
    Ui,
    /// Print shell completions: bash, zsh, fish, elvish or powershell.
    Completions { shell: clap_complete::Shell },
    /// Print the man page, or write one page per command into a directory.
    Man {
        /// Write `fontina.1`, `fontina-scan.1`, ... here instead of printing `fontina.1`.
        #[arg(long)]
        out_dir: Option<PathBuf>,
    },
    /// Print a JSON Schema: `face` (default), `collection`, or `cli-output`.
    Schema {
        #[arg(default_value = "face")]
        which: String,
    },
}

#[derive(Args)]
struct PreviewArgs {
    /// Face ids, `family:<name>`, or font file paths.
    #[arg(required = true)]
    targets: Vec<String>,
    /// Sample text; `\n` for a new line. Defaults to a pangram, or the face's own
    /// sample text when it has one.
    #[arg(long, short = 't')]
    text: Option<String>,
    /// Font size in pixels.
    #[arg(long, short = 's', default_value_t = 48.0)]
    size: f32,
    /// Variable axis setting, e.g. `wght=700`; repeatable.
    #[arg(long = "axis", short = 'a', value_parser = parse_axis)]
    axes: Vec<(String, f32)>,
    /// OpenType feature to turn on (`smcp`) or off (`liga=0`); repeatable.
    #[arg(long = "feature", short = 'f', value_parser = parse_feature)]
    features: Vec<(String, bool)>,
    /// Output protocol: auto, kitty, iterm, sixel, blocks, or png (needs --output).
    #[arg(long, short = 'p', default_value = "auto")]
    protocol: String,
    /// Write a PNG here instead of drawing in the terminal (one face only).
    #[arg(long, short = 'o')]
    output: Option<PathBuf>,
    /// Ink colour, `#rrggbb`.
    #[arg(long)]
    fg: Option<String>,
    /// Background colour for sixel and blocks, `#rrggbb`.
    #[arg(long)]
    bg: Option<String>,
    /// Clip to this many pixels wide.
    #[arg(long)]
    max_width: Option<u32>,
}

fn parse_axis(s: &str) -> std::result::Result<(String, f32), String> {
    let (tag, value) = s
        .split_once('=')
        .ok_or_else(|| format!("expected tag=value, got {s:?}"))?;
    let v: f32 = value
        .trim()
        .parse()
        .map_err(|_| format!("bad axis value {value:?}"))?;
    Ok((tag.trim().to_string(), v))
}

fn parse_feature(s: &str) -> std::result::Result<(String, bool), String> {
    let (tag, on) = match s.split_once('=') {
        Some((t, v)) => (t, !matches!(v.trim(), "0" | "off" | "false")),
        None => (s, true),
    };
    let tag = tag.trim();
    if tag.len() != 4 {
        return Err(format!("{tag:?} is not a four-character feature tag"));
    }
    Ok((tag.to_string(), on))
}

#[derive(Subcommand)]
enum AgentCmd {
    /// Write the agent for this system. Prints where it went and, where the system
    /// needs one, the command that starts it now rather than at the next login.
    Install {
        #[arg(long)]
        json: bool,
    },
    /// Remove it.
    Uninstall {
        #[arg(long)]
        json: bool,
    },
    /// Whether one is installed, and what it would contain if it were not.
    Status {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum TagCmd {
    /// All tags with their face counts.
    List {
        #[arg(long)]
        json: bool,
    },
    /// Add a tag to faces (created if new).
    Add {
        tag: String,
        /// Face ids, `family:<name>`, or indexed file paths.
        #[arg(required = true)]
        targets: Vec<String>,
    },
    /// Remove a tag from faces.
    Remove {
        tag: String,
        #[arg(required = true)]
        targets: Vec<String>,
    },
    Rename {
        old: String,
        new: String,
    },
    /// Delete a tag from every face.
    Delete {
        tag: String,
    },
}

#[derive(Subcommand)]
enum CollectionCmd {
    /// All collections with their face counts.
    List {
        #[arg(long)]
        json: bool,
    },
    Create {
        name: String,
    },
    Delete {
        name: String,
    },
    Rename {
        old: String,
        new: String,
    },
    /// Append faces to a collection (created if missing).
    Add {
        name: String,
        /// Face ids, `family:<name>`, or indexed file paths.
        #[arg(required = true)]
        targets: Vec<String>,
    },
    Remove {
        name: String,
        #[arg(required = true)]
        targets: Vec<String>,
    },
    /// The faces of a collection, in order.
    Show {
        name: String,
        #[arg(long)]
        json: bool,
    },
    /// Write a collection as JSON (`schemas/collection.json`).
    Export {
        name: String,
        /// Output file; `-` for stdout.
        #[arg(default_value = "-")]
        output: PathBuf,
    },
    /// Read a collection JSON file into this index, matching faces by identity hash,
    /// PostScript name, then path.
    Import {
        /// Input file; `-` for stdin.
        #[arg(default_value = "-")]
        input: PathBuf,
        /// Import under this name instead of the one in the file.
        #[arg(long)]
        name: Option<String>,
        /// Do not apply the tags stored in the file.
        #[arg(long)]
        no_tags: bool,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum SourceCmd {
    List {
        #[arg(long)]
        json: bool,
    },
    /// Register a directory and scan it now.
    Add {
        path: PathBuf,
        /// Register without following it in `watch`.
        #[arg(long)]
        no_watch: bool,
        #[arg(long)]
        json: bool,
    },
    /// Forget a directory; with `--purge`, drop its faces from the index too.
    Remove {
        path: PathBuf,
        #[arg(long)]
        purge: bool,
    },
    /// Turn watching on (default) or off for a source.
    Watch {
        path: PathBuf,
        #[arg(long)]
        off: bool,
    },
}

#[derive(Args)]
struct ListArgs {
    /// Full-text query over family, style, PostScript name and designer.
    query: Option<String>,
    #[command(flatten)]
    filter: FilterArgs,
    #[arg(long, short = 'n')]
    limit: Option<usize>,
    #[arg(long)]
    json: bool,
}

impl ListArgs {
    fn to_filter(&self) -> FaceFilter {
        FaceFilter {
            query: self.query.clone(),
            limit: self.limit,
            ..self.filter.to_filter()
        }
    }
}

/// Filters shared by `list`, `families`, `facets` and `covers`.
#[derive(Args, Clone, Default)]
struct FilterArgs {
    /// Exact family name.
    #[arg(long)]
    family: Option<String>,
    /// Only variable fonts (or `--variable=false` for static only).
    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    variable: Option<bool>,
    /// Only color fonts.
    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    color: Option<bool>,
    /// Only italic/oblique faces.
    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    italic: Option<bool>,
    /// Faces covering this script (ISO 15924, e.g. Arab, Cyrl, Hani).
    #[arg(long)]
    script: Option<String>,
    /// SPDX license prefix, e.g. OFL, Apache, LicenseRef-Proprietary.
    #[arg(long)]
    license: Option<String>,
    /// Only fonts whose license grants the four freedoms. Short for `--freedom free`.
    #[arg(long, conflicts_with_all = ["nonfree", "freedom"])]
    free: bool,
    /// Only fonts whose license withholds one of them. Short for `--freedom nonfree`.
    #[arg(long, conflicts_with = "freedom")]
    nonfree: bool,
    /// free, nonfree, unknown (a license nobody has ruled free) or unstated (none at all).
    #[arg(long, value_name = "STATE", value_parser = parse_freedom)]
    freedom: Option<Freedom>,
    /// Weight range, e.g. 600-900.
    #[arg(long, value_parser = parse_range)]
    weight: Option<(u16, u16)>,
    /// Width range in percent, e.g. 50-87.
    #[arg(long, value_parser = parse_range)]
    width: Option<(u16, u16)>,
    /// `OS/2` vendor id, e.g. GOOG, ADBE.
    #[arg(long)]
    vendor: Option<String>,
    /// Faces carrying this tag.
    #[arg(long)]
    tag: Option<String>,
    /// Faces in this collection.
    #[arg(long)]
    collection: Option<String>,
    /// Only faces activated or installed through fontina (`--active=false` for the rest).
    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    active: Option<bool>,
    /// Only faces in this activation state: session, user or installed.
    #[arg(long, value_parser = parse_state)]
    activation: Option<ActivationState>,
    /// Container: ttf, otf, ttc, woff or woff2.
    #[arg(long)]
    container: Option<String>,
    /// Only faces whose path starts with this prefix.
    #[arg(long)]
    under: Option<String>,
}

impl FilterArgs {
    /// `--free` and `--nonfree` are shorthands for the corresponding `--freedom`; clap
    /// has already rejected any combination of the three.
    fn freedom(&self) -> Option<Freedom> {
        self.freedom
            .or(self.free.then_some(Freedom::Free))
            .or(self.nonfree.then_some(Freedom::Nonfree))
    }

    fn to_filter(&self) -> FaceFilter {
        FaceFilter {
            family: self.family.clone(),
            variable: self.variable,
            color: self.color,
            italic: self.italic,
            script: self.script.clone(),
            license: self.license.clone(),
            freedom: self.freedom(),
            weight: self.weight,
            width: self.width,
            vendor: self.vendor.clone(),
            tag: self.tag.clone(),
            collection: self.collection.clone(),
            active: self.active,
            activation: self.activation,
            container: self.container.clone(),
            path_prefix: self.under.clone(),
            ..Default::default()
        }
    }
}

fn parse_freedom(s: &str) -> std::result::Result<Freedom, String> {
    s.parse()
}

fn parse_state(s: &str) -> std::result::Result<ActivationState, String> {
    s.parse()
        .map_err(|_| format!("unknown state {s:?}; use session, user or installed"))
}

fn parse_range(s: &str) -> std::result::Result<(u16, u16), String> {
    let (a, b) = s.split_once('-').unwrap_or((s, s));
    let lo: u16 = a.trim().parse().map_err(|_| format!("bad weight: {a}"))?;
    let hi: u16 = b.trim().parse().map_err(|_| format!("bad weight: {b}"))?;
    Ok((lo.min(hi), lo.max(hi)))
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

fn open_index(cli: &Cli) -> Result<Index> {
    let path = cli.db.clone().unwrap_or_else(Index::default_path);
    Index::open(&path).with_context(|| format!("opening index at {}", path.display()))
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match &cli.command {
        Command::Scan {
            paths,
            system,
            force,
            follow_symlinks,
            prune,
            json,
        } => {
            let system_roots: Vec<PathBuf> = if *system {
                fontina_platform::system_font_dirs()
                    .into_iter()
                    .map(|d| d.path)
                    .filter(|p| p.exists())
                    .collect()
            } else {
                Vec::new()
            };
            if paths.is_empty() && system_roots.is_empty() {
                bail!("nothing to scan: pass directories or --system");
            }
            let mut index = open_index(&cli)?;
            let opts = ScanOptions {
                force: *force,
                follow_symlinks: *follow_symlinks,
                prune: *prune,
                kind: None,
            };
            let started = std::time::Instant::now();
            let mut report = fontina_core::ScanReport::default();
            if !paths.is_empty() {
                report = fontina_core::scan::scan(&mut index, paths, &opts)?;
            }
            if !system_roots.is_empty() {
                let sys = fontina_core::scan::scan(
                    &mut index,
                    &system_roots,
                    &ScanOptions {
                        kind: Some(SourceKind::System),
                        ..opts.clone()
                    },
                )?;
                report.candidates += sys.candidates;
                report.parsed += sys.parsed;
                report.faces += sys.faces;
                report.unchanged += sys.unchanged;
                report.removed += sys.removed;
                report.failed.extend(sys.failed);
            }
            if *json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!(
                    "scanned {} candidates in {:.2}s: {} parsed ({} faces), {} unchanged, {} removed, {} failed",
                    report.candidates,
                    started.elapsed().as_secs_f64(),
                    report.parsed,
                    report.faces,
                    report.unchanged,
                    report.removed,
                    report.failed.len()
                );
                for f in &report.failed {
                    eprintln!("  ! {}: {}", f.path, f.error);
                }
            }
        }
        Command::List(args) => {
            let index = open_index(&cli)?;
            let faces = index.list(&args.to_filter())?;
            if args.json {
                println!("{}", serde_json::to_string_pretty(&faces)?);
            } else {
                print_table(&faces);
            }
        }
        Command::Families(args) => {
            let index = open_index(&cli)?;
            let families = index.families(&args.to_filter())?;
            if args.json {
                println!("{}", serde_json::to_string_pretty(&families)?);
            } else {
                print_families(&families);
            }
        }
        Command::Facets(args) => {
            let index = open_index(&cli)?;
            let facets = index.facets(&args.to_filter())?;
            if args.json {
                println!("{}", serde_json::to_string_pretty(&facets)?);
            } else {
                print_facets(&facets);
            }
        }
        Command::Tag(cmd) => run_tag(&cli, cmd)?,
        Command::Collection(cmd) => run_collection(&cli, cmd)?,
        Command::Source(cmd) => run_source(&cli, cmd)?,
        Command::Activate {
            targets,
            session,
            replace,
            json,
        } => {
            let state = if *session {
                ActivationState::Session
            } else {
                ActivationState::User
            };
            run_activate(&cli, targets, state, *replace, *json)?
        }
        Command::Install {
            targets,
            replace,
            json,
        } => run_activate(&cli, targets, ActivationState::Installed, *replace, *json)?,
        Command::Deactivate { targets, json } => run_deactivate(&cli, targets, false, *json)?,
        Command::Uninstall { targets, json } => run_deactivate(&cli, targets, true, *json)?,
        Command::Conflicts { targets, json } => {
            let index = open_index(&cli)?;
            let ids = resolve_all_ids(&index, targets)?;
            let conflicts = collect_conflicts(&index, &ids)?;
            if *json {
                println!("{}", serde_json::to_string_pretty(&conflicts)?);
            } else if conflicts.is_empty() {
                println!("no conflicts");
            } else {
                print_conflicts(&conflicts);
                std::process::exit(2);
            }
        }
        Command::Activations { json } => {
            let index = open_index(&cli)?;
            let records = index.activations()?;
            if *json {
                println!("{}", serde_json::to_string_pretty(&records)?);
            } else if records.is_empty() {
                println!("nothing activated or installed through fontina");
            } else {
                for r in &records {
                    println!(
                        "{:<10} [{}] {} {}  {}",
                        r.state.as_str(),
                        r.face.id,
                        r.face.family,
                        r.face.subfamily,
                        r.installed_path.as_deref().unwrap_or(&r.face.path)
                    );
                }
                println!("{} face(s)", records.len());
            }
        }
        Command::Restore { json } => run_restore(&cli, *json)?,
        Command::Agent(cmd) => run_agent(&cli, cmd)?,
        Command::Watch {
            paths,
            debounce_ms,
            json,
        } => {
            let mut index = open_index(&cli)?;
            let mut roots: Vec<PathBuf> = index
                .sources()?
                .into_iter()
                .filter(|s| s.watch && std::path::Path::new(&s.path).is_dir())
                .map(|s| PathBuf::from(s.path))
                .collect();
            roots.extend(paths.iter().cloned());
            if roots.is_empty() {
                bail!(
                    "nothing to watch: add a source (`fontina source add <dir>`) or pass directories"
                );
            }
            if !*json {
                for r in &roots {
                    eprintln!("watching {}", r.display());
                }
            }
            fontina_core::watch::watch(
                &mut index,
                &roots,
                &fontina_core::watch::WatchOptions {
                    debounce: std::time::Duration::from_millis(*debounce_ms),
                    ..Default::default()
                },
                |ev| {
                    if *json {
                        println!("{}", serde_json::to_string(ev).unwrap_or_default());
                    } else {
                        println!(
                            "{} path(s): {} parsed ({} faces), {} unchanged, {} removed, {} failed",
                            ev.paths.len(),
                            ev.report.parsed,
                            ev.report.faces,
                            ev.report.unchanged,
                            ev.report.removed,
                            ev.report.failed.len()
                        );
                        for f in &ev.report.failed {
                            eprintln!("  ! {}: {}", f.path, f.error);
                        }
                    }
                    true
                },
            )?;
        }
        Command::Info { target, json } => {
            let faces = resolve_faces(&cli, target)?;
            if *json {
                println!("{}", serde_json::to_string_pretty(&faces)?);
            } else {
                for f in &faces {
                    print_info(f);
                }
            }
        }
        Command::Dupes { json } => {
            let index = open_index(&cli)?;
            let groups = index.duplicates()?;
            if *json {
                println!("{}", serde_json::to_string_pretty(&groups)?);
            } else if groups.is_empty() {
                println!("no duplicates");
            } else {
                for g in &groups {
                    println!(
                        "{} ({}):",
                        g.reason,
                        g.key.chars().take(16).collect::<String>()
                    );
                    for f in &g.faces {
                        println!(
                            "  [{}] {} {}  {}#{}",
                            f.id, f.family, f.subfamily, f.path, f.index
                        );
                    }
                }
            }
        }
        Command::Css {
            targets,
            url_prefix,
        } => {
            if targets.is_empty() {
                bail!("pass face ids or font file paths");
            }
            for t in targets {
                for face in resolve_faces(&cli, t)? {
                    let url = url_prefix.as_ref().map(|p| {
                        let name = std::path::Path::new(&face.file.path)
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_default();
                        format!("{}{}", p, name)
                    });
                    print!(
                        "{}",
                        fontina_core::css::font_face_rule(&face, url.as_deref())
                    );
                }
            }
        }
        Command::Stats { json } => {
            let index = open_index(&cli)?;
            let stats = index.stats()?;
            if *json {
                println!("{}", serde_json::to_string_pretty(&stats)?);
            } else {
                println!("index:     {}", stats.db_path);
                println!("files:     {}", stats.files);
                println!("faces:     {}", stats.faces);
                println!("families:  {}", stats.families);
                println!("variable:  {}", stats.variable_faces);
                println!("color:     {}", stats.color_faces);
                println!("failed:    {}", stats.failed_files);
                println!("tags:      {}", stats.tags);
                println!("collections: {}", stats.collections);
                println!("sources:   {}", stats.sources);
                println!("active:    {}", stats.activations);
                for (p, e) in index.failures()?.iter().take(20) {
                    println!("  ! {p}: {e}");
                }
            }
        }
        Command::Dirs { json } => {
            let dirs = fontina_platform::system_font_dirs();
            if *json {
                println!("{}", serde_json::to_string_pretty(&dirs)?);
            } else {
                for d in dirs {
                    println!(
                        "{:<60} {}{}",
                        d.path.display(),
                        d.description,
                        if d.user_writable {
                            " (install target)"
                        } else {
                            ""
                        }
                    );
                }
            }
        }
        Command::Check {
            targets,
            strict,
            min,
            json,
        } => {
            if targets.is_empty() {
                bail!("pass face ids or font file paths");
            }
            let min_sev = match min.as_str() {
                "info" => fontina_core::Severity::Info,
                "warn" | "warning" => fontina_core::Severity::Warn,
                "error" => fontina_core::Severity::Error,
                other => bail!("unknown level {other:?}; use info, warn or error"),
            };
            let mut reports = Vec::new();
            for t in targets {
                for face in resolve_faces(&cli, t)? {
                    let mut r = fontina_core::check_face(&face);
                    r.findings.retain(|f| f.severity >= min_sev);
                    reports.push(r);
                }
            }
            let failed = reports.iter().filter(|r| !r.passed(*strict)).count();
            if *json {
                println!("{}", serde_json::to_string_pretty(&reports)?);
            } else {
                for r in &reports {
                    let status = if r.passed(*strict) { "PASS" } else { "FAIL" };
                    println!(
                        "{status}  {} {}  ({}#{})  {} error(s), {} warning(s)",
                        r.family, r.subfamily, r.path, r.index, r.errors, r.warnings
                    );
                    for f in &r.findings {
                        let tag = match f.severity {
                            fontina_core::Severity::Error => "ERROR",
                            fontina_core::Severity::Warn => "WARN ",
                            fontina_core::Severity::Info => "info ",
                        };
                        println!("  {tag} {:<22} {}", f.id, f.message);
                    }
                }
                println!("{} face(s) checked, {} failed", reports.len(), failed);
            }
            if failed > 0 {
                std::process::exit(1);
            }
        }
        Command::Covers {
            text,
            variable,
            under,
            limit,
            json,
        } => {
            let index = open_index(&cli)?;
            let filter = FaceFilter {
                variable: if *variable { Some(true) } else { None },
                path_prefix: under.clone(),
                limit: *limit,
                ..Default::default()
            };
            let faces = index.covering(text, &filter)?;
            if *json {
                println!("{}", serde_json::to_string_pretty(&faces)?);
            } else {
                let n = text
                    .chars()
                    .filter(|c| !c.is_whitespace() && !c.is_control())
                    .collect::<std::collections::BTreeSet<_>>()
                    .len();
                println!("{} distinct character(s)", n);
                print_table(&faces);
            }
        }
        Command::Glyphs {
            target,
            block,
            json,
        } => {
            let faces = resolve_faces(&cli, target)?;
            for face in &faces {
                let blocks = fontina_core::unicode::glyph_map(&face.coverage.ranges);
                if let Some(name) = block {
                    let needle = name.to_ascii_lowercase();
                    let hits: Vec<_> = blocks
                        .iter()
                        .filter(|b| b.block.to_ascii_lowercase().contains(&needle))
                        .collect();
                    if hits.is_empty() {
                        bail!("no covered block matches {name:?}");
                    }
                    if *json {
                        println!("{}", serde_json::to_string_pretty(&hits)?);
                    } else {
                        for b in hits {
                            println!(
                                "{} (U+{:04X}–U+{:04X}): {} of {}",
                                b.block,
                                b.start,
                                b.end,
                                b.codepoints.len(),
                                b.block_size
                            );
                            let chars: Vec<char> = b
                                .codepoints
                                .iter()
                                .map(|&c| fontina_core::unicode::cell_for(c).glyph)
                                .collect();
                            for chunk in chars.chunks(64) {
                                println!("  {}", chunk.iter().collect::<String>());
                            }
                        }
                    }
                } else if *json {
                    println!("{}", serde_json::to_string_pretty(&blocks)?);
                } else {
                    println!(
                        "{} {}: {} codepoints in {} blocks",
                        face.names.family,
                        face.names.subfamily,
                        face.coverage.codepoints,
                        blocks.len()
                    );
                    for b in &blocks {
                        println!(
                            "  {:<44} U+{:04X}–U+{:04X}  {:>5} / {:<5} {:>3}%",
                            b.block,
                            b.start,
                            b.end,
                            b.codepoints.len(),
                            b.block_size,
                            b.codepoints.len() * 100 / b.block_size as usize
                        );
                    }
                }
            }
        }
        Command::License { targets, json } => {
            let faces: Vec<fontina_core::FaceMetadata> = if targets.is_empty() {
                let index = open_index(&cli)?;
                let mut out = Vec::new();
                for s in index.list(&FaceFilter::default())? {
                    if let Some(f) = index.get_face(s.id)? {
                        out.push(f);
                    }
                }
                out
            } else {
                let mut out = Vec::new();
                for t in targets {
                    out.extend(resolve_faces(&cli, t)?);
                }
                out
            };
            let rows: Vec<LicenseRow> = faces
                .iter()
                .map(|f| {
                    let v = fontina_core::freedom::assess(f.license.spdx.as_deref());
                    LicenseRow {
                        family: &f.names.family,
                        subfamily: &f.names.subfamily,
                        path: &f.file.path,
                        spdx: f.license.spdx.as_deref(),
                        freedom: v.freedom,
                        reason: v.reason,
                        embedding: f.os2.as_ref().map(|o| &o.embedding),
                        reserved_font_names: &f.license.reserved_font_names,
                        url: f.license.url.as_deref(),
                        copyright: f.names.copyright.as_deref(),
                    }
                })
                .collect();
            if *json {
                println!("{}", serde_json::to_string_pretty(&rows)?);
            } else {
                let mut by: std::collections::BTreeMap<String, Vec<&LicenseRow>> =
                    Default::default();
                for r in &rows {
                    by.entry(r.spdx.unwrap_or("(none embedded)").to_string())
                        .or_default()
                        .push(r);
                }
                for (spdx, rs) in &by {
                    let v = rs[0];
                    println!(
                        "{spdx}  [{}]  ({} face(s))\n  {}",
                        v.freedom,
                        rs.len(),
                        v.reason
                    );
                    for r in rs {
                        let emb = r
                            .embedding
                            .map(|e| format!("{:?}", e.level))
                            .unwrap_or_else(|| "-".into());
                        let rfn = if r.reserved_font_names.is_empty() {
                            String::new()
                        } else {
                            format!("  RFN: {}", r.reserved_font_names.join(", "))
                        };
                        println!("  {} {}  [{emb}]{rfn}  {}", r.family, r.subfamily, r.path);
                    }
                }
                let mut tally: Vec<String> = Vec::new();
                for f in Freedom::ALL {
                    let n = rows.iter().filter(|r| r.freedom == f).count();
                    if n > 0 {
                        tally.push(format!("{n} {f}"));
                    }
                }
                println!("\n{}", tally.join(", "));
            }
        }
        Command::Specimen {
            targets,
            output,
            text,
            link,
            title,
        } => {
            if targets.is_empty() {
                bail!("pass face ids or font file paths");
            }
            let mut faces = Vec::new();
            for t in targets {
                faces.extend(resolve_faces(&cli, t)?);
            }
            let html = fontina_core::specimen::render(
                &faces,
                &fontina_core::specimen::SpecimenOptions {
                    text: text.clone(),
                    link: *link,
                    title: title.clone(),
                },
            )?;
            if output.as_os_str() == "-" {
                print!("{html}");
            } else {
                std::fs::write(output, &html)
                    .with_context(|| format!("writing {}", output.display()))?;
                eprintln!(
                    "wrote {} ({} faces, {} KB)",
                    output.display(),
                    faces.len(),
                    html.len() / 1024
                );
            }
        }
        Command::Preview(args) => run_preview(&cli, args)?,
        Command::Ui => {
            let path = cli.db.clone().unwrap_or_else(Index::default_path);
            ui::run(&path)?
        }
        Command::Completions { shell } => {
            let mut cmd = Cli::command();
            clap_complete::generate(*shell, &mut cmd, "fontina", &mut std::io::stdout());
        }
        Command::Man { out_dir } => {
            let cmd = Cli::command();
            match out_dir {
                Some(dir) => {
                    std::fs::create_dir_all(dir)
                        .with_context(|| format!("creating {}", dir.display()))?;
                    clap_mangen::generate_to(cmd, dir)
                        .with_context(|| format!("writing man pages to {}", dir.display()))?;
                    eprintln!("wrote man pages to {}", dir.display());
                }
                None => {
                    let mut out = Vec::new();
                    clap_mangen::Man::new(cmd).render(&mut out)?;
                    std::io::stdout().write_all(&out)?;
                }
            }
        }
        Command::Schema { which } => {
            let schema = match which.as_str() {
                "face" => fontina_core::face_schema(),
                "collection" => fontina_core::collection_schema(),
                "cli-output" | "cli_output" | "cli" => fontina_core::cli_output_schema(),
                other => bail!("unknown schema {other:?}; use face, collection or cli-output"),
            };
            println!("{}", serde_json::to_string_pretty(&schema)?);
        }
    }
    Ok(())
}

#[derive(serde::Serialize)]
struct LicenseRow<'a> {
    family: &'a str,
    subfamily: &'a str,
    path: &'a str,
    spdx: Option<&'a str>,
    freedom: Freedom,
    /// Why `freedom` is what it is, in one line.
    reason: &'static str,
    embedding: Option<&'a fontina_core::model::EmbeddingRights>,
    reserved_font_names: &'a [String],
    url: Option<&'a str>,
    copyright: Option<&'a str>,
}

/// The login agent: write it, remove it, or say where it is.
///
/// Everything here stays inside the user's own directories and none of it needs
/// elevation, so installing the agent cannot affect anyone else on the machine.
fn run_agent(cli: &Cli, cmd: &AgentCmd) -> Result<()> {
    use fontina_platform::agent;
    match cmd {
        AgentCmd::Install { json } => {
            // The binary as it was invoked. Deliberately not canonicalised: on Homebrew
            // and Nix the invoked path is a stable symlink and its target is a
            // version-specific store path that the next upgrade deletes, so resolving it
            // would produce an agent that fails at every login after an update.
            let exe = std::env::current_exe()
                .context("cannot find this executable, so no login agent can point at it")?;
            // The index has to travel with it. Without this an agent installed by
            // someone who keeps their index elsewhere restores from the default one,
            // finds nothing, and reports success.
            let mut args = vec!["restore".to_string()];
            if let Some(db) = &cli.db {
                args.push("--db".into());
                args.push(db.display().to_string());
            }
            let plan = agent::install(&exe, &args)?;
            if *json {
                println!(
                    "{}",
                    serde_json::json!({
                        "installed": true,
                        "path": plan.path,
                        "kind": plan.kind,
                        "activate_with": plan.activate_with,
                    })
                );
            } else {
                println!("wrote the {} to {}", plan.kind, plan.path.display());
                match &plan.activate_with {
                    Some(c) => println!("it starts at your next login; to start it now:  {c}"),
                    None => println!("it runs at your next login"),
                }
            }
        }
        AgentCmd::Uninstall { json } => {
            let plan = agent::plan(std::path::Path::new("/fontina"), &[]);
            let removed = agent::uninstall()?;
            if *json {
                println!(
                    "{}",
                    serde_json::json!({
                        "removed": removed,
                        "deactivate_with": plan.as_ref().and_then(|p| p.deactivate_with.clone()),
                    })
                );
            } else if removed {
                println!("removed the login agent");
                // Deleting the file does not undo the enablement: systemd keeps a
                // symlink that then fails at every login, and launchd keeps the job
                // loaded until logout.
                if let Some(c) = plan.as_ref().and_then(|p| p.deactivate_with.as_ref()) {
                    println!("if you enabled it, also run:  {c}");
                }
            } else {
                println!("no login agent was installed");
            }
        }
        AgentCmd::Status { json } => {
            let status = agent::status();
            let plan = agent::plan(std::path::Path::new("/fontina"), &[]);
            if *json {
                println!(
                    "{}",
                    serde_json::json!({
                        "installed": status.as_ref().is_some_and(|s| s.installed),
                        "enabled": status.as_ref().is_some_and(|s| s.enabled),
                        "path": status.as_ref().map(|s| s.path.clone()),
                        "kind": plan.as_ref().map(|p| p.kind),
                    })
                );
            } else {
                match (&status, &plan) {
                    (Some(s), Some(p)) if s.installed && s.enabled => {
                        println!("installed: {} at {}", p.kind, s.path.display())
                    }
                    (Some(s), Some(p)) if s.installed => {
                        // The file exists and the system has not been told to run it,
                        // which is not the same as being installed.
                        println!(
                            "written but not enabled: {} at {}",
                            p.kind,
                            s.path.display()
                        );
                        if let Some(c) = &p.activate_with {
                            println!("enable it with:  {c}");
                        }
                    }
                    (_, Some(p)) => println!(
                        "not installed; `fontina agent install` would write the {} to {}",
                        p.kind,
                        p.path.display()
                    ),
                    _ => println!("no home directory, so no login agent is possible here"),
                }
            }
        }
    }
    Ok(())
}

/// A target is a face id when numeric and not an existing path; otherwise a file path,
/// served from the index when present and parsed directly when not.
fn resolve_faces(cli: &Cli, target: &str) -> Result<Vec<fontina_core::FaceMetadata>> {
    if let Some(family) = target.strip_prefix("family:") {
        let index = open_index(cli)?;
        let ids = resolve_ids(&index, target)?;
        if ids.is_empty() {
            bail!("no indexed family named {family:?}");
        }
        return ids
            .iter()
            .filter_map(|id| index.get_face(*id).transpose())
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into);
    }
    let path = PathBuf::from(target);
    if !path.exists() {
        if let Ok(id) = target.parse::<i64>() {
            let index = open_index(cli)?;
            return match index.get_face(id)? {
                Some(f) => Ok(vec![f]),
                None => bail!("no face with id {id}"),
            };
        }
        bail!("{target}: no such file, and not a face id");
    }
    let canonical = std::fs::canonicalize(&path)?;
    if let Ok(index) = open_index(cli) {
        let cached = index.faces_for_path(&canonical.to_string_lossy())?;
        if !cached.is_empty() {
            return Ok(cached);
        }
    }
    let (_, faces) = fontina_core::load_file(&canonical)?;
    Ok(faces)
}

fn print_table(faces: &[FaceSummary]) {
    if faces.is_empty() {
        println!("no faces match");
        return;
    }
    let w_fam = faces
        .iter()
        .map(|f| f.family.chars().count())
        .max()
        .unwrap_or(6)
        .clamp(6, 40);
    let w_sub = faces
        .iter()
        .map(|f| f.subfamily.chars().count())
        .max()
        .unwrap_or(5)
        .clamp(5, 28);
    let any_tags = faces.iter().any(|f| !f.tags.is_empty());
    // One `println!` per row is one `write` syscall per row: Rust's stdout is line
    // buffered whether or not it is a terminal. Listing five thousand faces spent more
    // time in the kernel than in the query. Lock it once and buffer the whole table.
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());
    let _ = writeln!(
        out,
        "{:>6}  {:<w_fam$}  {:<w_sub$}  {:>4}  {:>4}  {:<5}  {:<12}  path{}",
        "id",
        "family",
        "style",
        "wght",
        "wdth",
        "flags",
        "license",
        if any_tags { "  [tags]" } else { "" }
    );
    for f in faces {
        // The flags column is exactly five characters, so it goes straight into the row
        // rather than through a `format!` and an allocation for every face listed.
        let _ = writeln!(
            out,
            "{:>6}  {:<w_fam$}  {:<w_sub$}  {:>4}  {:>4}  {}{}{}{}{}  {:<12}  {}{}{}",
            f.id,
            truncate(&f.family, w_fam),
            truncate(&f.subfamily, w_sub),
            f.weight.round() as i64,
            f.width.round() as i64,
            if f.variable { "V" } else { "-" },
            if f.color { "C" } else { "-" },
            if f.italic { "I" } else { "-" },
            match f.activation {
                Some(ActivationState::Session) => "s",
                Some(ActivationState::User) => "u",
                Some(ActivationState::Installed) => "i",
                None => "-",
            },
            freedom_flag(f.freedom),
            truncate(f.license.as_deref().unwrap_or("-"), 12),
            f.path,
            if f.index > 0 || f.container == "ttc" {
                format!("#{}", f.index)
            } else {
                String::new()
            },
            if f.tags.is_empty() {
                String::new()
            } else {
                format!("  [{}]", f.tags.join(", "))
            }
        );
    }
    let _ = writeln!(out, "{} face(s)", faces.len());
    // BufWriter swallows a failed flush in its destructor, and a closed pipe is the
    // ordinary way this ends; `die_on_broken_pipe` has already made that a signal.
    let _ = out.flush();
}

/// The fifth character of the `flags` column: `F` free, `N` nonfree, `?` a license
/// nobody has ruled on, `-` no license stated.
fn freedom_flag(f: Freedom) -> &'static str {
    match f {
        Freedom::Free => "F",
        Freedom::Nonfree => "N",
        Freedom::Unknown => "?",
        Freedom::Unstated => "-",
    }
}

/// Shorten to `n` characters, borrowing when it already fits. Listing a large library
/// formats two of these per row, and almost every one of them fits.
fn truncate(s: &str, n: usize) -> std::borrow::Cow<'_, str> {
    if s.chars().count() <= n {
        std::borrow::Cow::Borrowed(s)
    } else {
        std::borrow::Cow::Owned(s.chars().take(n.saturating_sub(1)).collect::<String>() + "…")
    }
}

fn print_info(f: &fontina_core::FaceMetadata) {
    let n = &f.names;
    println!("{} {}", n.family, n.subfamily);
    println!(
        "  file:        {}{}",
        f.file.path,
        if f.file.face_count > 1 {
            format!(" (face {} of {})", f.index, f.file.face_count)
        } else {
            String::new()
        }
    );
    println!(
        "  container:   {}  {} bytes  blake3 {}",
        f.file.container.as_str(),
        f.file.size,
        &f.file.blake3[..16]
    );
    if let Some(p) = &n.postscript_name {
        println!("  postscript:  {p}");
    }
    if let Some(v) = &n.version {
        println!("  version:     {v}");
    }
    if let Some(d) = &n.designer {
        println!("  designer:    {d}");
    }
    if let Some(m) = &n.manufacturer {
        println!("  vendor:      {m}");
    }
    println!(
        "  css:         weight {}; stretch {}; style {}",
        f.style.css.weight, f.style.css.stretch, f.style.css.style
    );
    println!(
        "  metrics:     {} upm, asc {} desc {} gap {}, italic angle {}",
        f.metrics.units_per_em,
        f.metrics.ascender,
        f.metrics.descender,
        f.metrics.line_gap,
        f.metrics.italic_angle
    );
    println!(
        "  outlines:    {:?}{}",
        f.capabilities.outlines,
        if f.capabilities.hinting {
            ", hinted"
        } else {
            ""
        }
    );
    if !f.capabilities.color.is_empty() {
        println!("  color:       {:?}", f.capabilities.color);
    }
    println!(
        "  glyphs:      {}   codepoints: {}",
        f.glyph_count, f.coverage.codepoints
    );
    let scripts: Vec<String> = f
        .coverage
        .scripts
        .iter()
        .take(8)
        .map(|s| format!("{} {}", s.script, s.codepoints))
        .collect();
    println!("  scripts:     {}", scripts.join(", "));
    if let Some(v) = &f.variable {
        println!(
            "  axes:        {}",
            v.axes
                .iter()
                .map(|a| format!("{} {}..{} (default {})", a.tag, a.min, a.max, a.default))
                .collect::<Vec<_>>()
                .join("; ")
        );
        if !v.instances.is_empty() {
            println!(
                "  instances:   {}",
                v.instances
                    .iter()
                    .filter_map(|i| i.name.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }
    if !f.features.gsub.is_empty() {
        println!("  gsub:        {}", f.features.gsub.join(" "));
    }
    if !f.features.gpos.is_empty() {
        println!("  gpos:        {}", f.features.gpos.join(" "));
    }
    if let Some(o) = &f.os2 {
        println!(
            "  embedding:   {:?}{}{}",
            o.embedding.level,
            if o.embedding.no_subsetting {
                ", no subsetting"
            } else {
                ""
            },
            if o.embedding.bitmap_only {
                ", bitmap only"
            } else {
                ""
            }
        );
    }
    println!(
        "  license:     {}{}",
        f.license.spdx.as_deref().unwrap_or("(none embedded)"),
        f.license
            .url
            .as_ref()
            .map(|u| format!("  {u}"))
            .unwrap_or_default()
    );
    println!();
}

/// Face ids for a target that must already be indexed: a numeric id, `family:<name>`, or
/// a file path.
fn resolve_ids(index: &Index, target: &str) -> Result<Vec<i64>> {
    if let Some(family) = target.strip_prefix("family:") {
        let faces = index.list(&FaceFilter {
            family: Some(family.to_string()),
            ..Default::default()
        })?;
        if faces.is_empty() {
            bail!("no indexed family named {family:?}");
        }
        return Ok(faces.into_iter().map(|f| f.id).collect());
    }
    let path = PathBuf::from(target);
    if path.exists() {
        let canonical = std::fs::canonicalize(&path)?;
        let ids = index.ids_for_path(&canonical.to_string_lossy())?;
        if ids.is_empty() {
            bail!("{target} is not indexed; run `fontina scan` on it first");
        }
        return Ok(ids);
    }
    if let Ok(id) = target.parse::<i64>() {
        if index.summaries(&[id])?.is_empty() {
            bail!("no face with id {id}");
        }
        return Ok(vec![id]);
    }
    bail!("{target}: no such file, and not a face id")
}

fn resolve_all_ids(index: &Index, targets: &[String]) -> Result<Vec<i64>> {
    let mut ids = Vec::new();
    for t in targets {
        ids.extend(resolve_ids(index, t)?);
    }
    ids.dedup();
    Ok(ids)
}

fn run_tag(cli: &Cli, cmd: &TagCmd) -> Result<()> {
    let mut index = open_index(cli)?;
    match cmd {
        TagCmd::List { json } => {
            let tags = index.tags()?;
            if *json {
                println!("{}", serde_json::to_string_pretty(&tags)?);
            } else if tags.is_empty() {
                println!("no tags");
            } else {
                for t in tags {
                    println!("{:<30} {:>6}", t.name, t.faces);
                }
            }
        }
        TagCmd::Add { tag, targets } => {
            let ids = resolve_all_ids(&index, targets)?;
            let n = index.tag(&ids, tag)?;
            println!("tagged {n} face(s) with {tag:?}");
        }
        TagCmd::Remove { tag, targets } => {
            let ids = resolve_all_ids(&index, targets)?;
            let n = index.untag(&ids, tag)?;
            println!("removed {tag:?} from {n} face(s)");
        }
        TagCmd::Rename { old, new } => {
            if !index.rename_tag(old, new)? {
                bail!("no tag named {old:?}");
            }
            println!("renamed {old:?} to {new:?}");
        }
        TagCmd::Delete { tag } => {
            if !index.delete_tag(tag)? {
                bail!("no tag named {tag:?}");
            }
            println!("deleted {tag:?}");
        }
    }
    Ok(())
}

fn run_collection(cli: &Cli, cmd: &CollectionCmd) -> Result<()> {
    let mut index = open_index(cli)?;
    match cmd {
        CollectionCmd::List { json } => {
            let cs = index.collections()?;
            if *json {
                println!("{}", serde_json::to_string_pretty(&cs)?);
            } else if cs.is_empty() {
                println!("no collections");
            } else {
                for c in cs {
                    println!("{:<30} {:>6}", c.name, c.faces);
                }
            }
        }
        CollectionCmd::Create { name } => {
            index.create_collection(name)?;
            println!("created {name:?}");
        }
        CollectionCmd::Delete { name } => {
            if !index.delete_collection(name)? {
                bail!("no collection named {name:?}");
            }
            println!("deleted {name:?}");
        }
        CollectionCmd::Rename { old, new } => {
            if !index.rename_collection(old, new)? {
                bail!("no collection named {old:?}");
            }
            println!("renamed {old:?} to {new:?}");
        }
        CollectionCmd::Add { name, targets } => {
            let ids = resolve_all_ids(&index, targets)?;
            let n = index.add_to_collection(name, &ids)?;
            println!("added {n} face(s) to {name:?}");
        }
        CollectionCmd::Remove { name, targets } => {
            let ids = resolve_all_ids(&index, targets)?;
            let n = index.remove_from_collection(name, &ids)?;
            println!("removed {n} face(s) from {name:?}");
        }
        CollectionCmd::Show { name, json } => {
            let faces = index.collection_faces(name)?;
            if *json {
                println!("{}", serde_json::to_string_pretty(&faces)?);
            } else {
                print_table(&faces);
            }
        }
        CollectionCmd::Export { name, output } => {
            let export = index.export_collection(name)?;
            let json = serde_json::to_string_pretty(&export)?;
            if output.as_os_str() == "-" {
                println!("{json}");
            } else {
                std::fs::write(output, json.as_bytes())
                    .with_context(|| format!("writing {}", output.display()))?;
                eprintln!("wrote {} ({} faces)", output.display(), export.faces.len());
            }
        }
        CollectionCmd::Import {
            input,
            name,
            no_tags,
            json,
        } => {
            let text = if input.as_os_str() == "-" {
                std::io::read_to_string(std::io::stdin())?
            } else {
                std::fs::read_to_string(input)
                    .with_context(|| format!("reading {}", input.display()))?
            };
            let export: fontina_core::CollectionExport =
                serde_json::from_str(&text).context("parsing collection JSON")?;
            let report = index.import_collection(&export, name.as_deref(), !*no_tags)?;
            if *json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!(
                    "imported {:?}: {} face(s) matched, {} missing, {} tag(s) applied",
                    report.collection,
                    report.matched,
                    report.missing.len(),
                    report.tags_applied
                );
                for m in &report.missing {
                    eprintln!("  missing: {} {}  {}", m.family, m.subfamily, m.path);
                }
            }
        }
    }
    Ok(())
}

fn run_source(cli: &Cli, cmd: &SourceCmd) -> Result<()> {
    let mut index = open_index(cli)?;
    match cmd {
        SourceCmd::List { json } => {
            let sources = index.sources()?;
            if *json {
                println!("{}", serde_json::to_string_pretty(&sources)?);
            } else if sources.is_empty() {
                println!("no sources; run `fontina scan <dir>` or `fontina source add <dir>`");
            } else {
                for s in sources {
                    println!(
                        "{:<60} {}{}",
                        s.path,
                        match s.kind {
                            SourceKind::User => "user",
                            SourceKind::System => "system",
                        },
                        if s.watch { ", watched" } else { "" }
                    );
                }
            }
        }
        SourceCmd::Add {
            path,
            no_watch,
            json,
        } => {
            if !path.is_dir() {
                bail!("{} is not a directory", path.display());
            }
            let canonical = std::fs::canonicalize(path)?;
            let report = fontina_core::scan::scan(
                &mut index,
                std::slice::from_ref(&canonical),
                &ScanOptions::default(),
            )?;
            let source =
                index.add_source(&canonical.to_string_lossy(), !*no_watch, SourceKind::User)?;
            if *json {
                println!("{}", serde_json::to_string_pretty(&source)?);
            } else {
                println!(
                    "added {}: {} parsed ({} faces), {} unchanged, {} failed{}",
                    source.path,
                    report.parsed,
                    report.faces,
                    report.unchanged,
                    report.failed.len(),
                    if source.watch { ", watched" } else { "" }
                );
            }
        }
        SourceCmd::Remove { path, purge } => {
            let key = std::fs::canonicalize(path)
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|_| path.to_string_lossy().into_owned());
            if !index.remove_source(&key, *purge)? {
                bail!("{key} is not a source");
            }
            println!(
                "removed {key}{}",
                if *purge { " and its faces" } else { "" }
            );
        }
        SourceCmd::Watch { path, off } => {
            let key = std::fs::canonicalize(path)
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|_| path.to_string_lossy().into_owned());
            if !index.set_source_watch(&key, !*off)? {
                bail!("{key} is not a source");
            }
            println!("{key}: watch {}", if *off { "off" } else { "on" });
        }
    }
    Ok(())
}

fn print_families(families: &[fontina_core::Family]) {
    if families.is_empty() {
        println!("no families match");
        return;
    }
    let w = families
        .iter()
        .map(|f| f.name.chars().count())
        .max()
        .unwrap_or(6)
        .clamp(6, 40);
    println!(
        "{:<w$}  {:>5}  {:<9}  {:<9}  {:<5}  {:<12}  scripts",
        "family", "faces", "weights", "widths", "flags", "license"
    );
    for f in families {
        let flags = format!(
            "{}{}{}{}",
            if f.variable { "V" } else { "-" },
            if f.color { "C" } else { "-" },
            if f.italic { "I" } else { "-" },
            if f.active > 0 { "A" } else { "-" }
        );
        let range = |lo: f32, hi: f32| {
            if (lo - hi).abs() < 0.5 {
                format!("{}", lo.round() as i64)
            } else {
                format!("{}-{}", lo.round() as i64, hi.round() as i64)
            }
        };
        println!(
            "{:<w$}  {:>5}  {:<9}  {:<9}  {:<5}  {:<12}  {}",
            truncate(&f.name, w),
            f.faces,
            range(f.weights[0], f.weights[1]),
            range(f.widths[0], f.widths[1]),
            flags,
            truncate(f.license.as_deref().unwrap_or("-"), 12),
            f.scripts
                .iter()
                .take(4)
                .cloned()
                .collect::<Vec<_>>()
                .join(" ")
        );
    }
    println!("{} family(ies)", families.len());
}

fn print_facets(f: &fontina_core::Facets) {
    println!("{} face(s) in {} family(ies)", f.faces, f.families);
    let row =
        |label: &str, items: &[fontina_core::index::FacetCount], name: &dyn Fn(&str) -> String| {
            if items.is_empty() {
                return;
            }
            let parts: Vec<String> = items
                .iter()
                .take(12)
                .map(|c| format!("{} {}", name(&c.value), c.count))
                .collect();
            let more = if items.len() > 12 {
                format!(" · +{} more", items.len() - 12)
            } else {
                String::new()
            };
            println!("{label:<11} {}{more}", parts.join(" · "));
        };
    row("weight", &f.weight, &|v| {
        format!(
            "{v} {}",
            fontina_core::index::weight_name(v.parse().unwrap_or(400))
        )
    });
    row("width", &f.width, &|v| {
        format!(
            "{v}% {}",
            fontina_core::index::width_name(v.parse().unwrap_or(100.0))
        )
    });
    row("style", &f.style, &|v| v.to_string());
    println!("{:<11} {}   color {}", "variable", f.variable, f.color);
    row("container", &f.container, &|v| v.to_string());
    row("script", &f.script, &|v| v.to_string());
    row("license", &f.license, &|v| v.to_string());
    row("freedom", &f.freedom, &|v| v.to_string());
    row("vendor", &f.vendor, &|v| v.to_string());
    row("tag", &f.tag, &|v| v.to_string());
    row("collection", &f.collection, &|v| v.to_string());
    row("activation", &f.activation, &|v| v.to_string());
    row("source", &f.source, &|v| v.to_string());
}

fn system_roots() -> Vec<String> {
    fontina_platform::system_font_dirs()
        .into_iter()
        .map(|d| d.path.to_string_lossy().into_owned())
        .collect()
}

pub(crate) fn collect_conflicts(index: &Index, ids: &[i64]) -> Result<Vec<fontina_core::Conflict>> {
    let roots = system_roots();
    let mut out: Vec<fontina_core::Conflict> = Vec::new();
    for id in ids {
        for c in index.conflicts(*id, &roots)? {
            if !ids.contains(&c.face.id) && !out.iter().any(|o| o.face.id == c.face.id) {
                out.push(c);
            }
        }
    }
    Ok(out)
}

fn print_conflicts(conflicts: &[fontina_core::Conflict]) {
    for c in conflicts {
        eprintln!(
            "conflict: [{}] {} {} ({})  {}",
            c.face.id, c.face.family, c.face.subfamily, c.reason, c.face.path
        );
    }
    eprintln!(
        "{} conflict(s); pass --replace to deactivate the ones fontina manages",
        conflicts.len()
    );
}

/// The distinct files behind a set of face ids, each with every face id in that file.
pub(crate) fn files_for(index: &Index, ids: &[i64]) -> Result<Vec<(PathBuf, Vec<i64>)>> {
    let mut out: Vec<(PathBuf, Vec<i64>)> = Vec::new();
    for s in index.summaries(ids)? {
        let path = PathBuf::from(&s.path);
        if out.iter().any(|(p, _)| *p == path) {
            continue;
        }
        let faces = index.file_faces(s.id)?;
        out.push((path, faces));
    }
    Ok(out)
}

fn run_activate(
    cli: &Cli,
    targets: &[String],
    state: ActivationState,
    replace: bool,
    json: bool,
) -> Result<()> {
    let mut index = open_index(cli)?;
    let ids = resolve_all_ids(&index, targets)?;
    let activator = fontina_platform::activator();
    let conflicts = collect_conflicts(&index, &ids)?;
    if !conflicts.is_empty() {
        if !replace {
            print_conflicts(&conflicts);
            std::process::exit(2);
        }
        for c in &conflicts {
            match c.face.activation {
                Some(ActivationState::Installed) => {
                    let rec = index.activation(c.face.id)?;
                    if let Some(p) = rec.and_then(|r| r.installed_path) {
                        activator.uninstall(std::path::Path::new(&p))?;
                    }
                    let faces = index.file_faces(c.face.id)?;
                    index.clear_activation(&faces)?;
                    eprintln!("uninstalled {} {}", c.face.family, c.face.subfamily);
                }
                Some(_) => {
                    let removed = activator.deactivate(std::path::Path::new(&c.face.path))?;
                    let faces = index.file_faces(c.face.id)?;
                    index.clear_activation(&faces)?;
                    let note = if removed {
                        ""
                    } else {
                        " (nothing was registered; cleared the record)"
                    };
                    eprintln!("deactivated {} {}{note}", c.face.family, c.face.subfamily);
                }
                None => eprintln!(
                    "warning: {} {} is a system font at {}; it cannot be replaced, the OS decides which wins",
                    c.face.family, c.face.subfamily, c.face.path
                ),
            }
        }
    }
    let mut done = Vec::new();
    for (path, faces) in files_for(&index, &ids)? {
        match state {
            ActivationState::Installed => {
                let installed = activator
                    .install(&path)
                    .with_context(|| format!("installing {}", path.display()))?;
                index.set_activation(&faces, state, Some(&installed.to_string_lossy()))?;
            }
            ActivationState::Session | ActivationState::User => {
                let scope = if state == ActivationState::Session {
                    fontina_platform::Scope::Session
                } else {
                    fontina_platform::Scope::User
                };
                activator
                    .activate(&path, scope)
                    .with_context(|| format!("activating {}", path.display()))?;
                index.set_activation(&faces, state, None)?;
            }
        }
        done.extend(faces);
    }
    let records: Vec<_> = index
        .activations()?
        .into_iter()
        .filter(|r| done.contains(&r.face.id))
        .collect();
    if json {
        println!("{}", serde_json::to_string_pretty(&records)?);
    } else {
        for r in &records {
            println!(
                "{} {} {}{}",
                match state {
                    ActivationState::Installed => "installed",
                    _ => "activated",
                },
                r.face.family,
                r.face.subfamily,
                match (&r.installed_path, state) {
                    (Some(p), _) => format!(" -> {p}"),
                    (None, ActivationState::Session) => " (until logout)".into(),
                    _ => String::new(),
                }
            );
        }
    }
    Ok(())
}

fn run_deactivate(cli: &Cli, targets: &[String], uninstall: bool, json: bool) -> Result<()> {
    let mut index = open_index(cli)?;
    let ids = resolve_all_ids(&index, targets)?;
    let activator = fontina_platform::activator();
    let mut done = Vec::new();
    for (path, faces) in files_for(&index, &ids)? {
        let record = index.activation(faces[0])?;
        if uninstall {
            let Some(installed) = record.as_ref().and_then(|r| r.installed_path.clone()) else {
                bail!("{} was not installed by fontina", path.display());
            };
            activator
                .uninstall(std::path::Path::new(&installed))
                .with_context(|| format!("uninstalling {installed}"))?;
        } else if !activator
            .deactivate(&path)
            .with_context(|| format!("deactivating {}", path.display()))?
        {
            // Nothing was registered under that path: the record is stale, so clearing it
            // is all there is to do, and saying so beats reporting a removal that was not.
            eprintln!("{}: nothing was active; cleared the record", path.display());
        }
        index.clear_activation(&faces)?;
        done.push(path);
    }
    if json {
        println!("{}", serde_json::to_string_pretty(&done)?);
    } else {
        for p in &done {
            println!(
                "{} {}",
                if uninstall {
                    "uninstalled"
                } else {
                    "deactivated"
                },
                p.display()
            );
        }
    }
    Ok(())
}

#[derive(Debug, Default, serde::Serialize)]
struct RestoreReport {
    restored: usize,
    reinstalled: usize,
    failed: Vec<(String, String)>,
}

fn run_restore(cli: &Cli, json: bool) -> Result<()> {
    let mut index = open_index(cli)?;
    let activator = fontina_platform::activator();
    let report = restore_activations(&mut index, activator.as_ref())?;
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!(
            "restored {} activation(s), {} reinstalled, {} failed",
            report.restored,
            report.reinstalled,
            report.failed.len()
        );
        for (p, e) in &report.failed {
            eprintln!("  ! {p}: {e}");
        }
    }
    Ok(())
}

/// Reapply every recorded activation with `activator`. A face is either restored or
/// failed, never both, and a reinstall counts only once the index has been told where
/// the new copy went.
fn restore_activations(
    index: &mut Index,
    activator: &dyn fontina_platform::FontActivator,
) -> Result<RestoreReport> {
    let mut report = RestoreReport::default();
    for r in index.activations()? {
        let path = std::path::Path::new(&r.face.path);
        let mut reinstalled = false;
        let result = match r.state {
            ActivationState::Session => activator.activate(path, fontina_platform::Scope::Session),
            ActivationState::User => activator.activate(path, fontina_platform::Scope::User),
            ActivationState::Installed => {
                match r
                    .installed_path
                    .as_deref()
                    .filter(|p| std::path::Path::new(p).exists())
                {
                    Some(_) => Ok(()),
                    None => activator.install(path).and_then(|p| {
                        // A database error here is a failure to record the install, not
                        // an empty face list to write nothing for.
                        let os = |e: fontina_core::Error| {
                            fontina_platform::PlatformError::Os(e.to_string())
                        };
                        let faces = index.file_faces(r.face.id).map_err(os)?;
                        index
                            .set_activation(&faces, r.state, Some(&p.to_string_lossy()))
                            .map_err(os)?;
                        reinstalled = true;
                        Ok(())
                    }),
                }
            }
        };
        match result {
            Ok(()) => {
                report.restored += 1;
                report.reinstalled += usize::from(reinstalled);
            }
            Err(e) => report.failed.push((r.face.path.clone(), e.to_string())),
        }
    }
    Ok(report)
}

/// Which inline-image protocol the terminal speaks, from the environment.
fn detect_protocol() -> &'static str {
    let env = |k: &str| std::env::var(k).unwrap_or_default();
    let term = env("TERM");
    let program = env("TERM_PROGRAM");
    if !std::io::IsTerminal::is_terminal(&std::io::stdout()) {
        return "blocks";
    }
    if term.starts_with("xterm-kitty")
        || !env("KITTY_WINDOW_ID").is_empty()
        || program == "ghostty"
        || term.contains("ghostty")
        || env("KONSOLE_VERSION").parse::<u32>().unwrap_or(0) >= 220400
        || program == "WezTerm"
    {
        return "kitty";
    }
    if program == "iTerm.app" || program == "mintty" || !env("ITERM_SESSION_ID").is_empty() {
        return "iterm";
    }
    if term.starts_with("foot")
        || term == "mlterm"
        || term.contains("sixel")
        || !env("WT_SESSION").is_empty()
    {
        return "sixel";
    }
    "blocks"
}

/// Whether the terminal background is dark, from `COLORFGBG` ("15;0" = light on dark).
fn dark_background() -> bool {
    std::env::var("COLORFGBG")
        .ok()
        .and_then(|v| v.rsplit(';').next()?.parse::<u8>().ok())
        .map(|bg| bg <= 6 || bg == 8)
        .unwrap_or(true)
}

fn run_preview(cli: &Cli, args: &PreviewArgs) -> Result<()> {
    use fontina_core::render::{RenderOptions, encode, render_face};
    let mut faces = Vec::new();
    for t in &args.targets {
        faces.extend(resolve_faces(cli, t)?);
    }
    let protocol = if args.output.is_some() {
        "png"
    } else if args.protocol == "auto" {
        detect_protocol()
    } else {
        args.protocol.as_str()
    };
    if protocol == "png" && args.output.is_none() {
        bail!("--protocol png needs --output <file.png>");
    }
    if args.output.is_some() && faces.len() != 1 {
        bail!("--output writes one face; got {}", faces.len());
    }
    let dark = dark_background();
    let fg = match &args.fg {
        Some(s) => encode::parse_rgb(s).with_context(|| format!("bad colour {s:?}"))?,
        None if dark => [235, 235, 235],
        None => [20, 20, 20],
    };
    let bg = match &args.bg {
        Some(s) => encode::parse_rgb(s).with_context(|| format!("bad colour {s:?}"))?,
        None if dark => [0, 0, 0],
        None => [255, 255, 255],
    };
    let tmux = std::env::var_os("TMUX").is_some();
    let mut out = std::io::stdout().lock();
    for face in &faces {
        let text = args
            .text
            .clone()
            .or_else(|| face.names.sample_text.clone())
            .unwrap_or_else(|| fontina_core::typography::DEFAULT_TEXT.into())
            .replace("\\n", "\n");
        let size = if protocol == "blocks" && args.size == 48.0 {
            24.0
        } else {
            args.size
        };
        let bitmap = render_face(
            face,
            &RenderOptions {
                text,
                size,
                variations: args.axes.clone(),
                features: args.features.clone(),
                padding: 2,
                max_width: args.max_width.or_else(|| {
                    (protocol == "blocks").then(|| terminal_columns().saturating_sub(1))
                }),
            },
        )
        .with_context(|| format!("rendering {}", face.file.path))?;
        if protocol == "png" {
            let path = args.output.as_ref().expect("checked");
            std::fs::write(path, encode::png(&bitmap, fg, args.bg.as_ref().map(|_| bg)))
                .with_context(|| format!("writing {}", path.display()))?;
            eprintln!(
                "wrote {} ({}x{}, {} glyphs)",
                path.display(),
                bitmap.width,
                bitmap.height,
                bitmap.glyphs
            );
            continue;
        }
        writeln!(
            out,
            "{} {}  ({} {}px{})",
            face.names.family,
            face.names.subfamily,
            face.file.container.as_str(),
            size as u32,
            if bitmap.missing > 0 {
                format!(", {} glyph(s) missing", bitmap.missing)
            } else {
                String::new()
            }
        )?;
        let rendered = match protocol {
            "kitty" => encode::kitty(&encode::png(&bitmap, fg, None), tmux),
            "iterm" => encode::iterm(&encode::png(&bitmap, fg, None), tmux),
            "sixel" => {
                let mut s = encode::sixel(&bitmap, fg, bg, 16);
                s.push('\n');
                s
            }
            "blocks" => encode::half_blocks(&bitmap, fg, bg),
            other => {
                bail!("unknown protocol {other:?}; use auto, kitty, iterm, sixel, blocks or png")
            }
        };
        out.write_all(rendered.as_bytes())?;
    }
    Ok(())
}

fn terminal_columns() -> u32 {
    std::env::var("COLUMNS")
        .ok()
        .and_then(|c| c.parse().ok())
        .or_else(|| {
            // SAFETY: TIOCGWINSZ fills a plain struct; a failure leaves it untouched.
            #[cfg(unix)]
            unsafe {
                let mut ws: [u16; 4] = [0; 4];
                if libc_ioctl_winsize(ws.as_mut_ptr()) == 0 && ws[1] > 0 {
                    return Some(ws[1] as u32);
                }
            }
            None
        })
        .unwrap_or(80)
}

#[cfg(unix)]
unsafe fn libc_ioctl_winsize(ws: *mut u16) -> i32 {
    unsafe extern "C" {
        fn ioctl(fd: i32, request: u64, ...) -> i32;
    }
    #[cfg(any(target_os = "macos", target_os = "freebsd", target_os = "openbsd"))]
    const TIOCGWINSZ: u64 = 0x4008_7468;
    #[cfg(not(any(target_os = "macos", target_os = "freebsd", target_os = "openbsd")))]
    const TIOCGWINSZ: u64 = 0x5413;
    unsafe { ioctl(1, TIOCGWINSZ, ws) }
}

/// A stable key for a parsed face, for caches: the file's hash and the face index.
pub(crate) fn face_key(face: &fontina_core::FaceMetadata) -> i64 {
    let h = &face.file.blake3;
    let n = i64::from_str_radix(&h[..15.min(h.len())], 16).unwrap_or(0);
    n.wrapping_mul(31).wrapping_add(face.index as i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fontina_platform::{FontActivator, Scope};
    use std::path::Path;

    /// An activator that installs without touching the machine, and can break the index
    /// while it does so — the one way, from outside the core, to make the write that
    /// records an install fail.
    struct Fake {
        db: PathBuf,
        /// Table to drop from a second connection while `install` runs.
        breaks: Option<&'static str>,
    }

    impl FontActivator for Fake {
        fn install(&self, file: &Path) -> fontina_platform::Result<PathBuf> {
            if let Some(table) = self.breaks {
                rusqlite::Connection::open(&self.db)
                    .and_then(|c| c.execute_batch(&format!("DROP TABLE {table}")))
                    .expect("second connection");
            }
            Ok(file.with_extension("installed"))
        }
        fn uninstall(&self, _installed: &Path) -> fontina_platform::Result<()> {
            Ok(())
        }
        fn activate(&self, _file: &Path, _scope: Scope) -> fontina_platform::Result<()> {
            Ok(())
        }
        fn deactivate(&self, _file: &Path) -> fontina_platform::Result<bool> {
            Ok(true)
        }
    }

    /// An index over one fixture with its faces recorded as installed, but with no
    /// installed path, so `restore` has to install them again.
    fn installed_index(name: &str) -> (PathBuf, Index) {
        let dir =
            std::env::temp_dir().join(format!("fontina-restore-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("index.db");
        let mut index = Index::open(&db).unwrap();
        let font = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/Amiri-Regular.ttf");
        fontina_core::scan::scan(&mut index, &[font], &ScanOptions::default()).unwrap();
        let ids: Vec<i64> = index
            .list(&FaceFilter::default())
            .unwrap()
            .iter()
            .map(|f| f.id)
            .collect();
        assert_eq!(ids.len(), 1);
        index
            .set_activation(&ids, ActivationState::Installed, None)
            .unwrap();
        (db, index)
    }

    #[test]
    fn restore_counts_a_reinstall_once_it_is_recorded() {
        let (db, mut index) = installed_index("ok");
        let report = restore_activations(&mut index, &Fake { db, breaks: None }).unwrap();
        assert_eq!(report.restored, 1);
        assert_eq!(report.reinstalled, 1);
        assert!(report.failed.is_empty(), "{:?}", report.failed);
        assert!(index.activations().unwrap()[0].installed_path.is_some());
    }

    #[test]
    fn restore_does_not_count_a_reinstall_it_could_not_record() {
        // The install succeeds, then writing where the copy went fails.
        let (db, mut index) = installed_index("write");
        let report = restore_activations(
            &mut index,
            &Fake {
                db,
                breaks: Some("activations"),
            },
        )
        .unwrap();
        assert_eq!(report.failed.len(), 1, "{report:?}");
        assert_eq!(report.restored, 0, "{report:?}");
        assert_eq!(
            report.reinstalled, 0,
            "a face counted as reinstalled and failed: {report:?}"
        );
    }

    #[test]
    fn restore_surfaces_a_database_error_when_looking_up_the_faces() {
        // Reading the file's faces fails: that is a failure, not an empty face list to
        // silently write nothing for.
        let (db, mut index) = installed_index("read");
        let report = restore_activations(
            &mut index,
            &Fake {
                db,
                breaks: Some("faces"),
            },
        )
        .unwrap();
        assert_eq!(report.failed.len(), 1, "{report:?}");
        assert_eq!(report.restored, 0, "{report:?}");
        assert_eq!(report.reinstalled, 0, "{report:?}");
    }
}
