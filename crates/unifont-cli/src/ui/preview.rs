//! Half-block previews for the details pane, rendered through `unifont_core::render` and
//! cached by face, text, size and pane width.

use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use unifont_core::FaceMetadata;
use unifont_core::render::{Bitmap, RenderOptions, render_face};

#[derive(Default)]
pub struct Cache {
    key: Option<(i64, String, u32, u32)>,
    lines: Vec<Line<'static>>,
}

/// Sample text for a face: its own sample string, else a phrase in its main script.
pub fn sample_for(face: &FaceMetadata) -> String {
    let script = face
        .coverage
        .scripts
        .iter()
        .map(|s| s.script.as_str())
        .find(|s| !matches!(*s, "Zyyy" | "Zinh" | "Zzzz"))
        .unwrap_or("Latn");
    match script {
        "Arab" => "صِف خَلقَ خَودِ كَمِثلِ الشَمسِ",
        "Cyrl" => "Съешь же ещё этих мягких булок",
        "Grek" => "Ξεσκεπάζω την ψυχοφθόρα βδελυγμία",
        "Hebr" => "דג סקרן שט בים מאוכזב",
        "Deva" => "ऋषियों को सताने वाले दुष्ट",
        "Hani" => "視野無限廣 窗外有藍天",
        "Hira" | "Kana" => "いろはにほへと ちりぬるを",
        "Hang" => "키스의 고유조건은 입술끼리",
        "Thai" => "เป็นมนุษย์สุดประเสริฐเลิศคุณค่า",
        _ => "Sphinx of black quartz, judge my vow",
    }
    .to_string()
}

impl Cache {
    pub fn clear(&mut self) {
        self.key = None;
        self.lines.clear();
    }

    /// Lines for a preview fitting `cols` x `px_rows` pixels (two per text row).
    pub fn lines(
        &mut self,
        face: &FaceMetadata,
        text: &str,
        size: f32,
        cols: u32,
        px_rows: u32,
    ) -> Vec<Line<'static>> {
        let id = crate::face_key(face);
        let key = (id, text.to_string(), size as u32, cols);
        if self.key.as_ref() == Some(&key) {
            return self.lines.clone();
        }
        let opts = RenderOptions {
            text: text.to_string(),
            size,
            padding: 1,
            max_width: Some(cols),
            ..Default::default()
        };
        let lines = match render_face(face, &opts) {
            Ok(bitmap) => to_lines(&bitmap, px_rows),
            Err(e) => vec![Line::from(Span::styled(
                format!("preview unavailable: {e}"),
                Style::default().fg(Color::Red),
            ))],
        };
        self.key = Some(key);
        self.lines = lines.clone();
        lines
    }
}

/// Coverage to `▀` cells. Ink is drawn with the terminal's default foreground; the
/// half-block trick needs an explicit pair of colours only where there is ink.
fn to_lines(bm: &Bitmap, px_rows: u32) -> Vec<Line<'static>> {
    let (w, h) = (bm.width as usize, (bm.height.min(px_rows)) as usize);
    let mut out = Vec::with_capacity(h.div_ceil(2));
    // Ink colour: a neutral light grey blended over black reads on dark and light
    // themes alike once the block glyph carries both halves.
    let ink = |a: u8| {
        let v = 30 + (a as u16 * 200 / 255) as u8;
        Color::Rgb(v, v, v)
    };
    for row in 0..h.div_ceil(2) {
        let y0 = row * 2;
        let y1 = y0 + 1;
        let mut spans: Vec<Span<'static>> = Vec::with_capacity(w);
        let mut blank = 0usize;
        for x in 0..w {
            let top = bm.coverage[y0 * w + x];
            let bottom = if y1 < h { bm.coverage[y1 * w + x] } else { 0 };
            if top == 0 && bottom == 0 {
                blank += 1;
                continue;
            }
            if blank > 0 {
                spans.push(Span::raw(" ".repeat(blank)));
                blank = 0;
            }
            spans.push(Span::styled(
                "▀",
                Style::default().fg(ink(top)).bg(ink(bottom)),
            ));
        }
        out.push(Line::from(spans));
    }
    out
}
