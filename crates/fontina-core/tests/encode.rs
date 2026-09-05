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

//! The terminal encoders, against one small bitmap written as text.
//!
//! These bytes go straight at a terminal, so a mistake in them is a corrupted screen
//! rather than an error message: a stray escape inside a sixel, a kitty chunk that lost
//! a byte at a boundary, a PNG whose CRC no reader will accept. The tests therefore
//! state the properties a reader of each format would check — signature and chunk CRCs,
//! introducer and terminator, the base64 round trip, the doubling of escapes under tmux
//! — and keep a snapshot of the small outputs, which can be read by eye.

use fontina_core::render::Bitmap;
use fontina_core::render::encode::{self, Rgb};
use std::io::Read as _;

// ----- the bitmap under test -----

/// The test bitmap, as art: `.` is bare, `-` a third of the ink, `+` two thirds, `#`
/// all of it. Eight columns and seven rows, so a sixel gets one full band of six pixel
/// rows and one band with a single row in it, and the four identical `#` columns across
/// the middle are a long enough run to take the run-length branch.
#[rustfmt::skip] // it is a picture; keep it one row to a line
const ART: [&str; 7] = [
    "........",
    ".######.",
    ".#....#.",
    ".#-++-#.",
    ".#....#.",
    ".######.",
    "...--...",
];

const W: u32 = 8;
const H: u32 = 7;

/// Ink, and a background to blend it over. Both differ in every channel, so a channel
/// swapped for another shows up.
const FG: Rgb = [0xFF, 0x88, 0x00];
const BG: Rgb = [0x11, 0x22, 0x33];

fn coverage_of(c: char) -> u8 {
    match c {
        '.' => 0,
        '-' => 85,
        '+' => 170,
        '#' => 255,
        other => panic!("{other:?} is not one of . - + #"),
    }
}

fn art_of(coverage: u8) -> char {
    match coverage {
        0 => '.',
        85 => '-',
        170 => '+',
        255 => '#',
        other => panic!("{other} is not one of the four levels the art uses"),
    }
}

fn bitmap() -> Bitmap {
    let coverage: Vec<u8> = ART
        .iter()
        .flat_map(|row| row.chars().map(coverage_of))
        .collect();
    assert_eq!(coverage.len(), (W * H) as usize);
    Bitmap {
        width: W,
        height: H,
        coverage,
        baseline: 5.0,
        glyphs: 1,
        missing: 0,
    }
}

/// Escapes made visible, so a snapshot can be read and a failure diffed by eye.
fn visible(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '\x1b' => "<ESC>".to_string(),
            '\x07' => "<BEL>".to_string(),
            c if c == '\n' || !c.is_control() => c.to_string(),
            c => format!("<{:02x}>", c as u32),
        })
        .collect()
}

// ----- PNG -----

/// CRC-32/ISO-HDLC, written the slow way on purpose: the encoder builds a table, and a
/// check worth having does not share the arithmetic it is checking.
fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                0xEDB8_8320 ^ (crc >> 1)
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

struct Chunk {
    kind: String,
    data: Vec<u8>,
    crc_ok: bool,
}

/// Walk a PNG the way a decoder does: signature, then length-type-data-CRC quads.
fn chunks(png: &[u8]) -> Vec<Chunk> {
    assert_eq!(
        &png[..8],
        b"\x89PNG\r\n\x1a\n",
        "the file does not open with the PNG signature"
    );
    let mut out = Vec::new();
    let mut i = 8;
    while i < png.len() {
        assert!(i + 8 <= png.len(), "a chunk header runs past the end");
        let len = u32::from_be_bytes(png[i..i + 4].try_into().unwrap()) as usize;
        assert!(
            i + 12 + len <= png.len(),
            "a chunk of {len} bytes runs past the end of the file"
        );
        let body = &png[i + 4..i + 8 + len];
        let crc = u32::from_be_bytes(png[i + 8 + len..i + 12 + len].try_into().unwrap());
        out.push(Chunk {
            kind: String::from_utf8(body[..4].to_vec()).unwrap(),
            data: body[4..].to_vec(),
            crc_ok: crc == crc32(body),
        });
        i += 12 + len;
    }
    out
}

