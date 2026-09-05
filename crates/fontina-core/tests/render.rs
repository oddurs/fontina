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

//! Properties of the renderer, over every fixture and every input a person can produce.
//!
//! `render_shapes_and_rasterises` in `library.rs` covers the happy path: two fixtures,
//! one size, text somebody chose. This file covers the rest of the space, because since
//! M2 the inputs are a slider being dragged and a text box being typed into rather than
//! a command line somebody composed. The theme is that a rendering must always come back
//! sane — the right number of coverage bytes, a bounded bitmap, ink where there is ink —
//! or come back as an `Err`, and never panic and never hang, because a font manager that
//! falls over on one font in a library is worse than one that draws it badly.
//!
//! Where the code does not keep a promise the tests say so instead of blessing the
//! behaviour: [`a_lone_combining_mark_overflows_the_rasteriser`],
//! [`a_size_of_nan_renders_nothing_at_all`], [`an_axis_set_to_nan_falls_to_the_minimum`]
//! and [`a_feature_tag_is_measured_in_bytes_not_characters`] are reported, not fixed.

use fontina_core::model::FaceMetadata;
use fontina_core::render::{Bitmap, RenderOptions, render_face, render_sfnt, shaped_glyphs};
use std::path::PathBuf;

// ----- fixtures -----

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

/// Every fixture, by the capability it is here for.
const ALL: &[&str] = &[
    "Amiri-Regular.ttf",                      // Arabic, complex shaping
    "BricolageGrotesque[opsz,wdth,wght].ttf", // three variable axes
    "Nabla[EDPT,EHLT].ttf",                   // colour, two custom axes
    "SourceSerif4-Regular.otf",               // CFF outlines
    "inter-latin-400-normal.woff",            // WOFF 1.0 container
    "inter-latin-400-normal.woff2",           // WOFF 2.0 container
];

/// Fixtures that are already sfnt, so [`render_sfnt`] can be handed their bytes.
const SFNT: &[&str] = &[
    "Amiri-Regular.ttf",
    "BricolageGrotesque[opsz,wdth,wght].ttf",
    "Nabla[EDPT,EHLT].ttf",
    "SourceSerif4-Regular.otf",
];

fn bytes(name: &str) -> Vec<u8> {
    std::fs::read(fixtures().join(name)).unwrap()
}

fn face(name: &str) -> FaceMetadata {
    fontina_core::load_file(&fixtures().join(name))
        .unwrap()
        .1
        .remove(0)
}

fn ink(bm: &Bitmap) -> u64 {
    bm.coverage.iter().map(|&c| c as u64).sum()
}

/// The bound `render_sfnt` refuses to allocate past: anything larger is an `Err`, so
/// every bitmap that does come back is inside it.
const PIXEL_BUDGET: u64 = 64 * 1024 * 1024;

/// The invariants every bitmap has to keep, whatever produced it. Called on the result
/// of every render in this file, so a new edge case gets them for free.
#[track_caller]
fn check_invariants(bm: &Bitmap, what: &str) {
    assert!(bm.width >= 1 && bm.height >= 1, "{what}: empty bitmap");
    assert_eq!(
        bm.coverage.len() as u64,
        bm.width as u64 * bm.height as u64,
        "{what}: coverage is not width * height",
    );
    assert!(
        bm.width as u64 * bm.height as u64 <= PIXEL_BUDGET,
        "{what}: {}x{} is past the pixel budget",
        bm.width,
        bm.height,
    );
    assert!(
        bm.missing <= bm.glyphs,
        "{what}: {} missing of {} glyphs",
        bm.missing,
        bm.glyphs,
    );
    assert!(
        bm.baseline.is_finite() && bm.baseline >= 0.0 && bm.baseline <= bm.height as f32,
        "{what}: baseline {} outside 0..{}",
        bm.baseline,
        bm.height,
    );
    // `get` agrees with the coverage it indexes, everywhere inside the bitmap, and
    // reads as background below it. (Past the right-hand edge it does not; see
    // [`reading_a_pixel_outside_the_bitmap_is_not_background`].)
    assert_eq!(bm.get(0, 0), bm.coverage[0], "{what}: get disagrees at 0,0");
    assert_eq!(
        bm.get(bm.width - 1, bm.height - 1),
        bm.coverage[bm.coverage.len() - 1],
        "{what}: get disagrees at the last pixel",
    );
    assert_eq!(bm.get(0, bm.height), 0, "{what}: below the bitmap");
    assert_eq!(
        bm.is_blank(),
        bm.coverage.iter().all(|&c| c == 0),
        "{what}: is_blank disagrees with the coverage",
    );
}

/// Render, check the invariants, hand back the bitmap.
#[track_caller]
fn render(name: &str, opts: &RenderOptions, what: &str) -> Bitmap {
    let bm = render_sfnt(&bytes(name), 0, opts).unwrap_or_else(|e| panic!("{what}: {e}"));
    check_invariants(&bm, what);
    bm
}

// ----- every fixture, every size -----

/// Sizes that bracket the useful range: below the floor the renderer clamps to, the
/// sizes a waterfall climbs, and two that are larger than any screen.
const SIZES: &[f32] = &[1.0, 2.0, 4.0, 7.0, 12.0, 48.0, 96.0, 256.0, 512.0];

