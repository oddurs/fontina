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

use crate::error::{Error, Result};
use crate::model::FaceMetadata;
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

const DEFAULT_TEXT: &str = "Sphinx of black quartz, judge my vow. 0123456789";

/// Script-specific sample paragraphs keyed by ISO 15924 code, with direction.
const SAMPLES: &[(&str, &str, &str)] = &[
    (
        "Latn",
        "ltr",
        "The quick brown fox jumps over the lazy dog. Zwölf Boxkämpfer jagen Viktor quer über den großen Sylter Deich. Portez ce vieux whisky au juge blond qui fume. Árvíztűrő tükörfúrógép.",
    ),
    (
        "Cyrl",
        "ltr",
        "Съешь же ещё этих мягких французских булок, да выпей чаю. Жебракують філософи при ґанку церкви в Гадячі, ще й шатро їхнє п'яне знаємо.",
    ),
    (
        "Grek",
        "ltr",
        "Ξεσκεπάζω την ψυχοφθόρα βδελυγμία. Τάχιστη αλώπηξ βαφής ψημένη γη, δρασκελίζει υπέρ νωθρού κυνός.",
    ),
    (
        "Arab",
        "rtl",
        "صِف خَلقَ خَودِ كَمِثلِ الشَمسِ إِذ بَزَغَت يَحظى الضَجيعُ بِها نَجلاءَ مِعطارِ. نص حكيم له سر قاطع وذو شأن عظيم مكتوب على ثوب أخضر ومغلف بجلد أزرق.",
    ),
    (
        "Hebr",
        "rtl",
        "דג סקרן שט בים מאוכזב ולפתע מצא חברה. עטלף אבק נס דרך מזגן שהתפוצץ כי חם.",
    ),
    (
        "Deva",
        "ltr",
        "ऋषियों को सताने वाले दुष्ट राक्षसों के राजा रावण का सर्वनाश करने वाले विष्णुवतार भगवान श्रीराम, अयोध्या के महाराज दशरथ के बड़े सपुत्र थे।",
    ),
    (
        "Beng",
        "ltr",
        "আমি বাংলায় গান গাই, আমি বাংলার গান গাই। আমি আমার আমিকে চিরদিন এই বাংলায় খুঁজে পাই।",
    ),
    ("Taml", "ltr", "யாதும் ஊரே யாவரும் கேளிர் தீதும் நன்றும் பிறர்தர வாரா."),
    (
        "Thai",
        "ltr",
        "เป็นมนุษย์สุดประเสริฐเลิศคุณค่า กว่าบรรดาฝูงสัตว์เดรัจฉาน จงฝ่าฟันพัฒนาวิชาการ",
    ),
    (
        "Hani",
        "ltr",
        "視野無限廣，窗外有藍天。天地玄黃，宇宙洪荒。日月盈昃，辰宿列張。",
    ),
    (
        "Hira",
        "ltr",
        "いろはにほへと ちりぬるを わかよたれそ つねならむ うゐのおくやま けふこえて",
    ),
    (
        "Kana",
        "ltr",
        "イロハニホヘト チリヌルヲ ワカヨタレソ ツネナラム",
    ),
    (
        "Hang",
        "ltr",
        "키스의 고유조건은 입술끼리 만나야 하고 특별한 기술은 필요치 않다.",
    ),
    (
        "Geor",
        "ltr",
        "გთხოვთ ახლავე გაიაროთ რეგისტრაცია უნიკოდის მეათე საერთაშორისო კონფერენციაზე.",
    ),
    (
        "Armn",
        "ltr",
        "Բել դղյակի ձախ ժամն օֆ ազգությանը ցպահանջ չճշտած վնաս էր և փառք։",
    ),
    ("Ethi", "ltr", "ሰማይ አይታረስ ንጉሥ አይከሰስ። ብላ ካለኝ እንደአባቴ በቆመጠኝ።"),
];

/// Human labels for common OpenType features.
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

