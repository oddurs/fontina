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

//! The configuration file: defaults for the things a person would otherwise retype.
//!
//! One TOML file, read once at startup, holding *only* defaults. Every value in it can
//! be overridden by the flag that sets the same thing, so nothing here can make a
//! command do something its arguments do not say. That is the whole design: a file that
//! can only change what happens when you leave an option out is a file you can read
//! someone else's copy of and still predict what their commands do.
//!
//! Precedence, highest first: the flag, then the environment, then this file, then the
//! built-in default. `fontina config` prints every setting with which of those it came
//! from, because a setting whose origin you cannot see is worse than no setting.
//!
//! Absent is not an error: with no file at all, fontina behaves exactly as it did before
//! there was one. A file that exists and does not parse *is* an error, naming the line,
//! and so is a key nobody recognises — a typo that is silently ignored is a setting that
//! silently does nothing.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Where a value came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    /// Compiled in.
    Default,
    /// The configuration file.
    File,
    /// An environment variable.
    Environment,
    /// An argument on this command line.
    Flag,
}

impl Source {
    pub fn label(self) -> &'static str {
        match self {
            Source::Default => "default",
            Source::File => "config",
            Source::Environment => "environment",
            Source::Flag => "flag",
        }
    }
}

/// The file, as written. Every field is optional: a config file says only what it wants
/// to change.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub index: IndexConfig,
    #[serde(default)]
    pub scan: ScanConfig,
    #[serde(default)]
    pub preview: PreviewConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IndexConfig {
    /// Where the index lives. `--db` and `FONTINA_DB` both win over this.
    pub db: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScanConfig {
    /// Directories `fontina scan` walks when it is given none.
    pub sources: Option<Vec<String>>,
    /// Whether a bare `fontina scan` also walks the operating system's font directories.
    pub system: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewConfig {
    /// Sample text, when `--text` is not given.
    pub text: Option<String>,
    /// Size in pixels.
    pub size: Option<f32>,
    /// `auto`, `kitty`, `iterm`, `sixel`, `blocks` or `png`.
    pub protocol: Option<String>,
    /// Ink colour, `#rrggbb`.
    pub fg: Option<String>,
    /// Background colour, `#rrggbb`.
    pub bg: Option<String>,
}

/// The file, and where it was looked for.
pub struct Loaded {
    pub config: Config,
    pub path: PathBuf,
    /// Whether that path exists. A missing file is not an error.
    pub found: bool,
}

/// `$FONTINA_CONFIG`, or `config.toml` in the platform configuration directory.
pub fn path() -> PathBuf {
    if let Some(p) = std::env::var_os("FONTINA_CONFIG") {
        return PathBuf::from(p);
    }
    directories::ProjectDirs::from("", "", "fontina")
        .map(|d| d.config_dir().join("config.toml"))
        .unwrap_or_else(|| PathBuf::from("fontina.toml"))
}

/// Read the file, if there is one.
pub fn load() -> Result<Loaded> {
    let path = path();
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Loaded {
                config: Config::default(),
                path,
                found: false,
            });
        }
        Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
    };
    let config: Config =
        toml::from_str(&text).with_context(|| format!("reading {}", path.display()))?;
    Ok(Loaded {
        config,
        path,
        found: true,
    })
}

/// Expand a leading `~/`, so a config file can say `~/Fonts` and mean it.
pub fn expand(p: &str) -> PathBuf {
    if let Some(rest) = p.strip_prefix("~/")
        && let Some(home) = directories::BaseDirs::new().map(|b| b.home_dir().to_path_buf())
    {
        return home.join(rest);
    }
    PathBuf::from(p)
}

/// One resolved setting, for `fontina config`.
#[derive(serde::Serialize)]
pub struct Setting {
    pub key: &'static str,
    pub value: String,
    pub source: Source,
}

fn setting(key: &'static str, from_file: Option<String>, default: String) -> Setting {
    match from_file {
        Some(v) => Setting {
            key,
            value: v,
            source: Source::File,
        },
        None => Setting {
            key,
            value: default,
            source: Source::Default,
        },
    }
}

impl Config {
    /// Every setting, its value and where it came from. `db_from_cli` is what `--db`
    /// resolved to, flag or environment, which outranks the file.
    pub fn settings(&self, db_from_cli: Option<&Path>) -> Vec<Setting> {
        let db = match (db_from_cli, &self.index.db) {
            (Some(p), _) => Setting {
                key: "index.db",
                value: p.display().to_string(),
                // clap fills `--db` from FONTINA_DB as well, so which of the two it was
                // is only knowable by asking the environment directly.
                source: match std::env::var_os("FONTINA_DB") {
                    Some(v) if Path::new(&v) == p => Source::Environment,
                    _ => Source::Flag,
                },
            },
            (None, Some(p)) => Setting {
                key: "index.db",
                value: expand(p).display().to_string(),
                source: Source::File,
            },
            (None, None) => Setting {
                key: "index.db",
                value: fontina_core::Index::default_path().display().to_string(),
                source: Source::Default,
            },
        };
        vec![
            db,
            setting(
                "scan.sources",
                self.scan
                    .sources
                    .as_ref()
                    .map(|s| s.join(", "))
                    .filter(|s| !s.is_empty()),
                "(none: `scan` needs a path)".into(),
            ),
            setting(
                "scan.system",
                self.scan.system.map(|b| b.to_string()),
                "false".into(),
            ),
            setting(
                "preview.text",
                self.preview.text.clone(),
                "(the face's own sample text, or a pangram)".into(),
            ),
            setting(
                "preview.size",
                self.preview.size.map(|s| s.to_string()),
                "48".into(),
            ),
            setting(
                "preview.protocol",
                self.preview.protocol.clone(),
                "auto".into(),
            ),
            setting(
                "preview.fg",
                self.preview.fg.clone(),
                "(the terminal's foreground)".into(),
            ),
            setting(
                "preview.bg",
                self.preview.bg.clone(),
                "(the terminal's background)".into(),
            ),
        ]
    }
}