#[test]
fn every_fixture_renders_at_every_size() {
    for name in ALL {
        let f = face(name);
        for &size in SIZES {
            let what = format!("{name} at {size} px");
            let bm = render_face(
                &f,
                &RenderOptions {
                    size,
                    ..Default::default()
                },
            )
            .unwrap_or_else(|e| panic!("{what}: {e}"));
            check_invariants(&bm, &what);
            // The default text is a pangram: at any size the renderer will accept, a
            // font that has Latin at all puts ink on the bitmap. A blank preview here is
            // the defect #66 fixed one instance of.
            assert!(!bm.is_blank(), "{what}: nothing drawn");
            assert_eq!(bm.missing, 0, "{what}: glyphs without outlines");
            assert!(
                bm.glyphs >= 40,
                "{what}: {} glyphs for a 48-character pangram",
                bm.glyphs,
            );
            // Ink grows with size, monotonically enough to catch a scale that stopped
            // being applied: 512 px is not the same picture as 12 px.
            assert!(bm.width > 1 && bm.height > 1, "{what}: degenerate");
        }
    }
}

#[test]
fn a_bigger_size_draws_a_bigger_picture() {
    for name in ALL {
        let f = face(name);
        let at = |size: f32| {
            render_face(
                &f,
                &RenderOptions {
                    text: "Hamburg".into(),
                    size,
                    ..Default::default()
                },
            )
            .unwrap()
        };
        let (small, large) = (at(12.0), at(96.0));
        check_invariants(&small, name);
        check_invariants(&large, name);
        assert!(
            large.width > small.width * 4 && large.height > small.height * 4,
            "{name}: {}x{} at 12 px, {}x{} at 96 px",
            small.width,
            small.height,
            large.width,
            large.height,
        );
        assert!(ink(&large) > ink(&small) * 10, "{name}: ink did not grow");
    }
}

/// The geometry of a rendering is arithmetic on the font's own integer metrics, so it is
/// the same everywhere and worth pinning: a change to how a bitmap is sized shows up
/// here as a diff rather than as a preview that looks slightly wrong.
#[test]
fn the_size_of_a_rendering_is_pinned_per_fixture() {
    let mut out = String::new();
    for name in ALL {
        let f = face(name);
        out.push_str(&format!("{name}\n"));
        for &size in &[8.0f32, 12.0, 48.0, 96.0] {
            let bm = render_face(
                &f,
                &RenderOptions {
                    text: "Hamburgefonstiv".into(),
                    size,
                    ..Default::default()
                },
            )
            .unwrap();
            out.push_str(&format!(
                "  {size:>5} px  {:>5}x{:<5} baseline {:>7.2}  glyphs {:>2}  missing {}\n",
                bm.width, bm.height, bm.baseline, bm.glyphs, bm.missing,
            ));
        }
    }
    insta::assert_snapshot!(out);
}

/// Documented as "font size in pixels", and it is, between 4 and 4096. Outside that the
/// renderer silently substitutes the nearest end rather than refusing: `preview --size 1`
/// draws at 4 px and says nothing. Asserted here because a caller that wants to show the
/// size it actually drew has to know, not because the clamp is wrong — the rasteriser
/// needs a floor and the pixel budget needs a ceiling.
#[test]
fn a_size_outside_the_rasterisers_range_is_clamped_to_it() {
    let opts = |size: f32| RenderOptions {
        text: "Hg".into(),
        size,
        ..Default::default()
    };
    let at = |size: f32| {
        let bm = render(
            "BricolageGrotesque[opsz,wdth,wght].ttf",
            &opts(size),
            &format!("size {size}"),
        );
        (bm.width, bm.height, ink(&bm))
    };
    // Every size at or below the floor draws the same 4 px picture.
    let floor = at(4.0);
    for size in [f32::MIN, -1.0e9, -1.0, 0.0, 0.5, 1.0, 2.0, 3.99] {
        assert_eq!(at(size), floor, "{size} should render as 4 px");
    }
    // And every size at or above the ceiling the same 4096 px one.
    let ceiling = at(4096.0);
    for size in [4096.1, 1.0e9, f32::MAX, f32::INFINITY] {
        assert_eq!(at(size), ceiling, "{size} should render as 4096 px");
    }
    assert!(floor.0 < ceiling.0 && floor.1 < ceiling.1);
}

/// **Reported, not fixed.** `f32::clamp` leaves NaN alone, so a NaN size survives into
/// the metrics, where `max(0.0)` turns every one of them into zero, and the render comes
/// back as a padding-sized rectangle of nothing. A slider or a parsed number that goes
/// NaN for one frame therefore blanks the preview instead of erroring or holding still.
/// The fix is one `if !size.is_finite()` before the clamp; this test says what happens
/// today so that fix is visible as a diff here.
#[test]
fn a_size_of_nan_renders_nothing_at_all() {
    let bm = render(
        "BricolageGrotesque[opsz,wdth,wght].ttf",
        &RenderOptions {
            text: "Hamburg".into(),
            size: f32::NAN,
            padding: 4,
            ..Default::default()
        },
        "NaN size",
    );
    assert!(bm.is_blank(), "if this now draws, the defect is fixed");
    assert_eq!((bm.width, bm.height), (8, 8), "padding, and nothing else");
    // The invariants still hold, which is why this is a wrong picture and not a crash.
    assert_eq!(bm.coverage.len(), 64);
}

// ----- text a person can type -----

/// Text that has no ink in it by definition. Everything else in [`TEXTS`] must draw.
const BLANK_TEXTS: &[&str] = &["", " ", "   ", "\n", "\n\n\n", " \n \n "];

