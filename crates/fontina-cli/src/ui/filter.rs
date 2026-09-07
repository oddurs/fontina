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

//! A filter, written the way the command line writes it.
//!
//! The facet pane shows what the library contains and lets one value be chosen from
//! each heading. Everything else the command line can express — a weight range, two
//! scripts at once, a licence and a coverage threshold together, "not variable" — was
//! out of reach, and the only way to find out how many faces a filter would leave was
//! to run it in another window.
//!
//! So the filter bar takes the flags `fontina list` takes, and it takes them by handing
//! them to `fontina list`'s own parser. Nothing here lists the fields: `ListArgs` is the
//! command line's, `to_filter` is the command line's, and a flag added to one is a flag
//! the browser understands the same day. That is also the answer to "every `FaceFilter`
//! field is reachable" — not a checklist somebody keeps, but the same code path.
//!
//! What is written here is the tokeniser, because a line typed into a prompt is not
//! `argv` and something has to make it one.

use fontina_core::FaceFilter;

/// The flags of `fontina list`, with no program name in front of them.
#[derive(clap::Parser)]
#[command(
    name = "filter",
    no_binary_name = true,
    disable_help_flag = true,
    disable_version_flag = true
)]
struct Line {
    #[command(flatten)]
    args: crate::ListArgs,
}

/// Turn a typed line into a filter, or say what is wrong with it.
///
/// The error is clap's own first line, which is the sentence the command line would
/// have printed. A reader who mistypes `--wieght` should meet the same words in both
/// places.
pub fn parse(line: &str) -> Result<FaceFilter, String> {
    use clap::Parser;
    let tokens = tokenise(line)?;
    match Line::try_parse_from(tokens) {
        Ok(parsed) => Ok(parsed.args.to_filter()),
        Err(e) => Err(first_line(&e.to_string())),
    }
}

/// clap renders an error as a paragraph with usage and a hint. A status line is a line.
fn first_line(message: &str) -> String {
    message
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("cannot parse that")
        .trim_start_matches("error: ")
        .to_string()
}

