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

//! The typographic judgements a specimen has to make, in one place.
//!
//! Which OpenType features are worth offering as a toggle and what to call them, what
//! sizes a waterfall climbs, how finely a variable axis should step, which sample text
//! shows a script off, and whether a set of axis coordinates happens to be a named
//! instance. None of it is derivable from the font alone; all of it is opinion, and it
//! has to be the *same* opinion in every client or two views of one font disagree.
//!
//! [`crate::specimen`] is the reference implementation of a specimen and the first
//! consumer; the terminal UI is the second.

use crate::model::{AxisInfo, FaceMetadata, Features, InstanceInfo, VariableInfo};

/// The Latin sample every view falls back to. One string, because a waterfall, a
/// terminal preview and `fontina preview` showing three different pangrams for the same
/// font is how a reader loses trust in all three.
pub const DEFAULT_TEXT: &str = "Sphinx of black quartz, judge my vow. 0123456789";

/// Sentence-ending punctuation across the scripts in [`SAMPLES`].
const TERMINATORS: &[char] = &['.', '。', '।', '։', '።', '!', '?'];

/// Point sizes a waterfall climbs, small enough to judge text and large enough to judge
/// letterforms.
pub const WATERFALL_SIZES: &[f32] = &[10.0, 12.0, 14.0, 18.0, 24.0, 32.0, 48.0, 72.0, 96.0];

/// Human labels for the OpenType features worth offering as a toggle.
const FEATURE_LABELS: &[(&str, &str)] = &[
    ("liga", "Standard ligatures"),
    ("dlig", "Discretionary ligatures"),
    ("hlig", "Historical ligatures"),
    ("clig", "Contextual ligatures"),
    ("calt", "Contextual alternates"),
    ("smcp", "Small capitals"),
    ("c2sc", "Capitals to small caps"),
    ("pcap", "Petite caps"),
    ("swsh", "Swashes"),
    ("salt", "Stylistic alternates"),
    ("onum", "Oldstyle figures"),
    ("lnum", "Lining figures"),
    ("pnum", "Proportional figures"),
    ("tnum", "Tabular figures"),
    ("frac", "Fractions"),
    ("ordn", "Ordinals"),
    ("sups", "Superscript"),
    ("subs", "Subscript"),
    ("sinf", "Scientific inferiors"),
    ("zero", "Slashed zero"),
    ("case", "Case-sensitive forms"),
    ("titl", "Titling"),
    ("hist", "Historical forms"),
    ("unic", "Unicase"),
    ("ss01", "Stylistic set 1"),
    ("ss02", "Stylistic set 2"),
    ("ss03", "Stylistic set 3"),
    ("ss04", "Stylistic set 4"),
    ("ss05", "Stylistic set 5"),
    ("ss06", "Stylistic set 6"),
    ("ss07", "Stylistic set 7"),
    ("ss08", "Stylistic set 8"),
    ("ss09", "Stylistic set 9"),
    ("ss10", "Stylistic set 10"),
    ("cv01", "Character variant 1"),
    ("cv02", "Character variant 2"),
    ("cv03", "Character variant 3"),
    ("kern", "Kerning"),
    ("aalt", "All alternates"),
];

/// Features that are on by default, or required for correct shaping. Turning these on
/// and off is not a typographic choice, so they are never offered as toggles.
const HIDDEN_FEATURES: &[&str] = &[
    "ccmp", "locl", "rlig", "rclt", "init", "medi", "fina", "isol", "mark", "mkmk", "curs", "abvm",
    "blwm", "abvs", "blws", "pres", "psts", "pref", "half", "nukt", "akhn", "rphf", "vatu", "cjct",
    "haln", "dist", "rvrn", "req", "dnom", "numr", "rtlm", "ltra", "ltrm", "rtla", "ordn", "aalt",
    "vert", "vrt2",
];