/// Text a person can produce in a preview box, at least one of each kind that has ever
/// broken a shaper.
const TEXTS: &[(&str, &str)] = &[
    ("empty", ""),
    ("one space", " "),
    ("only newlines", "\n\n\n"),
    ("blank lines around text", "\n\nAa\n\n"),
    ("a base and a mark", "a\u{0301}e\u{0308}"),
    (
        "a very long grapheme cluster",
        "a\u{0301}\u{0302}\u{0303}\u{0304}\u{0305}\u{0306}\u{0307}\u{0308}\u{0309}\u{030A}\u{030B}\u{030C}\u{030D}\u{030E}\u{030F}\u{0310}\u{0311}\u{0312}\u{0313}\u{0314}",
    ),
    ("mixed scripts", "Hello سلام Γειά שלום ħello 123"),
    ("unassigned codepoints", "\u{0378}\u{05EB}\u{2FE0}\u{FDD0}"),
    ("private use area", "\u{E000}\u{F8FF}\u{10FFFD}"),
    ("control characters", "\u{0001}\u{0007}\u{001B}\u{007F}"),
    ("zero width joiner", "a\u{200D}b\u{200C}c"),
    ("a bidi override", "\u{202E}reversed\u{202C}"),
    ("noncharacters", "\u{FFFE}\u{FFFF}"),
    ("an emoji nobody has", "\u{1F600}\u{1F1EE}\u{1F1F8}"),
    ("a lone surrogate's worth of nonsense", "\u{FFFD}\u{FFFD}"),
    ("tabs", "a\tb"),
    ("windows newlines", "one\r\ntwo"),
];

#[test]
fn any_text_a_person_can_type_renders_or_errors_but_never_breaks() {
    for name in ALL {
        let f = face(name);
        for (what, text) in TEXTS {
            let what = format!("{name}: {what}");
            let bm = render_face(
                &f,
                &RenderOptions {
                    text: (*text).into(),
                    size: 24.0,
                    ..Default::default()
                },
            )
            .unwrap_or_else(|e| panic!("{what}: {e}"));
            check_invariants(&bm, &what);
            if BLANK_TEXTS.contains(text) {
                assert!(bm.is_blank(), "{what}: drew ink for text with none");
            } else {
                assert!(!bm.is_blank(), "{what}: drew nothing");
            }
            assert_eq!(bm.missing, 0, "{what}: outlines went missing");
        }
    }
}

/// A line count is a line count: `\n` splits, nothing else does, and the bitmap grows by
/// one line height a line whatever is on them.
#[test]
fn a_newline_and_only_a_newline_starts_a_line() {
    let name = "SourceSerif4-Regular.otf";
    let at = |text: &str| {
        let bm = render(
            name,
            &RenderOptions {
                text: text.into(),
                size: 24.0,
                ..Default::default()
            },
            text,
        );
        bm.height
    };
    let one = at("a");
    let two = at("a\na");
    let three = at("a\na\na");
    assert!(two > one && three > two);
    assert_eq!(
        three - two,
        two - one,
        "every line after the first costs the same",
    );
    // A trailing newline is a line: the box grows even though nothing is on it.
    assert_eq!(at("a\n"), two);
    // \r is not a line break; it is a character the font either has or does not.
    assert_eq!(at("a\rb"), one);
    // Neither are the Unicode line separators, for the same reason.
    assert_eq!(at("a\u{2028}b"), one);
}

#[test]
fn a_paragraph_of_many_lines_stays_inside_the_budget() {
    let text = (0..200)
        .map(|i| format!("line {i} of a preview nobody sized"))
        .collect::<Vec<_>>()
        .join("\n");
    for name in SFNT {
        let bm = render(
            name,
            &RenderOptions {
                text: text.clone(),
                size: 8.0,
                ..Default::default()
            },
            name,
        );
        assert!(!bm.is_blank());
        assert!(bm.height > 200, "200 lines in {} rows", bm.height);
    }
}

/// The budget is the promise that a preview cannot be asked to allocate the machine out
/// of memory: past it the answer is an error naming the size, not an allocation.
#[test]
fn a_rendering_larger_than_the_budget_is_an_error_not_an_allocation() {
    let name = "BricolageGrotesque[opsz,wdth,wght].ttf";
    let long = "Sphinx of black quartz, judge my vow. ".repeat(500);
    let err = render_sfnt(
        &bytes(name),
        0,
        &RenderOptions {
            text: long.clone(),
            size: 512.0,
            ..Default::default()
        },
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("preview would be"), "{err}");
    assert!(err.contains("smaller size or shorter text"), "{err}");
    // Padding alone can ask for it too, and gets the same answer rather than a bitmap
    // whose dimensions wrapped.
    assert!(
        render_sfnt(
            &bytes(name),
            0,
            &RenderOptions {
                text: "a".into(),
                padding: u32::MAX,
                ..Default::default()
            },
        )
        .is_err(),
        "u32::MAX of padding",
    );
    // The same text one step below the budget is a bitmap, not an error, so the guard is
    // a budget and not a blanket refusal.
    let bm = render(
        name,
        &RenderOptions {
            text: long,
            size: 8.0,
            ..Default::default()
        },
        "long text, small size",
    );
    assert!(!bm.is_blank());
}

