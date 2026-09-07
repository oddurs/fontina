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

//! Self-contained HTML specimen: waterfall, script samples, variable axis sliders,
//! OpenType feature toggles, a glyph map by Unicode block, and side-by-side comparison
//! when several faces are given. Fonts are embedded as data URIs by default so the file
//! opens from disk in any browser (file:// font loads are blocked cross-origin otherwise).
//!
//! # The one rule the layout follows
//!
//! A specimen exists so a person can judge letterforms, which means the page must never
//! be mistaken for the type on it. The layout borrows the convention prepress already
//! settled on: guides are drawn in non-photo blue, the ink that does not reproduce. Every
//! rule, grid line, size marker and measurement here is blue, and the only black on the
//! page is the face being shown. That is also why nothing sets `font-smoothing`, applies a
//! text shadow, or otherwise touches rendering — a specimen that flatters a font lies
//! about it.
//!
//! Structurally that splits the sheet in two: a rail carrying the instrument (the
//! specifications, the axis sliders, the feature toggles) and a column carrying nothing
//! but type. The rail is sticky, so the controls stay within reach while the type column
//! scrolls past them.

use crate::error::{Error, Result};
use crate::model::FaceMetadata;
use crate::typography;
use base64::Engine;
use std::fmt::Write;

#[derive(Debug, Clone, Default)]
pub struct SpecimenOptions {
    /// Sample text shown in the waterfall and comparison. Defaults to a pangram.
    pub text: Option<String>,
    /// Reference font files by path instead of embedding them.
    pub link: bool,
    pub title: Option<String>,
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn mime(container: crate::Container) -> &'static str {
    match container {
        crate::Container::Woff => "font/woff",
        crate::Container::Woff2 => "font/woff2",
        crate::Container::Otf => "font/otf",
        _ => "font/ttf",
    }
}

fn src_for(face: &FaceMetadata, link: bool) -> Result<String> {
    let fragment = if face.file.face_count > 1 {
        format!("#{}", face.index)
    } else {
        String::new()
    };
    if link {
        // Through `css::file_url`, not by hand: a path is embedded here inside a
        // `<style>` element, and a directory named `</style>` would otherwise close it
        // and put the rest of the document's markup at the mercy of a file name.
        // Percent-encoding leaves no `<` to find.
        return Ok(format!(
            "url({}) format({})",
            crate::css::css_string(&format!(
                "{}{fragment}",
                crate::css::file_url(&face.file.path)
            )),
            crate::css::css_string(&face.style.css.format)
        ));
    }
    let bytes =
        std::fs::read(&face.file.path).map_err(|e| Error::Io(face.file.path.clone().into(), e))?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
    Ok(format!(
        "url({}) format({})",
        crate::css::css_string(&format!(
            "data:{};base64,{b64}{fragment}",
            mime(face.file.container)
        )),
        crate::css::css_string(&face.style.css.format)
    ))
}

