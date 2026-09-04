use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;
use unifont_core::{ActivationState, FaceFilter, FaceSummary, Index, ScanOptions, SourceKind};

/// unifont: a lightweight, standards-based font manager.
#[derive(Parser)]
#[command(name = "unifont", version, about, propagate_version = true)]
struct Cli {
    /// Path to the index database (default: the platform data directory).
    #[arg(long, global = true, env = "UNIFONT_DB")]
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
    /// Print a JSON Schema: `face` (default), `collection`, or `cli-output`.
    Schema {
        #[arg(default_value = "face")]
        which: String,
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
    /// Only faces activated or installed through unifont (`--active=false` for the rest).
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
    fn to_filter(&self) -> FaceFilter {
        FaceFilter {
            family: self.family.clone(),
            variable: self.variable,
            color: self.color,
            italic: self.italic,
            script: self.script.clone(),
            license: self.license.clone(),
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
                unifont_platform::system_font_dirs()
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
            let mut report = unifont_core::ScanReport::default();
            if !paths.is_empty() {
                report = unifont_core::scan::scan(&mut index, paths, &opts)?;
            }
            if !system_roots.is_empty() {
                let sys = unifont_core::scan::scan(
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
                        unifont_core::css::font_face_rule(&face, url.as_deref())
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
            let dirs = unifont_platform::system_font_dirs();
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
                "info" => unifont_core::Severity::Info,
                "warn" | "warning" => unifont_core::Severity::Warn,
                "error" => unifont_core::Severity::Error,
                other => bail!("unknown level {other:?}; use info, warn or error"),
            };
            let mut reports = Vec::new();
            for t in targets {
                for face in resolve_faces(&cli, t)? {
                    let mut r = unifont_core::check_face(&face);
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
                            unifont_core::Severity::Error => "ERROR",
                            unifont_core::Severity::Warn => "WARN ",
                            unifont_core::Severity::Info => "info ",
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
                let blocks = unifont_core::unicode::glyph_map(&face.coverage.ranges);
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
                                .filter_map(|&c| char::from_u32(c))
                                .map(|c| if c.is_control() { '\u{FFFD}' } else { c })
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
            let faces: Vec<unifont_core::FaceMetadata> = if targets.is_empty() {
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
                .map(|f| LicenseRow {
                    family: &f.names.family,
                    subfamily: &f.names.subfamily,
                    path: &f.file.path,
                    spdx: f.license.spdx.as_deref(),
                    embedding: f.os2.as_ref().map(|o| &o.embedding),
                    reserved_font_names: &f.license.reserved_font_names,
                    url: f.license.url.as_deref(),
                    copyright: f.names.copyright.as_deref(),
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
                    println!("{spdx}  ({} face(s))", rs.len());
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
            let html = unifont_core::specimen::render(
                &faces,
                &unifont_core::specimen::SpecimenOptions {
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
        Command::Schema { which } => {
            let schema = match which.as_str() {
                "face" => unifont_core::face_schema(),
                "collection" => unifont_core::collection_schema(),
                "cli-output" | "cli_output" | "cli" => unifont_core::cli_output_schema(),
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
    embedding: Option<&'a unifont_core::model::EmbeddingRights>,
    reserved_font_names: &'a [String],
    url: Option<&'a str>,
    copyright: Option<&'a str>,
}

/// A target is a face id when numeric and not an existing path; otherwise a file path,
/// served from the index when present and parsed directly when not.
fn resolve_faces(cli: &Cli, target: &str) -> Result<Vec<unifont_core::FaceMetadata>> {
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
    let (_, faces) = unifont_core::load_file(&canonical)?;
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
    println!(
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
        let flags = format!(
            "{}{}{}{}",
            if f.variable { "V" } else { "-" },
            if f.color { "C" } else { "-" },
            if f.italic { "I" } else { "-" },
            match f.activation {
                Some(ActivationState::Session) => "s",
                Some(ActivationState::User) => "u",
                Some(ActivationState::Installed) => "i",
                None => "-",
            }
        );
        println!(
            "{:>6}  {:<w_fam$}  {:<w_sub$}  {:>4}  {:>4}  {:<5}  {:<12}  {}{}{}",
            f.id,
            truncate(&f.family, w_fam),
            truncate(&f.subfamily, w_sub),
            f.weight.round() as i64,
            f.width.round() as i64,
            flags,
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
    println!("{} face(s)", faces.len());
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        s.chars().take(n.saturating_sub(1)).collect::<String>() + "…"
    }
}

fn print_info(f: &unifont_core::FaceMetadata) {
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
            bail!("{target} is not indexed; run `unifont scan` on it first");
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
            let export: unifont_core::CollectionExport =
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
                println!("no sources; run `unifont scan <dir>` or `unifont source add <dir>`");
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
            let report = unifont_core::scan::scan(
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

fn print_families(families: &[unifont_core::Family]) {
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

fn print_facets(f: &unifont_core::Facets) {
    println!("{} face(s) in {} family(ies)", f.faces, f.families);
    let row =
        |label: &str, items: &[unifont_core::index::FacetCount], name: &dyn Fn(&str) -> String| {
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
            unifont_core::index::weight_name(v.parse().unwrap_or(400))
        )
    });
    row("width", &f.width, &|v| {
        format!(
            "{v}% {}",
            unifont_core::index::width_name(v.parse().unwrap_or(100.0))
        )
    });
    row("style", &f.style, &|v| v.to_string());
    println!("{:<11} {}   color {}", "variable", f.variable, f.color);
    row("container", &f.container, &|v| v.to_string());
    row("script", &f.script, &|v| v.to_string());
    row("license", &f.license, &|v| v.to_string());
    row("vendor", &f.vendor, &|v| v.to_string());
    row("tag", &f.tag, &|v| v.to_string());
    row("collection", &f.collection, &|v| v.to_string());
    row("activation", &f.activation, &|v| v.to_string());
    row("source", &f.source, &|v| v.to_string());
}