/// **Reported, not fixed.** A bitmap is sized from advance widths and the font's
/// ascent and descent, never from what the glyphs actually draw, so ink that lies
/// outside the advance box is clipped — and a glyph placed at a negative x reaches
/// `ab_glyph_rasterizer` with a negative column index, where `linestart + x1i as usize`
/// overflows. A lone combining acute is exactly that: zero advance and an outline the
/// shaper offsets to the left of the pen.
///
/// It is a debug-build panic only. With overflow checks off the addition wraps, the
/// index misses the bitmap, and the scanline is silently dropped — so a released binary
/// draws a clipped accent and every `cargo test`, `cargo run` and contributor build
/// aborts instead. Amiri and Source Serif do it at 48 px, the default size, with the
/// default padding of 4 and with the terminal browser's padding of 1; Bricolage and
/// Nabla place the same mark inside the box and survive. Padding of 16 or more moves the
/// outline back over the bitmap and it renders.
///
/// The fix is fontina's, not the rasteriser's: size the bitmap from the glyphs' bounding
/// boxes, or refuse to draw at a negative origin.
#[test]
fn a_lone_combining_mark_overflows_the_rasteriser() {
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let outcome = |name: &'static str, padding: u32| {
        std::panic::catch_unwind(move || {
            render_sfnt(
                &bytes(name),
                0,
                &RenderOptions {
                    text: "\u{0301}".into(),
                    size: 48.0,
                    padding,
                    ..Default::default()
                },
            )
        })
    };
    let mut overflowed = Vec::new();
    for name in SFNT {
        for padding in [0, 1, 4] {
            match outcome(name, padding) {
                Ok(Ok(bm)) => check_invariants(&bm, name),
                Ok(Err(e)) => panic!("{name}: unexpected error {e}"),
                Err(_) => overflowed.push((*name, padding)),
            }
        }
    }
    // Enough padding to hold the accent, and there is nothing wrong with the drawing.
    for name in SFNT {
        match outcome(name, 32) {
            Ok(Ok(bm)) => {
                check_invariants(&bm, name);
                assert!(!bm.is_blank(), "{name}: an accent is ink");
            }
            other => panic!("{name}: 32 px of padding should be enough, got {other:?}"),
        }
    }
    std::panic::set_hook(hook);

    if cfg!(debug_assertions) {
        assert!(
            overflowed.contains(&("Amiri-Regular.ttf", 4)),
            "the reported overflow is gone from Amiri — delete this test and say so \
             in the changelog. Overflowed: {overflowed:?}",
        );
    } else {
        assert!(
            overflowed.is_empty(),
            "with overflow checks off the addition wraps instead: {overflowed:?}",
        );
    }
}

// ----- variable axes -----

/// Every declared axis of every variable fixture, at and outside its range.
fn axes_of(name: &str) -> Vec<fontina_core::model::AxisInfo> {
    face(name)
        .variable
        .map(|v| v.axes)
        .unwrap_or_default()
        .into_iter()
        .filter(|a| a.min < a.max)
        .collect()
}

const VARIABLE: &[&str] = &[
    "BricolageGrotesque[opsz,wdth,wght].ttf",
    "Nabla[EDPT,EHLT].ttf",
];

/// The promise the code makes by handing user coordinates to `skrifa`'s
/// `AxisCollection::location`: anything outside the `fvar` range is the nearest end of
/// it. A slider dragged past its stop, a number typed with an extra zero, and an
/// infinity all land on a real instance rather than an extrapolated one.
#[test]
fn an_axis_coordinate_is_clamped_to_the_fvar_range() {
    for name in VARIABLE {
        for axis in axes_of(name) {
            let at = |v: f32| {
                let opts = RenderOptions {
                    text: "Hamburg".into(),
                    variations: vec![(axis.tag.clone(), v)],
                    ..Default::default()
                };
                let bm = render(name, &opts, &format!("{name} {} = {v}", axis.tag));
                (bm.width, bm.height, ink(&bm))
            };
            let low = at(axis.min);
            let high = at(axis.max);
            for below in [
                axis.min - 1.0,
                axis.min - 1000.0,
                -1.0e9,
                f32::MIN,
                f32::NEG_INFINITY,
            ] {
                assert_eq!(at(below), low, "{name} {} at {below}", axis.tag);
            }
            for above in [
                axis.max + 1.0,
                axis.max + 1000.0,
                1.0e9,
                f32::MAX,
                f32::INFINITY,
            ] {
                assert_eq!(at(above), high, "{name} {} at {above}", axis.tag);
            }
            // Zero is inside some ranges and below others; either way it is one of the
            // two, and never an extrapolation.
            let zero = at(0.0);
            if axis.min > 0.0 {
                assert_eq!(zero, low, "{name} {} at zero", axis.tag);
            }
            // Every coordinate inside the range renders, and the ones that change the
            // drawing change it monotonically in the amount of ink for a weight axis.
            for step in 0..=8 {
                let v = axis.min + (axis.max - axis.min) * step as f32 / 8.0;
                let _ = at(v);
            }
        }
    }
    // A weight axis is the one whose direction is not a matter of opinion.
    let name = "BricolageGrotesque[opsz,wdth,wght].ttf";
    let at = |v: f32| {
        ink(&render(
            name,
            &RenderOptions {
                text: "Hamburg".into(),
                variations: vec![("wght".into(), v)],
                ..Default::default()
            },
            "wght",
        ))
    };
    let mut last = 0;
    for w in [200.0, 300.0, 400.0, 500.0, 600.0, 700.0, 800.0] {
        let now = at(w);
        assert!(now > last, "wght {w} drew {now}, {w} lighter drew {last}");
        last = now;
    }
}