fn inflate(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    flate2::read::ZlibDecoder::new(data)
        .read_to_end(&mut out)
        .expect("the IDAT is not a zlib stream");
    out
}

/// The scanlines of an inflated IDAT, with the filter byte checked and stripped.
fn scanlines(idat: &[u8], bytes_per_pixel: usize) -> Vec<Vec<u8>> {
    let stride = 1 + W as usize * bytes_per_pixel;
    assert_eq!(
        idat.len(),
        stride * H as usize,
        "the raster is {} bytes, not {H} rows of {stride}",
        idat.len(),
    );
    idat.chunks(stride)
        .map(|row| {
            assert_eq!(
                row[0], 0,
                "the encoder only ever writes filter type 0 (none)"
            );
            row[1..].to_vec()
        })
        .collect()
}

#[test]
fn png_carries_the_coverage_as_alpha_and_a_crc_every_reader_will_accept() {
    let png = encode::png(&bitmap(), FG, None);
    let chunks = chunks(&png);
    let kinds: Vec<&str> = chunks.iter().map(|c| c.kind.as_str()).collect();
    assert_eq!(kinds, ["IHDR", "IDAT", "IEND"], "chunk order");
    for c in &chunks {
        assert!(c.crc_ok, "the CRC of the {} chunk is wrong", c.kind);
    }

    let ihdr = &chunks[0].data;
    assert_eq!(ihdr.len(), 13);
    assert_eq!(u32::from_be_bytes(ihdr[0..4].try_into().unwrap()), W);
    assert_eq!(u32::from_be_bytes(ihdr[4..8].try_into().unwrap()), H);
    assert_eq!(
        ihdr[8..],
        [8, 6, 0, 0, 0],
        "8 bits a channel, colour type 6 (RGBA), no interlacing"
    );

    // Every pixel is the ink colour and the coverage is the alpha, so the image
    // composes over whatever the terminal's background happens to be.
    let rows = scanlines(&inflate(&chunks[1].data), 4);
    let mut back = Vec::new();
    for row in &rows {
        let mut line = String::new();
        for px in row.chunks(4) {
            assert_eq!(&px[..3], &FG[..], "a pixel is not the ink colour");
            line.push(art_of(px[3]));
        }
        back.push(line);
    }
    assert_eq!(
        back, ART,
        "the alpha channel is not the coverage it was given"
    );

    let decoded = format!(
        "IHDR {W}x{H}, 8 bits a channel, colour type {} (RGBA)\n\
         IDAT {} scanlines, filter none, every pixel #{:02x}{:02x}{:02x}, alpha:\n\
         {}\n\
         IEND\n\
         every chunk CRC checks out",
        ihdr[9],
        rows.len(),
        FG[0],
        FG[1],
        FG[2],
        back.join("\n"),
    );
    insta::assert_snapshot!(decoded);
}

