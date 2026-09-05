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

//! Encoders for [`Bitmap`](super::Bitmap): PNG (for files and for the kitty and iTerm2
//! inline-image protocols), DEC sixel, and half-block text that any terminal with
//! 24-bit colour can show.

use super::Bitmap;
use std::fmt::Write as _;
use std::io::Write as _;

/// An sRGB colour.
pub type Rgb = [u8; 3];

/// Parse `#rrggbb` or `rrggbb`.
pub fn parse_rgb(s: &str) -> Option<Rgb> {
    // One optional `#`, then exactly six hex digits. `from_str_radix` accepts a leading
    // sign, so `+f8800` used to parse as `#0f8800`, and trimming every leading `#`
    // accepted `###1a2b3c`.
    let s = s.trim();
    let s = s.strip_prefix('#').unwrap_or(s);
    if s.len() != 6 || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let v = u32::from_str_radix(s, 16).ok()?;
    Some([(v >> 16) as u8, (v >> 8) as u8, v as u8])
}

fn blend(fg: Rgb, bg: Rgb, a: u8) -> Rgb {
    let a = a as u32;
    let mix = |f: u8, b: u8| ((f as u32 * a + b as u32 * (255 - a)) / 255) as u8;
    [mix(fg[0], bg[0]), mix(fg[1], bg[1]), mix(fg[2], bg[2])]
}

// ----- PNG -----

fn crc32(data: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for (n, slot) in table.iter_mut().enumerate() {
        let mut c = n as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
        }
        *slot = c;
    }
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc = table[((crc ^ b as u32) & 0xFF) as usize] ^ (crc >> 8);
    }
    crc ^ 0xFFFF_FFFF
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let mut body = Vec::with_capacity(4 + data.len());
    body.extend_from_slice(kind);
    body.extend_from_slice(data);
    out.extend_from_slice(&body);
    out.extend_from_slice(&crc32(&body).to_be_bytes());
}

/// Encode as PNG. With `bg: None` the image is RGBA with the coverage as alpha (ink in
/// `fg`), so it composes over any terminal background; with `Some(bg)` it is opaque RGB.
pub fn png(bitmap: &Bitmap, fg: Rgb, bg: Option<Rgb>) -> Vec<u8> {
    let (w, h) = (bitmap.width as usize, bitmap.height as usize);
    let channels = if bg.is_some() { 3 } else { 4 };
    let mut raw = Vec::with_capacity(h * (1 + w * channels));
    for y in 0..h {
        raw.push(0); // filter: none
        for x in 0..w {
            let a = bitmap.coverage[y * w + x];
            match bg {
                Some(bg) => raw.extend_from_slice(&blend(fg, bg, a)),
                None => {
                    raw.extend_from_slice(&fg);
                    raw.push(a);
                }
            }
        }
    }
    let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    // The sink is a Vec<u8>; io::Write on one cannot fail.
    #[expect(clippy::expect_used, reason = "io::Write on a Vec<u8> cannot fail")]
    let idat = {
        z.write_all(&raw).expect("in-memory write");
        z.finish().expect("in-memory finish")
    };

    let mut out = Vec::with_capacity(idat.len() + 64);
    out.extend_from_slice(b"\x89PNG\r\n\x1a\n");
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&bitmap.width.to_be_bytes());
    ihdr.extend_from_slice(&bitmap.height.to_be_bytes());
    ihdr.extend_from_slice(&[8, if bg.is_some() { 2 } else { 6 }, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"IDAT", &idat);
    chunk(&mut out, b"IEND", &[]);
    out
}

// ----- sixel -----