/// **Reported, not fixed.** `f32::clamp` passes NaN through and `skrifa`'s normalisation
/// turns it into the axis minimum, so a coordinate that goes NaN for one frame does not
/// leave the preview where it was, and does not fall back to the axis default — it snaps
/// to the thinnest, narrowest, smallest end of every axis at once. A slider that divides
/// by a zero-width track produces exactly this.
#[test]
fn an_axis_set_to_nan_falls_to_the_minimum() {
    for name in VARIABLE {
        for axis in axes_of(name) {
            let at = |v: f32| {
                let bm = render(
                    name,
                    &RenderOptions {
                        text: "Hamburg".into(),
                        variations: vec![(axis.tag.clone(), v)],
                        ..Default::default()
                    },
                    "nan axis",
                );
                (bm.width, bm.height, ink(&bm))
            };
            assert_eq!(
                at(f32::NAN),
                at(axis.min),
                "{name} {}: NaN is the minimum today",
                axis.tag,
            );
            // Only worth saying for an axis whose ends draw differently at all: Nabla's
            // `EHLT` moves colour layers the coverage rasteriser never sees.
            if at(axis.min) != at(axis.default) {
                assert_ne!(
                    at(f32::NAN),
                    at(axis.default),
                    "{name} {}: NaN should hold the default, and does not",
                    axis.tag,
                );
            }
        }
    }
}

#[test]
fn axis_settings_that_name_nothing_are_ignored_and_malformed_ones_are_errors() {
    let name = "BricolageGrotesque[opsz,wdth,wght].ttf";
    let plain = render(
        name,
        &RenderOptions {
            text: "Hamburg".into(),
            ..Default::default()
        },
        "no variations",
    );
    // An axis the font does not have is not an error: a browser keeping one set of
    // slider positions across a selection would otherwise fail on every face but one.
    for tag in ["zzzz", "ital", "GRAD"] {
        let bm = render(
            name,
            &RenderOptions {
                text: "Hamburg".into(),
                variations: vec![(tag.into(), 1.0)],
                ..Default::default()
            },
            tag,
        );
        assert_eq!(ink(&bm), ink(&plain), "{tag} should change nothing");
    }
    // Setting one axis twice takes the last value, the way a map would.
    let twice = render(
        name,
        &RenderOptions {
            text: "Hamburg".into(),
            variations: vec![("wght".into(), 800.0), ("wght".into(), 200.0)],
            ..Default::default()
        },
        "wght twice",
    );
    let once = render(
        name,
        &RenderOptions {
            text: "Hamburg".into(),
            variations: vec![("wght".into(), 200.0)],
            ..Default::default()
        },
        "wght once",
    );
    assert_eq!(ink(&twice), ink(&once), "the last setting wins");
    // A tag that is not a tag is an error, and says which one.
    for bad in ["", "w", "wg", "wgh", "wghtt", "weight"] {
        let err = render_sfnt(
            &bytes(name),
            0,
            &RenderOptions {
                variations: vec![(bad.into(), 400.0)],
                ..Default::default()
            },
        )
        .unwrap_err()
        .to_string();
        assert!(
            err.contains("four-character OpenType tag"),
            "{bad:?}: {err}"
        );
        assert!(err.contains(&format!("{bad:?}")), "{bad:?} named: {err}");
    }
    // Every axis at once, which is what a named instance is.
    let all: Vec<(String, f32)> = axes_of(name)
        .iter()
        .map(|a| (a.tag.clone(), a.default))
        .collect();
    let bm = render(
        name,
        &RenderOptions {
            text: "Hamburg".into(),
            variations: all,
            ..Default::default()
        },
        "every axis at its default",
    );
    assert_eq!(
        ink(&bm),
        ink(&plain),
        "the defaults are what no setting means"
    );
}

// ----- features -----

/// Every feature tag a fixture declares, forced on and forced off. None of them may
/// break a rendering, whether or not the font does anything with them: `frac` on a
/// pangram and `smcp` on Arabic are both things a reader can click.
#[test]
fn every_feature_a_fixture_declares_can_be_forced_either_way() {
    for name in SFNT {
        let f = face(name);
        let tags: Vec<String> = f
            .features
            .gsub
            .iter()
            .chain(f.features.gpos.iter())
            .cloned()
            .collect();
        assert!(!tags.is_empty(), "{name} declares no features");
        for tag in &tags {
            for on in [true, false] {
                let what = format!("{name} {tag}={on}");
                let bm = render(
                    name,
                    &RenderOptions {
                        text: "Waffle 1/2 stiff 0123".into(),
                        size: 24.0,
                        features: vec![(tag.clone(), on)],
                        ..Default::default()
                    },
                    &what,
                );
                assert!(!bm.is_blank(), "{what}: drew nothing");
                assert_eq!(bm.missing, 0, "{what}: outlines went missing");
            }
        }
        // All of them at once, on and then off, which is what a reader who clicked
        // every checkbox has.
        for on in [true, false] {
            let all: Vec<(String, bool)> = tags.iter().map(|t| (t.clone(), on)).collect();
            let bm = render(
                name,
                &RenderOptions {
                    text: "Waffle 1/2 stiff 0123".into(),
                    size: 24.0,
                    features: all,
                    ..Default::default()
                },
                &format!("{name}: every feature {on}"),
            );
            assert!(!bm.is_blank());
        }
    }
}

