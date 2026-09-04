use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;
use unifont_core::{FaceFilter, FaceSummary, Index, ScanOptions};

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
    /// Print the JSON Schema for face metadata.
    Schema,
}

#[derive(Args)]
struct ListArgs {
    /// Full-text query over family, style, PostScript name and designer.
    query: Option<String>,
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
    /// Only faces whose path starts with this prefix.
    #[arg(long)]
    under: Option<String>,
    #[arg(long, short = 'n')]
    limit: Option<usize>,
    #[arg(long)]
    json: bool,
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
            let mut roots = paths.clone();
            if *system {
                roots.extend(
                    unifont_platform::system_font_dirs()
                        .into_iter()
                        .map(|d| d.path)
                        .filter(|p| p.exists()),
                );
            }
            if roots.is_empty() {
                bail!("nothing to scan: pass directories or --system");
            }
            let mut index = open_index(&cli)?;
            let opts = ScanOptions {
                force: *force,
                follow_symlinks: *follow_symlinks,
                prune: *prune,
            };
            let started = std::time::Instant::now();
            let report = unifont_core::scan::scan(&mut index, &roots, &opts)?;
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
            let filter = FaceFilter {
                query: args.query.clone(),
                family: args.family.clone(),
                variable: args.variable,
                color: args.color,
                italic: args.italic,
                script: args.script.clone(),
                license: args.license.clone(),
                weight: args.weight,
                path_prefix: args.under.clone(),
                limit: args.limit,
            };
            let faces = index.list(&filter)?;
            if args.json {
                println!("{}", serde_json::to_string_pretty(&faces)?);
            } else {
                print_table(&faces);
            }
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
        Command::Schema => {
            println!(
                "{}",
                serde_json::to_string_pretty(&unifont_core::face_schema())?
            );
        }
    }
    Ok(())
}

/// A target is a face id when numeric and not an existing path; otherwise a file path,
/// served from the index when present and parsed directly when not.
fn resolve_faces(cli: &Cli, target: &str) -> Result<Vec<unifont_core::FaceMetadata>> {
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
    println!(
        "{:>6}  {:<w_fam$}  {:<w_sub$}  {:>4}  {:>4}  {:<5}  {:<12}  path",
        "id", "family", "style", "wght", "wdth", "flags", "license"
    );
    for f in faces {
        let flags = format!(
            "{}{}{}",
            if f.variable { "V" } else { "-" },
            if f.color { "C" } else { "-" },
            if f.italic { "I" } else { "-" }
        );
        println!(
            "{:>6}  {:<w_fam$}  {:<w_sub$}  {:>4}  {:>4}  {:<5}  {:<12}  {}{}",
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