/// Render the specimen document.
pub fn render(faces: &[FaceMetadata], opts: &SpecimenOptions) -> Result<String> {
    if faces.is_empty() {
        return Err(Error::Other("no faces to render".into()));
    }
    let text = opts
        .text
        .clone()
        .unwrap_or_else(|| typography::DEFAULT_TEXT.to_string());
    let title = opts.title.clone().unwrap_or_else(|| {
        if faces.len() == 1 {
            format!("{} {}", faces[0].names.family, faces[0].names.subfamily)
        } else {
            format!("{} faces", faces.len())
        }
    });

    let mut h = String::with_capacity(64 * 1024);
    h.push_str("<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    writeln!(h, "<title>{}</title>", esc(&title)).ok();
    writeln!(
        h,
        "<meta name=\"generator\" content=\"fontina {}\">",
        env!("CARGO_PKG_VERSION")
    )
    .ok();
    h.push_str("<style>\n");
    for (i, f) in faces.iter().enumerate() {
        let css = &f.style.css;
        writeln!(h, "@font-face{{font-family:\"uf{i}\";font-weight:{};font-stretch:{};font-style:{};src:{};}}", css.weight, css.stretch, css.style, src_for(f, opts.link)?).ok();
    }
    h.push_str(CSS);
    h.push_str("</style>\n</head>\n<body>\n");

    bar(&mut h, &title, &text);
    h.push_str("<main class=\"page\">\n");
    if faces.len() > 1 {
        index(&mut h, faces);
        compare(&mut h, faces, &text);
    }
    for (i, f) in faces.iter().enumerate() {
        sheet(&mut h, i, f, &text);
    }
    h.push_str("</main>\n");
    h.push_str("<footer class=\"page\">Generated by fontina. Fonts remain under their own licenses.</footer>\n");
    h.push_str("<script>\n");
    h.push_str(JS);
    h.push_str("</script>\n</body>\n</html>\n");
    Ok(h)
}

/// The one bar of chrome on the page: what is being shown, and the two settings that
/// apply to every face at once.
fn bar(h: &mut String, title: &str, text: &str) {
    h.push_str("<header class=\"bar\">\n");
    writeln!(h, "<h1>{}</h1>", esc(title)).ok();
    h.push_str("<div class=\"bar-set\">");
    writeln!(
        h,
        "<label class=\"field text\"><span>Sample text</span><input id=\"text\" value=\"{}\" spellcheck=\"false\" autocomplete=\"off\"></label>",
        esc(text)
    )
    .ok();
    h.push_str("<label class=\"field\"><span>Size</span><input id=\"size\" type=\"range\" min=\"8\" max=\"200\" value=\"48\"><output id=\"sizeout\">48<i>px</i></output></label>");
    h.push_str("<button type=\"button\" id=\"print\">Print</button>");
    h.push_str("</div>\n</header>\n");
}

/// Jump links when there is more than one face, each set in the face it points at, so the
/// index is itself a comparison.
fn index(h: &mut String, faces: &[FaceMetadata]) {
    h.push_str("<nav class=\"index\" aria-label=\"Faces on this sheet\">");
    for (i, f) in faces.iter().enumerate() {
        write!(
            h,
            "<a href=\"#face{i}\" style=\"font-family:'uf{i}'\">{}<i>{}</i></a>",
            esc(&f.names.family),
            esc(&f.names.subfamily)
        )
        .ok();
    }
    h.push_str("</nav>\n");
}

/// Every face setting the same words at the same size, on a shared baseline grid. It goes
/// first because it is the question a reader with several fonts open actually has.
fn compare(h: &mut String, faces: &[FaceMetadata], text: &str) {
    h.push_str("<section class=\"compare\">\n<h2>Side by side</h2>\n");
    for (i, f) in faces.iter().enumerate() {
        write!(
            h,
            "<div class=\"cmp\"><a class=\"cmp-name\" href=\"#face{i}\">{}<i>{}</i></a><div class=\"sample type js-text js-size\" style=\"font-family:'uf{i}'\" data-face=\"{i}\">{}</div></div>",
            esc(&f.names.family),
            esc(&f.names.subfamily),
            esc(text)
        )
        .ok();
    }
    h.push_str("</section>\n");
}

/// One face: the head where it names itself, the rail of specifications and controls, and
/// the column of type.
fn sheet(h: &mut String, i: usize, f: &FaceMetadata, text: &str) {
    let n = &f.names;
    writeln!(
        h,
        "<article class=\"face\" id=\"face{i}\" data-face=\"{i}\">"
    )
    .ok();
    write!(
        h,
        "<header class=\"head\"><h2 class=\"name type\" style=\"font-family:'uf{i}'\">{}</h2><p class=\"style type\" style=\"font-family:'uf{i}'\">{}</p><p class=\"lead type js-text js-size\" style=\"font-family:'uf{i}'\">{}</p></header>\n<div class=\"sheet\">\n",
        esc(&n.family),
        esc(&n.subfamily),
        esc(text)
    )
    .ok();

    h.push_str("<aside class=\"rail\">\n");
    spec(h, f);
    controls(h, f);
    h.push_str("</aside>\n<div class=\"column\">\n");

    waterfall(h, i, text);
    scripts(h, i, f);
    glyph_map(h, i, f);
    h.push_str("</div>\n</div>\n</article>\n");
}

/// What the font says about itself, as a specification list rather than a paragraph: a
/// reader scans this for one value, never reads it through.
fn spec(h: &mut String, f: &FaceMetadata) {
    let n = &f.names;
    h.push_str("<dl class=\"spec\">");
    let mut row = |k: &str, v: Option<&str>| {
        if let Some(v) = v.filter(|v| !v.is_empty()) {
            write!(h, "<dt>{}</dt><dd>{}</dd>", esc(k), esc(v)).ok();
        }
    };
    row("PostScript name", n.postscript_name.as_deref());
    row("Version", n.version.as_deref());
    row("Designer", n.designer.as_deref());
    row("Vendor", n.manufacturer.as_deref());
    row("License", f.license.spdx.as_deref());
    row(
        "Glyphs",
        Some(&format!(
            "{} glyphs, {} codepoints",
            f.glyph_count, f.coverage.codepoints
        )),
    );
    let scripts = f
        .coverage
        .scripts
        .iter()
        .take(6)
        .map(|s| format!("{} {}", s.script, s.codepoints))
        .collect::<Vec<_>>()
        .join(", ");
    row("Scripts", Some(&scripts));
    let file = std::path::Path::new(&f.file.path)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    row("File", Some(&file));
    h.push_str("</dl>\n");
}

/// The axis sliders and the feature toggles — everything on the page a reader can change
/// about this one face.
fn controls(h: &mut String, f: &FaceMetadata) {
    if let Some(v) = &f.variable {
        h.push_str("<section class=\"controls\"><h3>Variable axes<button type=\"button\" class=\"reset\">Reset</button></h3>\n");
        for a in &v.axes {
            let label = a.name.clone().unwrap_or_else(|| a.tag.clone());
            let step = typography::axis_step(a);
            write!(
                h,
                "<label class=\"axis-row\"><span class=\"axis-name\">{}<code>{}</code></span><input type=\"range\" class=\"axis\" data-tag=\"{}\" data-default=\"{}\" min=\"{}\" max=\"{}\" step=\"{step}\" value=\"{}\"><output>{}</output></label>",
                esc(&label),
                esc(&a.tag),
                esc(&a.tag),
                a.default,
                a.min,
                a.max,
                a.default,
                a.default
            )
            .ok();
        }
        if !v.instances.is_empty() {
            h.push_str("<label class=\"axis-row select\"><span class=\"axis-name\">Named instance</span><select class=\"instance\"><option value=\"\">Custom</option>");
            for inst in &v.instances {
                let coords = inst
                    .coordinates
                    .iter()
                    .map(f32::to_string)
                    .collect::<Vec<_>>()
                    .join(",");
                write!(
                    h,
                    "<option value=\"{coords}\">{}</option>",
                    esc(inst.name.as_deref().unwrap_or("?"))
                )
                .ok();
            }
            h.push_str("</select></label>");
        }
        h.push_str("</section>\n");
    }

    let toggles = typography::toggleable_features(&f.features);
    if !toggles.is_empty() {
        h.push_str("<section class=\"controls\"><h3>Features</h3><div class=\"chips\">");
        for t in toggles {
            let label = typography::feature_label(t);
            write!(
                h,
                "<label class=\"chip\" title=\"{}\"><input type=\"checkbox\" class=\"feat\" data-tag=\"{}\"><code>{}</code><span>{}</span></label>",
                esc(&label),
                esc(t),
                esc(t),
                esc(&label)
            )
            .ok();
        }
        h.push_str("</div></section>\n");
    }
}

/// The size ladder. Each rung is one line, clipped rather than wrapped, with its size
/// marked in the guide column so the marks line up down the page.
fn waterfall(h: &mut String, i: usize, text: &str) {
    h.push_str("<section class=\"waterfall\">\n");
    for size in typography::WATERFALL_SIZES {
        write!(
            h,
            "<div class=\"wf-row\"><span class=\"pt\">{size}</span><div class=\"wf type js-text\" style=\"font-family:'uf{i}';font-size:{size}px\">{}</div></div>",
            esc(text)
        )
        .ok();
    }
    h.push_str("\n</section>\n");
}

/// A paragraph per script the face actually covers, in that script's own words and its own
/// direction. Capped at three: past that it is a coverage report, not a specimen.
fn scripts(h: &mut String, i: usize, f: &FaceMetadata) {
    let mut shown = 0;
    for sc in &f.coverage.scripts {
        if sc.codepoints < 20 {
            continue;
        }
        let Some((dir, sample)) = typography::script_sample(&sc.script) else {
            continue;
        };
        if shown == 0 {
            h.push_str("<section class=\"paras\">\n");
        }
        write!(
            h,
            "<p class=\"para type\" dir=\"{}\" style=\"font-family:'uf{i}'\">{}</p>",
            dir.as_str(),
            esc(sample)
        )
        .ok();
        shown += 1;
        if shown == 3 {
            break;
        }
    }
    if shown > 0 {
        h.push_str("\n</section>\n");
    }
}

/// Every codepoint the face covers, by Unicode block, drawn on a grid. The blocks are
/// closed by default past the first two because a font with full coverage would otherwise
/// open on tens of thousands of cells; each summary carries how much of its block is
/// covered so the closed ones still say something.
fn glyph_map(h: &mut String, i: usize, f: &FaceMetadata) {
    let blocks = crate::unicode::glyph_map(&f.coverage.ranges);
    if blocks.is_empty() {
        return;
    }
    h.push_str("<section class=\"glyphs\"><h3>Glyph map</h3>\n");
    for (bi, b) in blocks.iter().enumerate() {
        let open = if bi < 2 { " open" } else { "" };
        let cps = b
            .codepoints
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let have = b.codepoints.len();
        let pct = if b.block_size > 0 {
            (have as f64 * 100.0 / f64::from(b.block_size)).round()
        } else {
            0.0
        };
        write!(
            h,
            "<details class=\"block\"{open} data-cps=\"{cps}\" data-font=\"uf{i}\"><summary><span class=\"block-name\">{}</span><span class=\"meter\" aria-hidden=\"true\"><i style=\"width:{pct}%\"></i></span><span class=\"count\">{have}<i>/{}</i></span></summary><div class=\"grid\"></div></details>",
            esc(&b.block),
            b.block_size
        )
        .ok();
    }
    h.push_str("</section>\n");
}

/// The stylesheet. Non-photo blue (`--guide`) draws everything that measures the type;
/// blue pencil (`--mark`) draws everything that labels or responds to a person. `--ink` is
/// reserved for the face itself, which is why no rule, marker or control uses it.
const CSS: &str = r#"
:root{
  color-scheme:light dark;
  --paper:#f4f4f1;--sheet:#fff;--field:#fff;
  --ink:#14141a;--ink-2:#5f636e;
  --guide:#bfe0f0;--mark:#0e6d94;
  --bar:56px;--rail:220px;--gap:44px;
}
@media(prefers-color-scheme:dark){:root{--paper:#0f1116;--sheet:#171a21;--field:#0b0d11;--ink:#e9ecf1;--ink-2:#89909e;--guide:#26404e;--mark:#68c8e9}}
*,*::before,*::after{box-sizing:border-box}
html{scroll-behavior:smooth}
@media(prefers-reduced-motion:reduce){html{scroll-behavior:auto}*{animation-duration:.01ms!important;transition-duration:.01ms!important}}
/* No font-smoothing, no text-shadow, no letter-spacing on the samples: a specimen that
   flatters a face lies about it. */
body{margin:0;background:var(--paper);color:var(--ink);font:400 13.5px/1.55 ui-sans-serif,system-ui,-apple-system,"Segoe UI",Roboto,sans-serif}
.page{max-width:1160px;margin:0 auto;padding:0 clamp(20px,4vw,44px)}
:focus-visible{outline:2px solid var(--mark);outline-offset:2px;border-radius:2px}
code{font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace}

.bar{position:sticky;top:0;z-index:5;display:flex;flex-wrap:wrap;gap:10px 28px;align-items:center;min-height:var(--bar);padding:9px clamp(20px,4vw,44px);background:var(--paper);border-bottom:1px solid var(--guide)}
.bar h1{flex:0 1 auto;min-width:0;margin:0;font-size:13.5px;font-weight:600;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.bar-set{display:flex;flex:1 1 400px;flex-wrap:wrap;gap:10px 20px;align-items:center;justify-content:flex-end}
.field{display:flex;gap:9px;align-items:center;color:var(--ink-2);white-space:nowrap}
.field.text{flex:1 1 20em;min-width:11em}
#text{flex:1;min-width:0;font:inherit;color:var(--ink);background:var(--field);border:1px solid var(--guide);border-radius:3px;padding:5px 9px}
#text:focus{border-color:var(--mark);outline:none}
#size{width:130px}
input[type=range]{accent-color:var(--mark)}
output{color:var(--mark);font-variant-numeric:tabular-nums}
#sizeout{min-width:4.2em}
output i{font-style:normal;color:var(--ink-2);font-size:.85em;margin-left:.15em}
#print{font:inherit;color:var(--ink);background:transparent;border:1px solid var(--guide);border-radius:3px;padding:5px 12px;cursor:pointer}
#print:hover{border-color:var(--mark);color:var(--mark)}

.index{display:flex;flex-wrap:wrap;gap:0 30px;padding-top:24px}
.index a{color:var(--ink);text-decoration:none;font-size:20px;line-height:1.3;padding:3px 0;border-bottom:1px solid transparent}
.index a i{font-style:normal;font-family:ui-sans-serif,system-ui,sans-serif;color:var(--ink-2);font-size:.6em;margin-left:.6em}
.index a:hover{border-bottom-color:var(--mark)}

.compare{padding:26px 0 32px}
.compare h2{margin:0 0 12px;font-size:12px;font-weight:600;color:var(--mark)}
.cmp{display:grid;grid-template-columns:var(--rail) minmax(0,1fr);gap:var(--gap);align-items:baseline;padding:14px 0;border-top:1px solid var(--guide)}
.cmp-name{color:var(--ink);text-decoration:none;font-size:12.5px;line-height:1.45}
.cmp-name i{font-style:normal;display:block;color:var(--ink-2)}
.cmp-name:hover{color:var(--mark)}
.sample{font-size:48px;line-height:1.15;white-space:nowrap;overflow-x:auto;overflow-y:hidden;scrollbar-width:thin}

.face{padding:42px 0 10px;scroll-margin-top:calc(var(--bar) + 10px)}
.face+.face,.compare+.face{border-top:1px solid var(--guide)}
.head{margin:0 0 32px}
.name{margin:0;font-weight:normal;font-size:clamp(2.6rem,7.5vw,5.4rem);line-height:1.02;overflow-wrap:anywhere}
.style{margin:.3em 0 0;font-weight:normal;font-size:clamp(1rem,2.2vw,1.45rem);line-height:1.2;color:var(--ink-2)}
.sheet{display:grid;grid-template-columns:var(--rail) minmax(0,1fr)}
.rail{position:sticky;top:calc(var(--bar) + 18px);align-self:start;max-height:calc(100vh - var(--bar) - 36px);overflow-y:auto;overscroll-behavior:contain;scrollbar-width:thin;padding-right:calc(var(--gap) - 6px)}
.column{min-width:0;border-left:1px solid var(--guide);padding-left:var(--gap)}

.spec{margin:0 0 26px;font-size:12.5px}
.spec dt{margin-top:11px;color:var(--ink-2);line-height:1.3}
.spec dt:first-child{margin-top:0}
.spec dd{margin:1px 0 0;line-height:1.4;overflow-wrap:anywhere}

.controls{margin:0 0 24px}
.controls h3{display:flex;align-items:center;justify-content:space-between;gap:10px;margin:0 0 11px;font-size:12px;font-weight:600;color:var(--mark)}
.reset{font:inherit;font-size:11.5px;color:var(--ink-2);background:none;border:0;border-bottom:1px solid var(--guide);padding:1px 2px;cursor:pointer}
.reset:hover{color:var(--mark);border-bottom-color:var(--mark)}
.axis-row{display:grid;grid-template-columns:minmax(0,1fr) auto;gap:1px 8px;margin:0 0 13px;font-size:12px}
.axis-name{display:flex;gap:6px;align-items:baseline;min-width:0;color:var(--ink-2)}
.axis-name code{font-size:11px;color:var(--mark)}
.axis-row input[type=range]{grid-column:1/-1;width:100%;margin:3px 0 0}
.axis-row output{grid-row:1;grid-column:2;justify-self:end;color:var(--ink)}
.axis-row.select{display:block}
.instance{width:100%;margin-top:5px;font:inherit;font-size:12px;color:var(--ink);background:var(--field);border:1px solid var(--guide);border-radius:3px;padding:4px 6px}
.chips{display:flex;flex-wrap:wrap;gap:5px}
.chip{position:relative;display:inline-flex;align-items:baseline;gap:5px;max-width:100%;padding:3px 8px;border:1px solid var(--guide);border-radius:3px;font-size:11.5px;color:var(--ink-2);cursor:pointer}
.chip input{position:absolute;inset:0;width:100%;height:100%;margin:0;opacity:0;cursor:pointer}
.chip code{font-size:11px;color:var(--mark)}
.chip>span{overflow:hidden;white-space:nowrap;text-overflow:ellipsis}
.chip:hover{border-color:var(--mark)}
.chip.on{background:var(--mark);border-color:var(--mark);color:var(--sheet)}
.chip.on code{color:var(--sheet);opacity:.7}
.chip:focus-within{outline:2px solid var(--mark);outline-offset:2px}

.lead{margin:26px 0 0;padding-top:24px;border-top:1px solid var(--guide);font-size:48px;line-height:1.18;overflow-wrap:anywhere}
.waterfall{border-top:1px solid var(--guide)}
.wf-row{display:grid;grid-template-columns:3.4ch minmax(0,1fr);gap:14px;align-items:baseline;padding:6px 0}
.wf-row+.wf-row{border-top:1px solid var(--guide)}
.pt{font-size:10.5px;line-height:1;text-align:right;color:var(--mark);font-variant-numeric:tabular-nums}
.wf{min-width:0;line-height:1.16;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}

.paras{margin-top:36px}
.para{margin:0 0 22px;font-size:19px;line-height:1.62;max-width:62ch;overflow-wrap:break-word}
.para[dir=rtl]{margin-left:auto;text-align:right}

.glyphs{margin-top:38px}
.glyphs h3{margin:0 0 10px;font-size:12px;font-weight:600;color:var(--mark)}
.block{border-top:1px solid var(--guide)}
.block:last-child{border-bottom:1px solid var(--guide)}
.block summary{display:grid;grid-template-columns:1.1em minmax(0,1fr) 84px auto;gap:14px;align-items:center;padding:9px 2px;font-size:12.5px;cursor:pointer;list-style:none}
.block summary::-webkit-details-marker{display:none}
.block summary::before{content:"+";color:var(--mark);text-align:center;line-height:1}
.block[open] summary::before{content:"\2212"}
.block summary:hover{color:var(--mark)}
.block-name{overflow:hidden;white-space:nowrap;text-overflow:ellipsis}
.meter{position:relative;height:3px;border-radius:2px;background:var(--guide);overflow:hidden}
.meter i{position:absolute;inset:0 auto 0 0;background:var(--mark)}
.count{font-size:11.5px;text-align:right;white-space:nowrap;color:var(--ink-2);font-variant-numeric:tabular-nums}
.count i{font-style:normal;opacity:.55}
.grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(54px,1fr));margin:2px 0 20px;background:var(--sheet);border-top:1px solid var(--guide);border-left:1px solid var(--guide)}
.g{display:flex;align-items:center;justify-content:center;position:relative;aspect-ratio:1;margin:0;padding:0 0 11px;border:0;border-right:1px solid var(--guide);border-bottom:1px solid var(--guide);overflow:hidden;background:var(--sheet);color:var(--ink);font:inherit;font-size:26px;line-height:1;cursor:pointer;appearance:none;-webkit-appearance:none}
.g::after{content:attr(data-u);position:absolute;left:0;right:0;bottom:3px;text-align:center;font:400 8.5px/1 ui-sans-serif,system-ui,sans-serif;font-variant-numeric:tabular-nums;color:var(--mark);opacity:.5}
.g:hover{background:var(--guide)}
.g:hover::after{opacity:1}
.g:focus-visible{outline:2px solid var(--mark);outline-offset:-2px}

footer.page{margin-top:28px;padding-block:26px 46px;border-top:1px solid var(--guide);color:var(--ink-2);font-size:11.5px}

@media(max-width:860px){
  .sheet,.cmp{grid-template-columns:minmax(0,1fr)}
  .cmp{gap:6px}
  .rail{position:static;max-height:none;overflow:visible;padding-right:0;margin-bottom:26px}
  .column{border-left:0;border-top:1px solid var(--guide);padding-left:0;padding-top:24px}
  .lead{font-size:34px}
  .sample{font-size:34px}
}

/* Non-photo blue is the ink that does not reproduce, so on paper the guides drop to a
   grey that is still there to read against. */
@media print{
  :root{--paper:#fff;--sheet:#fff;--field:#fff;--ink:#000;--ink-2:#444;--guide:#ccc;--mark:#666}
  body{font-size:10pt}
  .bar{position:static}
  .bar-set,.index,.reset,.axis-row input[type=range],.axis-row.select,.chip input,.block:not([open]){display:none}
  .sheet,.cmp{grid-template-columns:minmax(0,1fr)}
  .rail{position:static;max-height:none;overflow:visible;padding-right:0}
  .column{border-left:0;padding-left:0}
  .wf-row,.cmp,.block,.para,.head{break-inside:avoid}
  .face{break-before:page}
  .face:first-of-type{break-before:auto}
  .g{cursor:default}
  a{color:inherit;text-decoration:none}
}
"#;

const JS: &str = r#"
const $ = (s, r = document) => r.querySelector(s);
const $$ = (s, r = document) => [...r.querySelectorAll(s)];
const textInput = $('#text'), sizeInput = $('#size'), sizeOut = $('#sizeout');

// Every element that sets the reader's words, and every element the size slider drives.
function applyText(){ const t = textInput.value; $$('.js-text').forEach(el => { el.textContent = t; }); }
function applySize(){ const s = sizeInput.value; sizeOut.firstChild.nodeValue = s; $$('.js-size').forEach(el => { el.style.fontSize = s + 'px'; }); }
function initSize(){
  const el = $('.js-size');
  if (el) { const px = Math.round(parseFloat(getComputedStyle(el).fontSize)); if (px > 0) sizeInput.value = px; }
  sizeOut.firstChild.nodeValue = sizeInput.value;
}

textInput.addEventListener('input', applyText);
sizeInput.addEventListener('input', applySize);
$('#print').addEventListener('click', () => window.print());
initSize();

const bar = $('.bar');
const measureBar = () => document.documentElement.style.setProperty('--bar', bar.offsetHeight + 'px');
if (window.ResizeObserver) new ResizeObserver(measureBar).observe(bar); else addEventListener('resize', measureBar);
measureBar();

$$('article.face').forEach(art => {
  const i = art.dataset.face;
  const axes = $$('.axis', art), feats = $$('.feat', art), inst = $('.instance', art), reset = $('.reset', art);
  // Recomputed on every call: the glyph cells are built when a block is first opened.
  const targets = () => [...$$('.type', art), ...$$('.cmp .type[data-face="' + i + '"]')];

  // Coordinates that sit exactly on a named instance select it; anything else is Custom.
  // Exact, like `typography::matching_instance`: coordinates that are merely close are a
  // setting the reader chose, and calling that "Bold" would be a lie.
  function named(){ if (!inst) return ''; const now = axes.map(a => a.value).join(','); return [...inst.options].some(o => o.value === now) ? now : ''; }
  function applyVar(){
    const v = axes.map(a => '"' + a.dataset.tag + '" ' + a.value).join(', ');
    targets().forEach(el => { el.style.fontVariationSettings = v; });
    axes.forEach(a => { a.nextElementSibling.textContent = a.value; });
    if (inst && !inst.dataset.lock) inst.value = named();
  }
  function applyFeat(){
    const on = feats.filter(f => f.checked).map(f => '"' + f.dataset.tag + '" 1').join(', ');
    targets().forEach(el => { el.style.fontFeatureSettings = on || 'normal'; });
  }

  axes.forEach(a => a.addEventListener('input', applyVar));
  feats.forEach(f => f.addEventListener('change', () => { f.closest('.chip').classList.toggle('on', f.checked); applyFeat(); }));
  if (reset) reset.addEventListener('click', () => { axes.forEach(a => { a.value = a.dataset.default; }); applyVar(); });
  if (inst) inst.addEventListener('change', () => {
    if (!inst.value) return;
    const c = inst.value.split(',');
    inst.dataset.lock = '1';
    axes.forEach((a, k) => { if (c[k] !== undefined) a.value = c[k]; });
    applyVar();
    delete inst.dataset.lock;
  });

  $$('details.block', art).forEach(d => {
    const build = () => {
      const g = $('.grid', d);
      if (g.childElementCount) return;
      const font = d.dataset.font, frag = document.createDocumentFragment();
      for (const cp of d.dataset.cps.split(',')) {
        const n = +cp, b = document.createElement('button');
        b.type = 'button';
        b.className = 'g type';
        b.tabIndex = -1;
        b.style.fontFamily = "'" + font + "'";
        b.textContent = String.fromCodePoint(n);
        b.dataset.u = n.toString(16).toUpperCase().padStart(4, '0');
        b.title = 'U+' + b.dataset.u + ' — add to the sample text';
        b.tabIndex = frag.childElementCount ? -1 : 0;
        frag.appendChild(b);
      }
      g.appendChild(frag);
      rove(g);
      if (axes.length) applyVar();
      if (feats.length) applyFeat();
    };
    if (d.open) build();
    d.addEventListener('toggle', () => { if (d.open) build(); });
  });

  if (axes.length) applyVar();
});

// A grid of six hundred buttons must not be six hundred tab stops, so each block is
// one stop and the arrow keys move inside it.
function rove(grid){
  grid.addEventListener('keydown', e => {
    const cells = [...grid.children], i = cells.indexOf(document.activeElement);
    if (i < 0) return;
    const w = cells[0].offsetWidth || 1, per = Math.max(1, Math.round(grid.clientWidth / w));
    const step = {ArrowRight: 1, ArrowLeft: -1, ArrowDown: per, ArrowUp: -per, Home: -i, End: cells.length - 1 - i}[e.key];
    if (step === undefined) return;
    const next = cells[Math.min(cells.length - 1, Math.max(0, i + step))];
    if (!next || next === cells[i]) return;
    e.preventDefault();
    cells[i].tabIndex = -1; next.tabIndex = 0; next.focus();
  });
}

// A glyph you click is a glyph you wanted to see set. Appending beats a clipboard call,
// which a page opened from a file:// URL is not always allowed to make.
document.addEventListener('click', e => {
  const g = e.target.closest('.g');
  if (!g) return;
  textInput.value += g.textContent;
  applyText();
});
"#;