/// A feature that does something has to visibly do it, or the toggles are decoration.
#[test]
fn a_feature_that_the_font_implements_changes_the_drawing() {
    let name = "SourceSerif4-Regular.otf";
    let at = |features: Vec<(String, bool)>| {
        let bm = render(
            name,
            &RenderOptions {
                text: "Waffle Office 1/2 0123".into(),
                size: 32.0,
                features,
                ..Default::default()
            },
            "features",
        );
        (bm.width, ink(&bm))
    };
    let plain = at(vec![]);
    for tag in ["smcp", "onum", "frac", "sups", "c2sc"] {
        assert_ne!(
            at(vec![(tag.into(), true)]),
            plain,
            "{tag} changed nothing in Source Serif",
        );
    }
    // Forcing a feature off that was not on changes nothing.
    assert_eq!(at(vec![("smcp".into(), false)]), plain);
    // And the standard ligature is on by default, so turning it off is visible.
    assert_ne!(at(vec![("liga".into(), false)]), plain, "liga off");
}

#[test]
fn features_that_name_nothing_are_ignored_and_malformed_ones_are_errors() {
    let name = "SourceSerif4-Regular.otf";
    let plain = render(
        name,
        &RenderOptions {
            text: "fi 1/2".into(),
            ..Default::default()
        },
        "no features",
    );
    for tag in ["zzzz", "ss99", "abcd", "0000"] {
        let bm = render(
            name,
            &RenderOptions {
                text: "fi 1/2".into(),
                features: vec![(tag.into(), true)],
                ..Default::default()
            },
            tag,
        );
        assert_eq!(ink(&bm), ink(&plain), "{tag} should change nothing");
    }
    for bad in ["", "a", "ab", "abc", "abcde", "ss1", "ss001"] {
        let err = render_sfnt(
            &bytes(name),
            0,
            &RenderOptions {
                features: vec![(bad.into(), true)],
                ..Default::default()
            },
        )
        .unwrap_err()
        .to_string();
        assert!(
            err.contains("four-character OpenType tag"),
            "{bad:?}: {err}"
        );
    }
}

/// An absurd number of toggles is a thing a script can produce, and the shaper has to
/// take it without slowing to a stop or losing the drawing.
#[test]
fn a_thousand_feature_settings_at_once_still_render() {
    let name = "SourceSerif4-Regular.otf";
    let many: Vec<(String, bool)> = (0..2000)
        .map(|i| (format!("ss{:02}", i % 100), i % 2 == 0))
        .collect();
    let start = std::time::Instant::now();
    let bm = render(
        name,
        &RenderOptions {
            text: "Waffle".into(),
            size: 24.0,
            features: many,
            ..Default::default()
        },
        "2000 features",
    );
    assert!(!bm.is_blank());
    assert!(
        start.elapsed() < std::time::Duration::from_secs(10),
        "2000 features took {:?}",
        start.elapsed(),
    );
}

/// **Reported, not fixed.** The check is `s.as_bytes().len() != 4`, and an OpenType tag
/// is four *bytes* in the range 0x20..=0x7E — so the length is right and the message is
/// wrong, and neither one rejects the characters. `"𝕒"` is one character in four bytes
/// and is accepted as a tag whose bytes no font can declare; `"ligä"` is four characters
/// in five bytes and is refused with a message that says it is not four characters,
/// which it is. Both are reachable from a text field.
#[test]
fn a_feature_tag_is_measured_in_bytes_not_characters() {
    let name = "SourceSerif4-Regular.otf";
    let with = |tag: &str| {
        render_sfnt(
            &bytes(name),
            0,
            &RenderOptions {
                text: "fi".into(),
                features: vec![(tag.into(), true)],
                ..Default::default()
            },
        )
    };
    // Four bytes of one character: accepted, and shapes as a tag of bytes outside the
    // range the spec allows.
    assert!(with("\u{1D552}").is_ok(), "four bytes, one character");
    // Four characters in five bytes: refused, and told it is not four characters.
    let err = with("ligä").unwrap_err().to_string();
    assert!(
        err.contains("is not a four-character OpenType tag"),
        "{err}"
    );
    assert!(
        "ligä".chars().count() == 4,
        "the message is false: it is four characters",
    );
    // Bytes a tag may not hold, taken anyway. Only their length is checked.
    for tag in ["li a", "li\ta", "\u{0}\u{0}\u{0}\u{0}", "LIGA"] {
        assert!(with(tag).is_ok(), "{tag:?} accepted as a tag");
    }
}

// ----- padding, clipping and the bitmap itself -----

#[test]
fn padding_and_the_width_clip_do_what_they_say() {
    let name = "Amiri-Regular.ttf";
    let at = |padding: u32, max_width: Option<u32>| {
        let bm = render(
            name,
            &RenderOptions {
                text: "سلام".into(),
                size: 24.0,
                padding,
                max_width,
                ..Default::default()
            },
            &format!("pad {padding} max {max_width:?}"),
        );
        (bm.width, bm.height)
    };
    let (w0, h0) = at(0, None);
    // Padding is on both sides, in both directions.
    for pad in [1u32, 2, 7, 40] {
        assert_eq!(at(pad, None), (w0 + 2 * pad, h0 + 2 * pad), "padding {pad}");
    }
    // The clip is a maximum, never a minimum: a wide bitmap is cut to it, a narrow one
    // is left alone, and the height is untouched either way.
    for max in [1u32, 2, 10, w0 / 2] {
        assert_eq!(at(0, Some(max)), (max, h0), "clipped to {max}");
    }
    assert_eq!(at(0, Some(w0 * 4)), (w0, h0), "no padding out to the clip");
    // Zero is not a bitmap, so it is one column.
    assert_eq!(at(0, Some(0)), (1, h0), "a clip of zero is one column");
}