/// A commented file holding every setting at its default, for `fontina config --example`.
///
/// It is checked by a test to parse as a configuration, so what this prints can always be
/// saved and used.
pub const EXAMPLE: &str = r##"# fontina configuration.
#
# Save as the path `fontina config --path` prints. Every setting here is a *default*:
# the flag that sets the same thing always wins, so nothing in this file can make a
# command do something its arguments do not say.
#
# Delete any line to go back to fontina's own default. An unknown key is an error
# rather than something quietly ignored, so a typo tells you.

[index]
# Where the index lives. `--db` and FONTINA_DB both win over this.
# db = "~/.local/share/fontina/index.db"

[scan]
# Directories `fontina scan` walks when you give it none.
# sources = ["~/Fonts", "~/Library/Fonts"]
# Whether a bare `fontina scan` also walks the operating system's font directories.
# system = false

[preview]
# Sample text for `fontina preview` and the browser.
# text = "Sphinx of black quartz, judge my vow"
# Size in pixels.
# size = 48
# auto, kitty, iterm, sixel, blocks or png.
# protocol = "auto"
# Ink and background, #rrggbb. Left out, fontina uses the terminal's own colours.
# fg = "#f8f8f2"
# bg = "#282a36"
"##;

#[cfg(test)]
mod tests {
    use super::*;

    /// What `fontina config --example` prints can be saved and used.
    ///
    /// It is a commented file, so this also proves that every key it names is a key the
    /// parser accepts: uncommenting a line cannot produce an unknown field.
    #[test]
    fn the_example_file_is_a_configuration() {
        let parsed: Config = toml::from_str(EXAMPLE).expect("the example parses");
        assert!(parsed.preview.text.is_none(), "every line is commented out");

        // Uncomment every setting — a commented line holding an `=` — and it still
        // parses, which is what proves the keys it offers are keys that exist. The prose
        // comments around them stay comments.
        let live: String = EXAMPLE
            .lines()
            .map(|l| match l.strip_prefix("# ") {
                Some(rest) if rest.contains(" = ") => rest,
                _ => l,
            })
            .collect::<Vec<_>>()
            .join("\n");
        let parsed: Config = toml::from_str(&live)
            .unwrap_or_else(|e| panic!("the example's own settings must parse: {e}\n{live}"));
        assert!(parsed.preview.text.is_some(), "preview.text is a real key");
        assert!(parsed.scan.sources.is_some(), "scan.sources is a real key");
        assert!(parsed.index.db.is_some(), "index.db is a real key");
    }

    #[test]
    fn a_key_nobody_recognises_is_an_error_that_names_it() {
        // Silently ignoring a typo is a setting that silently does nothing.
        let err = toml::from_str::<Config>("[preview]\ntxet = \"typo\"\n")
            .expect_err("an unknown key is refused");
        let msg = err.to_string();
        assert!(msg.contains("txet"), "the typo is named: {msg}");
        assert!(msg.contains("text"), "and what was meant is offered: {msg}");
    }

    #[test]
    fn an_empty_file_is_every_default() {
        let parsed: Config = toml::from_str("").expect("an empty file is a valid one");
        let settings = parsed.settings(None);
        assert!(
            settings.iter().all(|s| s.source == Source::Default),
            "nothing is set, so everything is a default"
        );
        assert!(settings.iter().any(|s| s.key == "preview.size"));
    }

    #[test]
    fn a_setting_in_the_file_says_so_and_a_flag_outranks_it() {
        let parsed: Config = toml::from_str("[index]\ndb = \"/tmp/from-file.db\"\n").unwrap();
        let from_file = parsed.settings(None);
        let db = from_file.iter().find(|s| s.key == "index.db").unwrap();
        assert_eq!(db.source, Source::File);
        assert_eq!(db.value, "/tmp/from-file.db");

        let overridden = parsed.settings(Some(Path::new("/tmp/from-flag.db")));
        let db = overridden.iter().find(|s| s.key == "index.db").unwrap();
        assert_eq!(db.value, "/tmp/from-flag.db", "the flag wins");
    }

    #[test]
    fn a_leading_tilde_is_the_home_directory() {
        let home = directories::BaseDirs::new()
            .unwrap()
            .home_dir()
            .to_path_buf();
        assert_eq!(expand("~/Fonts"), home.join("Fonts"));
        assert_eq!(expand("/etc/fonts"), Path::new("/etc/fonts"));
        // Only a leading `~/`: a directory really called `~` stays itself.
        assert_eq!(expand("fonts/~/x"), Path::new("fonts/~/x"));
    }
}