/// Encode as a DEC sixel image with `levels` grey steps between `bg` and `fg`.
pub fn sixel(bitmap: &Bitmap, fg: Rgb, bg: Rgb, levels: u8) -> String {
    let levels = levels.clamp(2, 64) as usize;
    let (w, h) = (bitmap.width as usize, bitmap.height as usize);
    let mut out = String::with_capacity(w * h / 4 + 256);
    // DCS P1;P2;P3 q : P2=1 leaves untouched pixels at the background colour of the terminal.
    out.push_str("\x1bP0;1;0q");
    let _ = write!(out, "\"1;1;{w};{h}");
    for (i, level) in (0..levels).enumerate() {
        let a = (level * 255 / (levels - 1)) as u8;
        let c = blend(fg, bg, a);
        let _ = write!(
            out,
            "#{i};2;{};{};{}",
            c[0] as u32 * 100 / 255,
            c[1] as u32 * 100 / 255,
            c[2] as u32 * 100 / 255
        );
    }
    let quant = |a: u8| ((a as usize * (levels - 1) + 127) / 255).min(levels - 1);
    for band in 0..h.div_ceil(6) {
        let y0 = band * 6;
        let mut used = vec![false; levels];
        for y in y0..(y0 + 6).min(h) {
            for x in 0..w {
                used[quant(bitmap.coverage[y * w + x])] = true;
            }
        }
        let mut first = true;
        for (level, in_use) in used.iter().enumerate() {
            if !in_use || level == 0 {
                continue; // level 0 is the background: leave those pixels alone
            }
            if !first {
                out.push('$'); // carriage return within the band
            }
            first = false;
            let _ = write!(out, "#{level}");
            let mut run: Option<(u8, usize)> = None;
            let flush = |out: &mut String, run: Option<(u8, usize)>| {
                if let Some((ch, n)) = run {
                    if n > 3 {
                        let _ = write!(out, "!{n}{}", (63 + ch) as char);
                    } else {
                        for _ in 0..n {
                            out.push((63 + ch) as char);
                        }
                    }
                }
            };
            for x in 0..w {
                let mut bits = 0u8;
                for dy in 0..6 {
                    let y = y0 + dy;
                    if y < h && quant(bitmap.coverage[y * w + x]) == level {
                        bits |= 1 << dy;
                    }
                }
                run = match run {
                    Some((ch, n)) if ch == bits => Some((ch, n + 1)),
                    other => {
                        flush(&mut out, other);
                        Some((bits, 1))
                    }
                };
            }
            flush(&mut out, run);
        }
        out.push('-'); // next band
    }
    out.push_str("\x1b\\");
    out
}

// ----- half blocks -----

/// Render as text: one character per pixel column, two pixel rows per line, using `▀`
/// with 24-bit foreground and background colours. Cells with no ink keep the terminal's
/// own colours, so the preview sits on whatever background the user has.
pub fn half_blocks(bitmap: &Bitmap, fg: Rgb, bg: Rgb) -> String {
    let (w, h) = (bitmap.width as usize, bitmap.height as usize);
    let mut out = String::with_capacity(w * h * 12);
    for row in 0..h.div_ceil(2) {
        let y0 = row * 2;
        let y1 = y0 + 1;
        for x in 0..w {
            let top = bitmap.coverage[y0 * w + x];
            let bottom = if y1 < h {
                bitmap.coverage[y1 * w + x]
            } else {
                0
            };
            if top == 0 && bottom == 0 {
                out.push(' ');
                continue;
            }
            let t = blend(fg, bg, top);
            let b = blend(fg, bg, bottom);
            let _ = write!(
                out,
                "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m▀\x1b[0m",
                t[0], t[1], t[2], b[0], b[1], b[2]
            );
        }
        out.push('\n');
    }
    out
}

// ----- terminal inline-image protocols -----

fn b64(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
}

/// Wrap a terminal escape for tmux passthrough (`set -g allow-passthrough on`).
fn tmux_wrap(seq: &str, tmux: bool) -> String {
    if !tmux {
        return seq.to_string();
    }
    format!("\x1bPtmux;{}\x1b\\", seq.replace('\x1b', "\x1b\x1b"))
}

/// kitty graphics protocol: transmit and display a PNG, chunked at 4096 bytes.
pub fn kitty(png: &[u8], tmux: bool) -> String {
    let data = b64(png);
    let chunks: Vec<&str> = data
        .as_bytes()
        .chunks(4096)
        .map(|c| std::str::from_utf8(c).unwrap_or(""))
        .collect();
    let mut out = String::with_capacity(data.len() + 64 * chunks.len());
    for (i, c) in chunks.iter().enumerate() {
        let more = if i + 1 < chunks.len() { 1 } else { 0 };
        let seq = if i == 0 {
            format!("\x1b_Gf=100,a=T,t=d,q=2,m={more};{c}\x1b\\")
        } else {
            format!("\x1b_Gm={more};{c}\x1b\\")
        };
        out.push_str(&tmux_wrap(&seq, tmux));
    }
    out.push('\n');
    out
}

/// iTerm2 inline image protocol (also WezTerm, mintty, Konsole).
pub fn iterm(png: &[u8], tmux: bool) -> String {
    let seq = format!(
        "\x1b]1337;File=inline=1;size={};preserveAspectRatio=1:{}\x07",
        png.len(),
        b64(png)
    );
    let mut out = tmux_wrap(&seq, tmux);
    out.push('\n');
    out
}