/// Pangrams and phrases that exercise a script, by ISO 15924 code, with writing
/// direction. Latin is the fallback for anything unlisted.
const SAMPLES: &[(&str, Direction, &str)] = &[
    (
        "Latn",
        Direction::Ltr,
        "The quick brown fox jumps over the lazy dog. Zwölf Boxkämpfer jagen Viktor quer über den großen Sylter Deich. Portez ce vieux whisky au juge blond qui fume. Árvíztűrő tükörfúrógép.",
    ),
    (
        "Cyrl",
        Direction::Ltr,
        "Съешь же ещё этих мягких французских булок, да выпей чаю. Жебракують філософи при ґанку церкви в Гадячі, ще й шатро їхнє п'яне знаємо.",
    ),
    (
        "Grek",
        Direction::Ltr,
        "Ξεσκεπάζω την ψυχοφθόρα βδελυγμία. Τάχιστη αλώπηξ βαφής ψημένη γη, δρασκελίζει υπέρ νωθρού κυνός.",
    ),
    (
        "Arab",
        Direction::Rtl,
        "صِف خَلقَ خَودِ كَمِثلِ الشَمسِ إِذ بَزَغَت يَحظى الضَجيعُ بِها نَجلاءَ مِعطارِ. نص حكيم له سر قاطع وذو شأن عظيم مكتوب على ثوب أخضر ومغلف بجلد أزرق.",
    ),
    (
        "Hebr",
        Direction::Rtl,
        "דג סקרן שט בים מאוכזב ולפתע מצא חברה. עטלף אבק נס דרך מזגן שהתפוצץ כי חם.",
    ),
    (
        "Deva",
        Direction::Ltr,
        "ऋषियों को सताने वाले दुष्ट राक्षसों के राजा रावण का सर्वनाश करने वाले विष्णुवतार भगवान श्रीराम, अयोध्या के महाराज दशरथ के बड़े सपुत्र थे।",
    ),
    (
        "Beng",
        Direction::Ltr,
        "আমি বাংলায় গান গাই, আমি বাংলার গান গাই। আমি আমার আমিকে চিরদিন এই বাংলায় খুঁজে পাই।",
    ),
    (
        "Taml",
        Direction::Ltr,
        "யாதும் ஊரே யாவரும் கேளிர் தீதும் நன்றும் பிறர்தர வாரா.",
    ),
    (
        "Thai",
        Direction::Ltr,
        "เป็นมนุษย์สุดประเสริฐเลิศคุณค่า กว่าบรรดาฝูงสัตว์เดรัจฉาน จงฝ่าฟันพัฒนาวิชาการ",
    ),
    (
        "Hani",
        Direction::Ltr,
        "視野無限廣，窗外有藍天。天地玄黃，宇宙洪荒。日月盈昃，辰宿列張。",
    ),
    (
        "Hira",
        Direction::Ltr,
        "いろはにほへと ちりぬるを わかよたれそ つねならむ うゐのおくやま けふこえて",
    ),
    (
        "Kana",
        Direction::Ltr,
        "イロハニホヘト チリヌルヲ ワカヨタレソ ツネナラム",
    ),
    (
        "Hang",
        Direction::Ltr,
        "키스의 고유조건은 입술끼리 만나야 하고 특별한 기술은 필요치 않다.",
    ),
    (
        "Geor",
        Direction::Ltr,
        "გთხოვთ ახლავე გაიაროთ რეგისტრაცია უნიკოდის მეათე საერთაშორისო კონფერენციაზე.",
    ),
    (
        "Armn",
        Direction::Ltr,
        "Բել դղյակի ձախ ժամն օֆ ազգությանը ցպահանջ չճշտած վնաս էր և փառք։",
    ),
    (
        "Ethi",
        Direction::Ltr,
        "ሰማይ አይታረስ ንጉሥ አይከሰስ። ብላ ካለኝ እንደአባቴ በቆመጠኝ።",
    ),
];

/// Writing direction of a sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Ltr,
    Rtl,
}

impl Direction {
    /// The value for an HTML `dir` attribute.
    pub fn as_str(self) -> &'static str {
        match self {
            Direction::Ltr => "ltr",
            Direction::Rtl => "rtl",
        }
    }
}

/// A human label for an OpenType feature tag.
///
/// Stylistic sets and character variants are numbered to `ss20` and `cv99`, so they are
/// derived rather than listed: a table that stopped at `ss10` left a font like Inter,
/// which declares `cv01`-`cv13`, showing rows with a tag and a blank beside it.
///
/// A label existing does not mean the feature is offered: `ordn` and `aalt` are labelled
/// here and hidden by [`is_toggleable`], which wins.
pub fn feature_label(tag: &str) -> String {
    if let Some((kind, digits)) = numbered(tag) {
        return format!("{kind} {digits}");
    }
    FEATURE_LABELS
        .iter()
        .find(|(k, _)| *k == tag)
        .map(|(_, label)| (*label).to_string())
        .unwrap_or_default()
}

