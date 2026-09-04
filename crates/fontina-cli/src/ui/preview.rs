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

//! Half-block previews for the details pane, rendered through `fontina_core::render` and
//! cached by face, text, size and pane width.

use fontina_core::FaceMetadata;
use fontina_core::render::{Bitmap, RenderOptions, render_face};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

#[derive(Default)]
pub struct Cache {
    /// The face and the exact options its lines were rendered from. Keying on the
    /// options themselves means a new `RenderOptions` field cannot be left out of the
    /// key and silently stop the preview repainting.
    key: Option<(i64, RenderOptions, u32)>,
    lines: Vec<Line<'static>>,
}

/// Sample text for a face: the shared default for Latin, so the pane and the HTML
/// specimen agree, and the opening clause of its own script's paragraph otherwise.
pub fn sample_for(face: &FaceMetadata) -> String {
    fontina_core::typography::preview_text(face).to_string()
}

impl Cache {
    pub fn clear(&mut self) {
        self.key = None;
        self.lines.clear();
    }

    /// Lines for a preview of `face` under `opts`, fitting `px_rows` pixel rows (two per
    /// text row).
    pub fn lines(
        &mut self,
        face: &FaceMetadata,
        opts: &RenderOptions,
        px_rows: u32,
    ) -> Vec<Line<'static>> {
        let key = (crate::face_key(face), opts.clone(), px_rows);
        if self.key.as_ref() == Some(&key) {
            return self.lines.clone();
        }
        let lines = match render_face(face, opts) {
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