/// Split a typed line into arguments, honouring quotes.
///
/// A family name has spaces in it — `--family "Source Serif 4"` — so splitting on
/// whitespace would turn one argument into four and the error would be about `Serif`.
/// Single and double quotes both work and neither nests, which is as much shell as a
/// filter needs; a backslash escapes the next character inside double quotes or outside
/// them, and means nothing inside single quotes, which is the rule every shell agrees
/// on.
fn tokenise(line: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let mut word = String::new();
    let mut started = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            c if c.is_whitespace() => {
                if started {
                    out.push(std::mem::take(&mut word));
                    started = false;
                }
            }
            '\'' => {
                started = true;
                loop {
                    match chars.next() {
                        Some('\'') => break,
                        Some(c) => word.push(c),
                        None => return Err("unclosed '".into()),
                    }
                }
            }
            '"' => {
                started = true;
                loop {
                    match chars.next() {
                        Some('"') => break,
                        Some('\\') => match chars.next() {
                            Some(c) => word.push(c),
                            None => return Err("unclosed \"".into()),
                        },
                        Some(c) => word.push(c),
                        None => return Err("unclosed \"".into()),
                    }
                }
            }
            '\\' => {
                started = true;
                match chars.next() {
                    Some(c) => word.push(c),
                    None => return Err("a line cannot end in a backslash".into()),
                }
            }
            c => {
                started = true;
                word.push(c);
            }
        }
    }
    if started {
        out.push(word);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fontina_core::Freedom;

    #[test]
    fn a_line_of_flags_is_the_filter_the_command_line_would_build() {
        let f = parse("--weight 300-500 --script Arab --script Latn --free").unwrap();
        assert_eq!(f.weight, Some((300, 500)));
        assert_eq!(f.scripts, ["Arab", "Latn"], "both, not either");
        assert_eq!(f.freedom, Some(Freedom::Free));
    }

    /// The thing the facet pane cannot say: a range, a threshold and a negation
    /// together, and a text query alongside them.
    #[test]
    fn everything_the_facets_cannot_express_is_expressible_here() {
        let f =
            parse("serif --width 75-100 --script Cyrl --script-min 200 --variable=false").unwrap();
        assert_eq!(f.query.as_deref(), Some("serif"));
        assert_eq!(f.width, Some((75, 100)));
        assert_eq!(f.script_min, Some(200));
        assert_eq!(f.variable, Some(false), "not variable, not unset");
    }

    /// An empty line is no filter, not an error: it is how a reader clears one.
    #[test]
    fn an_empty_line_is_the_empty_filter() {
        let f = parse("   ").unwrap();
        assert_eq!(f.query, None);
        assert!(f.scripts.is_empty());
        assert_eq!(f.weight, None);
    }

    /// A family name has spaces in it, so the line is not split on whitespace.
    #[test]
    fn quotes_hold_a_value_together() {
        assert_eq!(
            tokenise(r#"--family "Source Serif 4" --tag 'my fonts'"#).unwrap(),
            ["--family", "Source Serif 4", "--tag", "my fonts"]
        );
        assert_eq!(
            parse(r#"--family "Source Serif 4""#)
                .unwrap()
                .family
                .as_deref(),
            Some("Source Serif 4")
        );
        // Backslashes escape, inside double quotes and out.
        assert_eq!(tokenise(r#"a\ b "c\"d""#).unwrap(), ["a b", "c\"d"]);
        // And a quote nobody closed is a sentence rather than a silent half-argument.
        assert!(tokenise(r#"--family "Source"#).is_err());
        assert!(tokenise("--family 'Source").is_err());
        assert!(tokenise("--family Source\\").is_err());
    }

    /// A single quote inside double quotes is a character, and the other way round,
    /// which is what lets a name with an apostrophe be typed at all.
    #[test]
    fn a_quote_inside_the_other_kind_is_just_a_character() {
        assert_eq!(tokenise(r#""it's""#).unwrap(), ["it's"]);
        assert_eq!(tokenise(r#"'say "hi"'"#).unwrap(), [r#"say "hi""#]);
    }

    /// A mistake gets the sentence the command line would have given, on one line,
    /// because a status line is one line.
    #[test]
    fn a_mistake_is_reported_in_the_command_lines_own_words() {
        let e = parse("--wieght 400").unwrap_err();
        assert!(e.contains("--wieght"), "{e}");
        assert!(!e.contains('\n'), "a status line is one line: {e:?}");

        let e = parse("--weight nonsense").unwrap_err();
        assert!(!e.is_empty() && !e.contains('\n'), "{e:?}");
    }

    /// Every field of `FaceFilter` the command line can set, the browser can set,
    /// because it is the same parser. A handful, spot-checked across the shapes:
    /// strings, bools, enums, ranges and repeats.
    #[test]
    fn the_shapes_of_every_kind_of_field_parse() {
        let f = parse(
            "--family Inter --variable --color=false --italic --lang TRK \
             --lang-source opentype --mono --license OFL-1.1 --vendor ATLR \
             --tag display --collection print --activation user --container woff2 \
             --under /Users/x/Fonts",
        )
        .unwrap();
        assert_eq!(f.family.as_deref(), Some("Inter"));
        assert_eq!(f.variable, Some(true));
        assert_eq!(f.color, Some(false));
        assert_eq!(f.italic, Some(true));
        assert_eq!(f.lang.as_deref(), Some("TRK"));
        assert_eq!(f.monospace, Some(true));
        assert_eq!(f.license.as_deref(), Some("OFL-1.1"));
        assert_eq!(f.vendor.as_deref(), Some("ATLR"));
        assert_eq!(f.tag.as_deref(), Some("display"));
        assert_eq!(f.collection.as_deref(), Some("print"));
        assert_eq!(f.container.as_deref(), Some("woff2"));
        assert_eq!(f.path_prefix.as_deref(), Some("/Users/x/Fonts"));
        assert!(f.activation.is_some());
        assert!(f.lang_source.is_some());
    }
}