#[test]
fn png_with_a_background_is_opaque_rgb_blended_over_it() {
    let png = encode::png(&bitmap(), FG, Some(BG));
    let chunks = chunks(&png);
    assert!(chunks.iter().all(|c| c.crc_ok), "a chunk CRC is wrong");
    assert_eq!(
        chunks[0].data[8..],
        [8, 2, 0, 0, 0],
        "colour type 2 (RGB): with a background there is nothing left to be transparent"
    );

    let rows = scanlines(&inflate(&chunks[1].data), 3);
    for (y, row) in rows.iter().enumerate() {
        for (x, px) in row.chunks(3).enumerate() {
            let a = coverage_of(ART[y].as_bytes()[x] as char);
            match a {
                0 => assert_eq!(px, &BG[..], "bare pixels keep the background, untouched"),
                255 => assert_eq!(px, &FG[..], "full coverage is the ink, undiluted"),
                _ => {
                    for c in 0..3 {
                        let exact = FG[c] as f32 * (a as f32 / 255.0)
                            + BG[c] as f32 * (1.0 - a as f32 / 255.0);
                        assert!(
                            (px[c] as f32 - exact).abs() <= 1.0,
                            "pixel {x},{y} channel {c} is {} where source-over gives {exact:.1}",
                            px[c]
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn a_flipped_byte_makes_the_crc_check_fail() {
    // The CRC assertions above are worth their lines only if they can fail. This is
    // the test for the test.
    let mut png = encode::png(&bitmap(), FG, None);
    let len = png.len();
    png[len - 8] ^= 0x01; // inside IEND's type, before its CRC
    assert!(
        chunks(&png).iter().any(|c| !c.crc_ok),
        "a corrupted chunk went unnoticed"
    );
}

// ----- sixel -----

/// The colour registers a sixel declares, as `(index, r, g, b)` in DEC percentages.
/// `#i;2;r;g;b` declares one; a bare `#i` further on only selects it.
fn registers(sixel: &str) -> Vec<(usize, u32, u32, u32)> {
    let mut out = Vec::new();
    for part in sixel.split('#').skip(1) {
        let fields: Vec<u32> = part
            .split(';')
            .map(|f| {
                f.chars()
                    .take_while(char::is_ascii_digit)
                    .collect::<String>()
                    .parse()
                    .unwrap_or(u32::MAX)
            })
            .collect();
        if fields.len() >= 5 && fields[1] == 2 {
            out.push((fields[0] as usize, fields[2], fields[3], fields[4]));
        }
    }
    out
}

#[test]
fn sixel_is_wrapped_in_a_device_control_string_and_holds_no_control_bytes() {
    let sixel = encode::sixel(&bitmap(), FG, BG, 4);
    assert!(
        sixel.starts_with("\x1bP0;1;0q\"1;1;8;7"),
        "introducer and raster attributes"
    );
    assert!(sixel.ends_with("\x1b\\"), "string terminator");
    assert_eq!(
        sixel.matches('-').count(),
        (H as usize).div_ceil(6),
        "one band separator per six pixel rows"
    );

    // The body is everything the terminal reads between the two escapes. A control byte
    // or a non-ASCII byte in there is the failure that leaves a shell unusable.
    let body = &sixel["\x1bP0;1;0q".len()..sixel.len() - "\x1b\\".len()];
    for (i, c) in body.char_indices() {
        assert!(
            c.is_ascii_graphic(),
            "byte {i} of the sixel body is {c:?}, which is not printable ASCII"
        );
    }

    insta::assert_snapshot!(visible(&sixel));
}

#[test]
fn sixel_declares_one_colour_register_a_level_inside_the_dec_percentage_range() {
    for (asked, expected) in [(0u8, 2usize), (1, 2), (4, 4), (16, 16), (200, 64)] {
        let sixel = encode::sixel(&bitmap(), FG, BG, asked);
        let regs = registers(&sixel);
        assert_eq!(
            regs.len(),
            expected,
            "{asked} levels should have been clamped to {expected}"
        );
        for (n, (i, r, g, b)) in regs.iter().enumerate() {
            assert_eq!(*i, n, "registers are declared in order from 0");
            for (name, v) in [("red", r), ("green", g), ("blue", b)] {
                assert!(
                    *v <= 100,
                    "{name} is {v}: sixel colours are percentages, not 0-255"
                );
            }
        }
        let pct = |c: u8| c as u32 * 100 / 255;
        assert_eq!(
            regs[0],
            (0, pct(BG[0]), pct(BG[1]), pct(BG[2])),
            "the lowest register is the background"
        );
        assert_eq!(
            *regs.last().unwrap(),
            (expected - 1, pct(FG[0]), pct(FG[1]), pct(FG[2])),
            "the highest register is the ink"
        );
    }
}

// ----- half blocks -----

#[test]
fn half_blocks_are_one_cell_a_column_and_two_pixel_rows_a_line() {
    let blocks = encode::half_blocks(&bitmap(), FG, BG);
    let lines: Vec<&str> = blocks.lines().collect();
    assert_eq!(
        lines.len(),
        (H as usize).div_ceil(2),
        "two pixel rows to a text line"
    );

    // Each line is exactly `W` cells, and a cell is either a bare space — so the
    // terminal's own colours show through where there is no ink — or a `▀` between a
    // pair of 24-bit colours and a reset. Nothing else may reach the screen.
    for (n, line) in lines.iter().enumerate() {
        let mut rest = *line;
        let mut cells = 0;
        while !rest.is_empty() {
            if let Some(tail) = rest.strip_prefix(' ') {
                rest = tail;
            } else {
                let (fg, tail) = rest
                    .strip_prefix("\x1b[38;2;")
                    .and_then(|t| t.split_once("m\x1b[48;2;"))
                    .unwrap_or_else(|| {
                        panic!("line {n} has no foreground colour at {}", visible(rest))
                    });
                let (bg, tail) = tail.split_once("m▀\x1b[0m").unwrap_or_else(|| {
                    panic!("line {n} has a cell that is never closed and reset")
                });
                for triple in [fg, bg] {
                    assert_eq!(
                        triple.split(';').count(),
                        3,
                        "line {n} has {triple:?} where a 24-bit r;g;b belongs"
                    );
                }
                rest = tail;
            }
            cells += 1;
        }
        assert_eq!(cells, W as usize, "line {n} is not {W} cells wide");
    }

    insta::assert_snapshot!(visible(&blocks));
}

// ----- kitty and iTerm2 -----

/// A payload big enough to be chunked, and incompressible enough to stay that way.
fn payload(n: usize) -> Vec<u8> {
    let mut x = 0x1234_5678u32;
    (0..n)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            x as u8
        })
        .collect()
}

fn b64_decode(s: &str) -> Vec<u8> {
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD
        .decode(s)
        .expect("the payload is not valid base64")
}

/// Split a kitty transmission into the bodies of its `ESC _G … ESC \` units.
fn kitty_units(s: &str) -> Vec<&str> {
    assert!(s.ends_with('\n'), "the transmission ends with a newline");
    let body = &s[..s.len() - 1];
    assert_eq!(body.matches('\n').count(), 0, "and holds no other newline");
    body.split_inclusive("\x1b\\")
        .map(|unit| {
            unit.strip_prefix("\x1b_G")
                .and_then(|u| u.strip_suffix("\x1b\\"))
                .unwrap_or_else(|| panic!("{} is not a kitty escape", visible(unit)))
        })
        .collect()
}

#[test]
fn kitty_chunks_the_payload_at_four_kilobytes_and_puts_it_back_together() {
    let png = payload(7000);
    let kitty = encode::kitty(&png, false);
    let units = kitty_units(&kitty);
    assert_eq!(units.len(), 3, "9336 base64 characters in 4096-byte chunks");

    let mut data = String::new();
    for (i, unit) in units.iter().enumerate() {
        let (keys, chunk) = unit.split_once(';').expect("keys and payload");
        let last = i + 1 == units.len();
        if i == 0 {
            assert_eq!(
                keys,
                format!("f=100,a=T,t=d,q=2,m={}", u8::from(!last)),
                "the first chunk declares the format, and only the first"
            );
        } else {
            assert_eq!(
                keys,
                format!("m={}", u8::from(!last)),
                "a continuation carries the more-follows flag and nothing else"
            );
        }
        assert!(
            chunk.len() <= 4096,
            "chunk {i} is {} bytes, over kitty's limit",
            chunk.len()
        );
        if !last {
            assert_eq!(chunk.len(), 4096, "only the last chunk may be short");
        }
        data.push_str(chunk);
    }
    assert_eq!(b64_decode(&data), png, "the image did not survive chunking");

    // A payload that fits in one chunk is sent as one chunk that says so.
    let small = encode::kitty(&encode::png(&bitmap(), FG, None), false);
    let units = kitty_units(&small);
    assert_eq!(units.len(), 1);
    assert!(
        units[0].starts_with("f=100,a=T,t=d,q=2,m=0;"),
        "{}",
        visible(units[0])
    );
}

/// Undo a tmux passthrough wrapper: `ESC P tmux ; … ESC \` with every escape inside
/// doubled, so nothing in it can be read as an escape tmux should act on itself.
///
/// The doubling is exactly why this cannot scan for the terminator first: the last
/// bytes of a wrapped kitty escape are `ESC ESC \ ESC \`, and the terminator appears
/// one byte into the doubled pair. So it walks the string instead, and a lone escape
/// where a pair belongs — the mistake that ends a passthrough halfway and spills the
/// rest of the image onto the screen — has nowhere to hide.
fn untmux(s: &str) -> String {
    assert!(s.ends_with('\n'));
    let mut out = String::new();
    let mut rest = &s[..s.len() - 1];
    while !rest.is_empty() {
        rest = rest
            .strip_prefix("\x1bPtmux;")
            .unwrap_or_else(|| panic!("{} is not a tmux passthrough", visible(rest)));
        loop {
            if let Some(tail) = rest.strip_prefix("\x1b\x1b") {
                out.push('\x1b');
                rest = tail;
            } else if let Some(tail) = rest.strip_prefix("\x1b\\") {
                rest = tail;
                break;
            } else {
                let c = rest
                    .chars()
                    .next()
                    .expect("the passthrough is unterminated");
                assert_ne!(
                    c, '\x1b',
                    "a single escape inside the wrapper would end the passthrough early"
                );
                out.push(c);
                rest = &rest[c.len_utf8()..];
            }
        }
    }
    out.push('\n');
    out
}

#[test]
fn tmux_passthrough_wraps_every_escape_and_changes_nothing_else() {
    let png = payload(7000);
    for (name, plain, wrapped) in [
        (
            "kitty",
            encode::kitty(&png, false),
            encode::kitty(&png, true),
        ),
        (
            "iterm",
            encode::iterm(&png, false),
            encode::iterm(&png, true),
        ),
    ] {
        assert_ne!(plain, wrapped, "{name} ignored the tmux flag");
        assert_eq!(
            untmux(&wrapped),
            plain,
            "{name} under tmux is not the same bytes, only wrapped"
        );
    }
}

#[test]
fn iterm_declares_the_size_of_the_image_it_sends() {
    let png = encode::png(&bitmap(), FG, None);
    let out = encode::iterm(&png, false);
    let body = out
        .strip_prefix("\x1b]1337;File=")
        .and_then(|b| b.strip_suffix("\x07\n"))
        .unwrap_or_else(|| panic!("{} is not an iTerm2 inline image", visible(&out)));
    let (keys, data) = body.split_once(':').expect("keys and payload");
    assert_eq!(
        keys,
        format!("inline=1;size={};preserveAspectRatio=1", png.len())
    );
    assert_eq!(
        b64_decode(data),
        png,
        "the declared size and the payload disagree"
    );
}

// ----- colours -----

#[test]
fn parse_rgb_takes_six_hex_digits_and_little_else() {
    assert_eq!(encode::parse_rgb("#1a2B3c"), Some([0x1a, 0x2b, 0x3c]));
    assert_eq!(encode::parse_rgb("1a2b3c"), Some([0x1a, 0x2b, 0x3c]));
    assert_eq!(encode::parse_rgb("  #FFFFFF \n"), Some([0xff, 0xff, 0xff]));
    assert_eq!(encode::parse_rgb("000000"), Some([0, 0, 0]));

    for bad in [
        "",
        "#",
        "nope",
        "fff",
        "#fff",
        "#12345",
        "1a2b3c4",
        "gg2b3c",
        "12 3c",
        "0x1234",
        "١٢٣٤٥٦",
    ] {
        assert_eq!(encode::parse_rgb(bad), None, "{bad:?} should be rejected");
    }

    // Two inputs that used to be accepted by accident: the six characters went to
    // `u32::from_str_radix`, which takes a leading sign, so five hex digits behind a `+`
    // parsed; and the `#` was removed with `trim_start_matches`, so any number of them
    // was fine.
    assert_eq!(encode::parse_rgb("+f8800"), None);
    assert_eq!(encode::parse_rgb("###1a2b3c"), None);
}