/// Features that are on by default or required for correct shaping; not offered as toggles.
const HIDDEN_FEATURES: &[&str] = &[
    "ccmp", "locl", "rlig", "rclt", "init", "medi", "fina", "isol", "mark", "mkmk", "curs", "abvm",
    "blwm", "abvs", "blws", "pres", "psts", "pref", "half", "nukt", "akhn", "rphf", "vatu", "cjct",
    "haln", "dist", "rvrn", "req", "dnom", "numr", "rtlm", "ltra", "ltrm", "rtla", "ordn", "aalt",
    "vert", "vrt2",
];

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
        return Ok(format!(
            "url(\"file://{}{}\") format(\"{}\")",
            face.file.path.replace('\\', "/"),
            fragment,
            face.style.css.format
        ));
    }
    let bytes =
        std::fs::read(&face.file.path).map_err(|e| Error::Io(face.file.path.clone().into(), e))?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
    Ok(format!(
        "url(\"data:{};base64,{}{}\") format(\"{}\")",
        mime(face.file.container),
        b64,
        fragment,
        face.style.css.format
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
        .unwrap_or_else(|| DEFAULT_TEXT.to_string());
    let title = opts.title.clone().unwrap_or_else(|| {
        if faces.len() == 1 {
            format!("{} {}", faces[0].names.family, faces[0].names.subfamily)
        } else {
            format!("{} faces", faces.len())
        }
    });
    let mut h = String::with_capacity(64 * 1024);
    writeln!(h, "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>{}</title>", esc(&title)).ok();
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
    writeln!(h, "<header class=\"top\"><h1>{}</h1><label>Sample text <input id=\"text\" value=\"{}\"></label><label>Size <input id=\"size\" type=\"range\" min=\"8\" max=\"160\" value=\"48\"> <output id=\"sizeout\">48</output>px</label><button id=\"print\" onclick=\"window.print()\">Print / PDF</button></header>", esc(&title), esc(&text)).ok();

    if faces.len() > 1 {
        h.push_str("<section class=\"compare\"><h2>Compare</h2>\n");
        for (i, f) in faces.iter().enumerate() {
            writeln!(h, "<div class=\"cmp\"><div class=\"cmp-label\">{} {}</div><div class=\"sample js-sample\" style=\"font-family:'uf{i}'\" data-face=\"{i}\">{}</div></div>", esc(&f.names.family), esc(&f.names.subfamily), esc(&text)).ok();
        }
        h.push_str("</section>\n");
    }

    for (i, f) in faces.iter().enumerate() {
        let n = &f.names;
        writeln!(
            h,
            "<article class=\"face\" id=\"face{i}\" data-face=\"{i}\">"
        )
        .ok();
        writeln!(
            h,
            "<h2 style=\"font-family:'uf{i}'\">{} <span class=\"sub\">{}</span></h2>",
            esc(&n.family),
            esc(&n.subfamily)
        )
        .ok();
        h.push_str("<dl class=\"meta\">");
        let mut meta = |k: &str, v: Option<&str>| {
            if let Some(v) = v {
                write!(h, "<dt>{}</dt><dd>{}</dd>", esc(k), esc(v)).ok();
            }
        };
        meta("PostScript", n.postscript_name.as_deref());
        meta("Version", n.version.as_deref());
        meta("Designer", n.designer.as_deref());
        meta("Vendor", n.manufacturer.as_deref());
        meta("License", f.license.spdx.as_deref());
        let glyphs = format!(
            "{} glyphs, {} codepoints",
            f.glyph_count, f.coverage.codepoints
        );
        meta("Glyphs", Some(&glyphs));
        let scripts = f
            .coverage
            .scripts
            .iter()
            .take(6)
            .map(|s| format!("{} {}", s.script, s.codepoints))
            .collect::<Vec<_>>()
            .join(", ");
        meta("Scripts", Some(&scripts));
        let file = std::path::Path::new(&f.file.path)
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        meta("File", Some(&file));
        h.push_str("</dl>\n");

        if let Some(v) = &f.variable {
            h.push_str("<div class=\"controls axes\">");
            for a in &v.axes {
                let label = a.name.clone().unwrap_or_else(|| a.tag.clone());
                let step = if a.max - a.min > 50.0 { "1" } else { "0.1" };
                write!(h, "<label><span>{} <code>{}</code></span><input type=\"range\" class=\"axis\" data-tag=\"{}\" min=\"{}\" max=\"{}\" step=\"{}\" value=\"{}\"><output>{}</output></label>", esc(&label), esc(&a.tag), esc(&a.tag), a.min, a.max, step, a.default, a.default).ok();
            }
            if !v.instances.is_empty() {
                h.push_str("<label><span>Instance</span><select class=\"instance\"><option value=\"\">Custom</option>");
                for inst in &v.instances {
                    let coords = inst
                        .coordinates
                        .iter()
                        .map(|c| c.to_string())
                        .collect::<Vec<_>>()
                        .join(",");
                    write!(
                        h,
                        "<option value=\"{}\">{}</option>",
                        coords,
                        esc(inst.name.as_deref().unwrap_or("?"))
                    )
                    .ok();
                }
                h.push_str("</select></label>");
            }
            h.push_str("</div>\n");
        }

        let toggles: Vec<&String> = f
            .features
            .gsub
            .iter()
            .filter(|t| !HIDDEN_FEATURES.contains(&t.as_str()))
            .collect();
        if !toggles.is_empty() {
            h.push_str("<div class=\"controls features\">");
            for t in toggles {
                let label = FEATURE_LABELS
                    .iter()
                    .find(|(k, _)| k == t)
                    .map(|(_, l)| *l)
                    .unwrap_or("");
                write!(h, "<label><input type=\"checkbox\" class=\"feat\" data-tag=\"{}\"><code>{}</code> {}</label>", esc(t), esc(t), esc(label)).ok();
            }
            h.push_str("</div>\n");
        }

        h.push_str("<div class=\"waterfall\">");
        for size in [10, 12, 14, 18, 24, 32, 48, 72, 96] {
            write!(h, "<div class=\"wf js-sample\" style=\"font-family:'uf{i}';font-size:{size}px\"><span class=\"pt\">{size}</span>{}</div>", esc(&text)).ok();
        }
        h.push_str("</div>\n");

        let mut shown = 0;
        for sc in &f.coverage.scripts {
            if let Some((_, dir, sample)) = SAMPLES.iter().find(|(code, _, _)| *code == sc.script) {
                if sc.codepoints < 20 {
                    continue;
                }
                write!(
                    h,
                    "<p class=\"para\" dir=\"{dir}\" style=\"font-family:'uf{i}'\">{}</p>",
                    esc(sample)
                )
                .ok();
                shown += 1;
                if shown == 3 {
                    break;
                }
            }
        }

        let blocks = crate::unicode::glyph_map(&f.coverage.ranges);
        h.push_str("<div class=\"glyphs\"><h3>Glyph map</h3>");
        for (bi, b) in blocks.iter().enumerate() {
            let open = if bi < 2 { " open" } else { "" };
            let cps = b
                .codepoints
                .iter()
                .map(|c| c.to_string())
                .collect::<Vec<_>>()
                .join(",");
            write!(h, "<details class=\"block\"{open} data-cps=\"{cps}\" data-font=\"uf{i}\"><summary>{} <span class=\"count\">{} / {}</span></summary><div class=\"grid\"></div></details>", esc(&b.block), b.codepoints.len(), b.block_size).ok();
        }
        h.push_str("</div>\n</article>\n");
    }
    h.push_str(
        "<footer>Generated by fontina. Fonts remain under their own licenses.</footer>\n<script>\n",
    );
    h.push_str(JS);
    h.push_str("</script>\n</body>\n</html>\n");
    Ok(h)
}

const CSS: &str = r#"
:root{color-scheme:light dark;--ink:#1b1d22;--muted:#5d616b;--rule:#dcddd8;--bg:#f7f7f5;--card:#fff;--accent:#2b5db8}
@media(prefers-color-scheme:dark){:root{--ink:#ecedea;--muted:#9a9ea8;--rule:#2e323a;--bg:#15171b;--card:#1d2026;--accent:#7fa7f0}}
*{box-sizing:border-box}body{margin:0;background:var(--bg);color:var(--ink);font:14px/1.5 system-ui,sans-serif}
.top{position:sticky;top:0;z-index:2;display:flex;flex-wrap:wrap;gap:16px;align-items:center;padding:12px 24px;background:var(--card);border-bottom:1px solid var(--rule)}
.top h1{font-size:16px;margin:0 16px 0 0}.top label{display:flex;gap:8px;align-items:center;color:var(--muted)}#text{width:28em;font:inherit;padding:4px 8px}
.top button{font:inherit;padding:4px 10px}
section.compare,article.face{padding:24px;border-bottom:1px solid var(--rule)}
article.face h2{font-size:40px;font-weight:normal;margin:0 0 8px;line-height:1.1}article.face h2 .sub{color:var(--muted);font-size:.6em}
.meta{display:grid;grid-template-columns:max-content 1fr;gap:2px 14px;margin:0 0 16px;font-size:13px;color:var(--muted)}.meta dt{font-weight:600}.meta dd{margin:0}
.controls{display:flex;flex-wrap:wrap;gap:8px 24px;margin:8px 0 16px;padding:12px;background:var(--card);border:1px solid var(--rule)}
.controls label{display:flex;gap:8px;align-items:center;font-size:13px}.controls code{color:var(--accent)}.axes input[type=range]{width:180px}
.waterfall .wf{white-space:nowrap;overflow:hidden;text-overflow:ellipsis;line-height:1.2;margin:4px 0}.wf .pt{display:inline-block;width:3em;font:11px system-ui;color:var(--muted);vertical-align:middle}
.para{font-size:20px;line-height:1.5;max-width:70ch;margin:16px 0}
.cmp{display:grid;grid-template-columns:200px 1fr;gap:16px;align-items:baseline;padding:8px 0;border-top:1px solid var(--rule)}.cmp-label{font-size:13px;color:var(--muted)}
.sample{font-size:48px;line-height:1.15;white-space:nowrap;overflow-x:auto}
.glyphs h3{font-size:14px;color:var(--muted);margin:24px 0 8px}
details.block summary{cursor:pointer;padding:6px 0}.count{color:var(--muted);font-size:12px;margin-left:6px}
.grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(56px,1fr));gap:2px;margin:6px 0 12px}
.g{aspect-ratio:1;display:flex;align-items:center;justify-content:center;font-size:26px;background:var(--card);border:1px solid var(--rule);position:relative}
.g::after{content:attr(data-u);position:absolute;bottom:2px;right:4px;font:9px system-ui;color:var(--muted)}
footer{padding:24px;color:var(--muted);font-size:12px}
@media print{.top{position:static}.top button,.controls,details.block:not([open]){display:none}article.face{break-inside:avoid-page}}
"#;

const JS: &str = r#"
const $ = (s, r=document) => r.querySelector(s), $$ = (s, r=document) => [...r.querySelectorAll(s)];
const textInput = $('#text'), sizeInput = $('#size');
function applyText(){ const t = textInput.value; $$('.js-sample').forEach(el => { const pt = el.querySelector('.pt'); el.textContent = t; if (pt) el.prepend(pt); }); }
function applySize(){ const s = sizeInput.value; $('#sizeout').textContent = s; $$('.sample').forEach(el => el.style.fontSize = s + 'px'); }
textInput.addEventListener('input', applyText); sizeInput.addEventListener('input', applySize);
function faceTargets(i){ return $$(`[data-face="${i}"] .js-sample, [data-face="${i}"] .para, [data-face="${i}"] .g, [data-face="${i}"] h2, .cmp .js-sample[data-face="${i}"]`); }
$$('article.face').forEach(art => {
  const i = art.dataset.face;
  const axes = $$('.axis', art), feats = $$('.feat', art), inst = $('.instance', art);
  function applyVar(){ const v = axes.map(a => `"${a.dataset.tag}" ${a.value}`).join(', '); faceTargets(i).forEach(el => el.style.fontVariationSettings = v); axes.forEach(a => a.nextElementSibling.textContent = a.value); if (inst && !inst.dataset.lock) inst.value = ''; }
  function applyFeat(){ const v = feats.filter(f => f.checked).map(f => `"${f.dataset.tag}" 1`).join(', ') || 'normal'; faceTargets(i).forEach(el => el.style.fontFeatureSettings = v); }
  axes.forEach(a => a.addEventListener('input', applyVar)); feats.forEach(f => f.addEventListener('change', applyFeat));
  if (inst) inst.addEventListener('change', () => { if (!inst.value) return; const c = inst.value.split(','); inst.dataset.lock = '1'; axes.forEach((a, k) => a.value = c[k]); applyVar(); delete inst.dataset.lock; inst.value = c.join(','); });
  $$('details.block', art).forEach(d => {
    const build = () => { const g = $('.grid', d); if (g.childElementCount) return; const font = d.dataset.font; const frag = document.createDocumentFragment();
      d.dataset.cps.split(',').forEach(cp => { const n = +cp; const el = document.createElement('div'); el.className = 'g'; el.dataset.face = i; el.style.fontFamily = `'${font}'`; el.textContent = String.fromCodePoint(n); el.dataset.u = n.toString(16).toUpperCase().padStart(4,'0'); el.title = 'U+' + el.dataset.u; frag.appendChild(el); });
      g.appendChild(frag); applyVar(); applyFeat(); };
    if (d.open) build(); d.addEventListener('toggle', () => d.open && build());
  });
});
"#;