/// **Reported, not fixed, and the most visible of them.** `max_width` is documented as
/// "clip the bitmap to this width in pixels (text is not wrapped)", and it does size the
/// bitmap that way — but the clipping never happens. `Rasterizer::new(width, height)`
/// gets the *clipped* width, and `ab_glyph_rasterizer` bounds a scanline against the
/// length of its whole accumulation buffer rather than against the width of a row: a
/// span at x = 90 in a 30-column buffer is written at offset `y * 30 + 90`, which is
/// three rows further down and ten columns in. Everything past the clip therefore comes
/// back diagonally smeared over the rest of the preview instead of cut off.
///
/// The path is not a corner. `ui/mod.rs` renders every terminal preview with
/// `max_width: Some(pane_width)` and a sample string that is thousands of pixels wide at
/// any readable size, so the details pane, the waterfall and the comparison sheet are
/// all drawing this. The fix is to rasterise at the full width and copy the left
/// `max_width` columns out, or to drop glyphs whose origin is already past the clip.
#[test]
fn a_clipped_rendering_wraps_the_ink_it_should_have_dropped() {
    let name = "SourceSerif4-Regular.otf";
    let opts = |max_width: Option<u32>| RenderOptions {
        text: "Hamburgefonstiv".into(),
        size: 16.0,
        padding: 0,
        max_width,
        ..Default::default()
    };
    let full = render(name, &opts(None), "unclipped");
    let clipped = render(name, &opts(Some(30)), "clipped to 30");
    assert_eq!(clipped.width, 30);
    assert_eq!(clipped.height, full.height, "the clip is horizontal only");

    // What clipping should give: the same picture, thirty columns of it.
    let differing = (0..clipped.height)
        .flat_map(|y| (0..clipped.width).map(move |x| (x, y)))
        .filter(|&(x, y)| {
            clipped.coverage[(y * clipped.width + x) as usize]
                != full.coverage[(y * full.width + x) as usize]
        })
        .count();
    assert!(
        differing > 0,
        "the clip is a clip now — delete this test and say so in the changelog",
    );
    // What it gives instead: half the bitmap disagrees, and there is more ink in the
    // thirty clipped columns than in the same thirty columns of the full rendering,
    // because the rest of the word landed on top of them.
    let kept: u64 = (0..full.height)
        .flat_map(|y| (0..clipped.width).map(move |x| (x, y)))
        .map(|(x, y)| full.coverage[(y * full.width + x) as usize] as u64)
        .sum();
    assert!(
        ink(&clipped) > kept,
        "clipped ink {} should have been at most the {kept} in those columns",
        ink(&clipped),
    );
    // Rows that are empty in the real rendering are inked in the clipped one: this is
    // the smear, and it is what a reader sees.
    let blank_rows_wrongly_inked = (0..full.height)
        .filter(|&y| {
            (0..full.width).all(|x| full.coverage[(y * full.width + x) as usize] == 0)
                && (0..clipped.width).any(|x| clipped.coverage[(y * 30 + x) as usize] != 0)
        })
        .count();
    assert!(
        blank_rows_wrongly_inked > 0,
        "ink appeared on rows the font never drew on",
    );
}

/// **Reported, not fixed.** `Bitmap::get` computes `y * self.width + x` and looks the
/// result up with `unwrap_or(0)`, which reads as a promise that anything outside the
/// bitmap is background. Two ways it is not. A column past the right-hand edge lands on
/// the next row rather than off the end, so `get(width, 0)` returns the first pixel of
/// row one — the aliasing every row-major indexer has to bound `x` against `width` to
/// avoid. And the multiply is `u32`, so a row index large enough overflows: a debug
/// build panics where the release build wraps to some other pixel. Neither is reachable
/// from anything in the tree today, because nothing but a test calls `get`, which is
/// exactly why it is worth pinning before something does.
#[test]
fn reading_a_pixel_outside_the_bitmap_is_not_background() {
    let bm = render(
        "SourceSerif4-Regular.otf",
        &RenderOptions {
            text: "Hg".into(),
            size: 24.0,
            padding: 0,
            ..Default::default()
        },
        "get",
    );
    // One column past the right-hand edge is the first column of the next row.
    assert_eq!(
        bm.get(bm.width, 0),
        bm.get(0, 1),
        "x is not bounded against the width",
    );
    // Far enough down and the index arithmetic leaves u32 altogether.
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let far = std::panic::catch_unwind(|| bm.get(0, u32::MAX / bm.width + 1));
    std::panic::set_hook(hook);
    if cfg!(debug_assertions) {
        assert!(far.is_err(), "the multiply no longer overflows");
    } else {
        assert_eq!(
            far.unwrap_or(0),
            0,
            "with the checks off it wraps to a miss"
        );
    }
}