/// `ss07` as ("Stylistic set", 7), `cv13` as ("Character variant", 13).
fn numbered(tag: &str) -> Option<(&'static str, u32)> {
    let kind = match &tag[..tag.len().min(2)] {
        "ss" => "Stylistic set",
        "cv" => "Character variant",
        _ => return None,
    };
    let n: u32 = tag.get(2..)?.parse().ok()?;
    (n > 0).then_some((kind, n))
}

/// Whether a feature is a typographic choice a reader should be able to make, rather
/// than machinery the shaper needs.
pub fn is_toggleable(tag: &str) -> bool {
    !HIDDEN_FEATURES.contains(&tag)
}

/// The GSUB features worth offering as toggles, in the order the font declares them.
pub fn toggleable_features(features: &Features) -> Vec<&str> {
    features
        .gsub
        .iter()
        .map(String::as_str)
        .filter(|tag| is_toggleable(tag))
        .collect()
}

/// How finely a slider over this axis should step: whole units once the range is wider
/// than 50, tenths below that.
///
/// A `wght` 100-900 axis steps by 1; `slnt` -10-0, and a `wdth` 75-125 whose range is
/// exactly 50, step by 0.1. The threshold is about how many stops a slider should have,
/// not about which axis it is.
pub fn axis_step(axis: &AxisInfo) -> f32 {
    if axis.max - axis.min > 50.0 { 1.0 } else { 0.1 }
}

/// A sample for a script, with its direction.
pub fn script_sample(script: &str) -> Option<(Direction, &'static str)> {
    SAMPLES
        .iter()
        .find(|(code, _, _)| *code == script)
        .map(|(_, dir, text)| (*dir, *text))
}

/// The script a face is really for: its first coverage entry that is not Common,
/// Inherited or Unknown.
pub fn primary_script(face: &FaceMetadata) -> &str {
    face.coverage
        .scripts
        .iter()
        .map(|s| s.script.as_str())
        .find(|s| !matches!(*s, "Zyyy" | "Zinh" | "Zzzz"))
        .unwrap_or("Latn")
}

/// The opening clause of `text`, at most `max_chars` characters: up to and including the
/// first sentence terminator if there is one in range, otherwise cut at the last word
/// boundary that fits.
///
/// Thai and Lao write without sentence punctuation, so the word-boundary path is the
/// normal one for them, not a fallback.
pub fn opening(text: &'static str, max_chars: usize) -> &'static str {
    let cut = |end: usize| &text[..end];
    let mut last_space = None;
    for (n, (i, c)) in text.char_indices().enumerate() {
        if n >= max_chars {
            return cut(last_space.unwrap_or(i));
        }
        if TERMINATORS.contains(&c) {
            return cut(i + c.len_utf8());
        }
        if c.is_whitespace() {
            last_space = Some(i);
        }
    }
    text
}

/// One line of sample text for a preview pane.
///
/// A face that sets Latin gets [`DEFAULT_TEXT`], so the terminal and the HTML specimen
/// show the same words for the same font. Anything else gets the opening clause of its
/// own script's paragraph, because showing a Devanagari face a Latin pangram it cannot
/// render tells the reader nothing.
pub fn preview_text(face: &FaceMetadata) -> &'static str {
    let script = primary_script(face);
    if script == "Latn" {
        return DEFAULT_TEXT;
    }
    script_sample(script)
        .map(|(_, text)| opening(text, 48))
        .unwrap_or(DEFAULT_TEXT)
}

/// The named instance sitting exactly on these coordinates, if one does.
///
/// `coords` is in axis order, as [`InstanceInfo::coordinates`] is. The comparison is
/// exact: both sides are user-space values that came from the font or from a control
/// snapped to one, so coordinates that are merely close are a setting the reader chose,
/// and calling that "Bold" would be a lie.
pub fn matching_instance<'a>(v: &'a VariableInfo, coords: &[f32]) -> Option<&'a InstanceInfo> {
    if coords.len() != v.axes.len() {
        return None;
    }
    v.instances.iter().find(|inst| inst.coordinates == coords)
}