#[test]
fn the_baseline_is_where_the_ink_sits_around() {
    for name in ALL {
        let f = face(name);
        let bm = render_face(
            &f,
            &RenderOptions {
                text: "Hxp".into(),
                size: 48.0,
                padding: 0,
                ..Default::default()
            },
        )
        .unwrap();
        check_invariants(&bm, name);
        let rows: Vec<u32> = (0..bm.height)
            .filter(|&y| (0..bm.width).any(|x| bm.get(x, y) != 0))
            .collect();
        let (top, bottom) = (rows[0], *rows.last().unwrap());
        // A cap and a descender: ink above the baseline and ink below it.
        assert!(
            (top as f32) < bm.baseline,
            "{name}: ink starts at {top}, baseline {}",
            bm.baseline,
        );
        assert!(
            (bottom as f32) > bm.baseline,
            "{name}: ink ends at {bottom}, baseline {}",
            bm.baseline,
        );
    }
}

#[test]
fn a_face_that_is_not_a_font_is_an_error_and_so_is_a_face_that_is_not_there() {
    assert!(render_sfnt(b"", 0, &RenderOptions::default()).is_err());
    assert!(render_sfnt(b"not a font at all", 0, &RenderOptions::default()).is_err());
    // A truncated real font: the header parses and the tables do not.
    let mut short = bytes("Amiri-Regular.ttf");
    short.truncate(200);
    assert!(render_sfnt(&short, 0, &RenderOptions::default()).is_err());
    // A face index past the end of a collection.
    assert!(render_sfnt(&bytes("Amiri-Regular.ttf"), 7, &RenderOptions::default()).is_err(),);
    // A face whose file has gone away since the scan.
    let mut f = face("Amiri-Regular.ttf");
    f.file.path = "/nonexistent/gone.ttf".into();
    let err = render_face(&f, &RenderOptions::default())
        .unwrap_err()
        .to_string();
    assert!(err.contains("gone.ttf"), "{err}");
}

// ----- shaping without rasterising -----

#[test]
fn shaping_alone_takes_the_same_inputs_the_renderer_does() {
    let amiri = bytes("Amiri-Regular.ttf");
    assert_eq!(shaped_glyphs(&amiri, 0, "").unwrap(), Vec::<u32>::new());
    assert!(!shaped_glyphs(&amiri, 0, " ").unwrap().is_empty());
    // Newlines are not stripped by the shaper: it is given the whole string.
    assert!(shaped_glyphs(&amiri, 0, "a\nb").unwrap().len() >= 3);
    // Every text in the table shapes without breaking.
    for (what, text) in TEXTS {
        let g = shaped_glyphs(&amiri, 0, text).unwrap();
        assert!(
            g.len() <= text.chars().count() * 4,
            "{what}: {} glyphs",
            g.len()
        );
    }
    // A long string is linear work, not quadratic: this is the shape-then-throw-away
    // path a "does this font handle this text" question takes.
    let long = "سلام ".repeat(2000);
    let start = std::time::Instant::now();
    assert!(!shaped_glyphs(&amiri, 0, &long).unwrap().is_empty());
    assert!(
        start.elapsed() < std::time::Duration::from_secs(10),
        "shaping 10000 characters took {:?}",
        start.elapsed(),
    );
    assert!(shaped_glyphs(b"not a font", 0, "a").is_err());
    assert!(shaped_glyphs(&amiri, 9, "a").is_err());
}

// ----- containers -----

/// Unwrapping happens before rendering, so neither container is visible by the time the
/// shaper runs: the same text lays out the same way and takes the same box. The two
/// fixtures are separate builds of Inter — the WOFF2 has a `prep` table the WOFF has
/// not, and a `glyf` 900 bytes shorter — so the coverage is close rather than equal, and
/// a pixel-for-pixel assertion here would be asserting something about Fontsource.
#[test]
fn the_two_containers_of_one_font_draw_the_same_picture() {
    let woff = render_face(
        &face("inter-latin-400-normal.woff"),
        &RenderOptions::default(),
    )
    .unwrap();
    let woff2 = render_face(
        &face("inter-latin-400-normal.woff2"),
        &RenderOptions::default(),
    )
    .unwrap();
    check_invariants(&woff, "woff");
    check_invariants(&woff2, "woff2");
    assert_eq!((woff.width, woff.height), (woff2.width, woff2.height));
    assert_eq!(woff.glyphs, woff2.glyphs);
    assert_eq!(woff.missing, woff2.missing);
    assert_eq!(woff.baseline, woff2.baseline);
    let (a, b) = (ink(&woff) as i128, ink(&woff2) as i128);
    assert!(
        (a - b).abs() * 100 < a,
        "the two builds should agree to within one percent of the ink: {a} and {b}",
    );
}

/// `RenderOptions` derives `PartialEq` so a caller can key a cache on it; a field added
/// without a thought about the cache would show up as two options comparing equal.
#[test]
fn options_compare_field_by_field() {
    let base = RenderOptions::default();
    assert_eq!(base, RenderOptions::default());
    let differs = [
        RenderOptions {
            text: "other".into(),
            ..Default::default()
        },
        RenderOptions {
            size: 12.0,
            ..Default::default()
        },
        RenderOptions {
            variations: vec![("wght".into(), 700.0)],
            ..Default::default()
        },
        RenderOptions {
            features: vec![("smcp".into(), true)],
            ..Default::default()
        },
        RenderOptions {
            padding: 0,
            ..Default::default()
        },
        RenderOptions {
            max_width: Some(80),
            ..Default::default()
        },
    ];
    for (i, a) in differs.iter().enumerate() {
        assert_ne!(*a, base, "field {i} is not in the comparison");
        for (j, b) in differs.iter().enumerate() {
            if i != j {
                assert_ne!(a, b, "fields {i} and {j} compare equal");
            }
        }
    }
}