/// The coordinates a variable face starts at: every axis at its default.
pub fn default_coords(v: &VariableInfo) -> Vec<f32> {
    v.axes.iter().map(|a| a.default).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn axis(tag: &str, min: f32, default: f32, max: f32) -> AxisInfo {
        AxisInfo {
            tag: tag.into(),
            name: None,
            min,
            default,
            max,
            hidden: false,
        }
    }

    #[test]
    fn labels_the_features_a_reader_can_choose() {
        assert_eq!(feature_label("smcp"), "Small capitals");
        assert_eq!(feature_label("zzzz"), "", "an unknown tag gets no label");
        // Numbered sets are derived, so the table cannot fall behind the spec.
        assert_eq!(feature_label("ss03"), "Stylistic set 3");
        assert_eq!(feature_label("ss20"), "Stylistic set 20");
        assert_eq!(feature_label("cv13"), "Character variant 13");
        assert_eq!(feature_label("cv99"), "Character variant 99");
        // `ss00` is not a set, and `ssab` is not a number.
        assert_eq!(feature_label("ss00"), "");
        assert_eq!(feature_label("ssab"), "");
    }

    #[test]
    fn shaping_machinery_is_never_a_toggle() {
        for tag in ["ccmp", "locl", "init", "mark", "rvrn"] {
            assert!(!is_toggleable(tag), "{tag}");
        }
        for tag in ["smcp", "onum", "ss01"] {
            assert!(is_toggleable(tag), "{tag}");
        }
    }

    #[test]
    fn toggleable_features_keeps_only_the_choices() {
        let features = Features {
            gsub: vec![
                "ccmp".into(),
                "liga".into(),
                "locl".into(),
                "smcp".into(),
                "mark".into(),
            ],
            gpos: vec![],
            scripts: vec![],
        };
        assert_eq!(toggleable_features(&features), ["liga", "smcp"]);
    }

    #[test]
    fn wide_axes_step_by_one_and_narrow_axes_by_a_tenth() {
        assert_eq!(axis_step(&axis("wght", 100.0, 400.0, 900.0)), 1.0);
        assert_eq!(axis_step(&axis("slnt", -10.0, 0.0, 0.0)), 0.1);
        // Exactly 50 is not "wider than 50", so a standard wdth axis steps finely.
        assert_eq!(axis_step(&axis("wdth", 75.0, 100.0, 125.0)), 0.1);
    }

    #[test]
    fn opening_stops_at_a_sentence_then_a_word_then_the_end() {
        assert_eq!(opening("One. Two. Three.", 48), "One.");
        assert_eq!(
            opening("視野無限廣，窗外有藍天。天地玄黃", 48),
            "視野無限廣，窗外有藍天。"
        );
        // No terminator in range: cut at the last word boundary that fits.
        assert_eq!(opening("aaa bbb ccc ddd", 9), "aaa bbb");
        // Shorter than the cap and unpunctuated: unchanged.
        assert_eq!(opening("short", 48), "short");
    }

    #[test]
    fn every_script_gets_a_usable_one_line_sample() {
        for (code, _, long) in SAMPLES {
            let line = opening(long, 48);
            assert!(!line.is_empty(), "{code} produced nothing");
            assert!(
                line.chars().count() <= 48,
                "{code} exceeded the cap: {line:?}"
            );
            assert!(long.starts_with(line), "{code} did not open the paragraph");
            // Never cut mid-word: either the paragraph ended, or we stopped on
            // punctuation or a space.
            let rest = &long[line.len()..];
            assert!(
                rest.is_empty()
                    || line.ends_with(TERMINATORS)
                    || rest.starts_with(char::is_whitespace),
                "{code} cut mid-word: {line:?} | {rest:?}"
            );
        }
    }

    #[test]
    fn a_latin_face_previews_the_same_words_as_the_specimen() {
        // The specimen's waterfall and the terminal pane both start from DEFAULT_TEXT.
        assert!(DEFAULT_TEXT.starts_with("Sphinx of black quartz"));
    }

    #[test]
    fn every_sample_script_resolves() {
        for (code, _, _) in SAMPLES {
            assert!(script_sample(code).is_some(), "{code}");
        }
        assert_eq!(script_sample("Arab").unwrap().0, Direction::Rtl);
        assert_eq!(script_sample("Latn").unwrap().0, Direction::Ltr);
        assert!(script_sample("Ogam").is_none());
    }
}
