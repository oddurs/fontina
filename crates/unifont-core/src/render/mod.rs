//! Truthful previews without a browser: text is shaped by harfrust (the HarfBuzz port
//! on `read-fonts`), outlines come from skrifa at the requested size and axis position,
//! and a coverage rasteriser fills them into an 8-bit bitmap. [`encode`] turns that
//! bitmap into PNG, sixel, or half-block text for terminals.

pub mod encode;

use crate::error::{Error, Result};
use crate::model::FaceMetadata;
use ab_glyph_rasterizer::{Point, Rasterizer};
use skrifa::MetadataProvider;
use skrifa::instance::Size;
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::raw::{FontRef, TableProvider};

/// What to render.
#[derive(Debug, Clone)]
pub struct RenderOptions {
    /// Text; `\n` starts a new line.
    pub text: String,
    /// Font size in pixels.
    pub size: f32,
    /// Variable axis settings in user space, e.g. `("wght", 700.0)`.
    pub variations: Vec<(String, f32)>,
    /// OpenType features to force on (`true`) or off (`false`), e.g. `("smcp", true)`.
    pub features: Vec<(String, bool)>,
    /// Pixels around the text.
    pub padding: u32,
    /// Clip the bitmap to this width in pixels (text is not wrapped).
    pub max_width: Option<u32>,
}

impl Default for RenderOptions {
    fn default() -> Self {
        RenderOptions {
            text: "Sphinx of black quartz, judge my vow".into(),
            size: 48.0,
            variations: Vec::new(),
            features: Vec::new(),
            padding: 4,
            max_width: None,
        }
    }
}

/// An 8-bit coverage bitmap, row-major, top-down.
#[derive(Debug, Clone)]
pub struct Bitmap {
    pub width: u32,
    pub height: u32,
    /// `width * height` coverage values, 0 (background) to 255 (full ink).
    pub coverage: Vec<u8>,
    /// Baseline of the first line, in pixels from the top.
    pub baseline: f32,
    /// Glyphs placed, over all lines.
    pub glyphs: usize,
    /// Glyphs the font had no outline for (rendered as gaps).
    pub missing: usize,
}

impl Bitmap {
    pub fn get(&self, x: u32, y: u32) -> u8 {
        self.coverage
            .get((y * self.width + x) as usize)
            .copied()
            .unwrap_or(0)
    }

    /// Any ink at all.
    pub fn is_blank(&self) -> bool {
        self.coverage.iter().all(|&c| c == 0)
    }
}

/// One shaped line, in pixels.
struct ShapedLine {
    glyphs: Vec<(skrifa::GlyphId, f32, f32)>, // id, x, y (y up, relative to baseline)
    advance: f32,
}

fn tag(s: &str) -> Result<skrifa::Tag> {
    let b = s.as_bytes();
    if b.len() != 4 {
        return Err(Error::Other(format!(
            "{s:?} is not a four-character OpenType tag"
        )));
    }
    Ok(skrifa::Tag::new(&[b[0], b[1], b[2], b[3]]))
}

fn shape_line(
    font: &FontRef,
    data: &harfrust::ShaperData,
    instance: &harfrust::ShaperInstance,
    features: &[harfrust::Feature],
    line: &str,
    scale: f32,
) -> ShapedLine {
    let shaper = data.shaper(font).instance(Some(instance)).build();
    let mut buffer = harfrust::UnicodeBuffer::new();
    buffer.push_str(line);
    buffer.guess_segment_properties();
    let out = shaper.shape(buffer, harfrust::ShapeOptions::new().features(features));
    let mut glyphs = Vec::with_capacity(out.len());
    let mut x = 0.0f32;
    for (info, pos) in out.glyph_infos().iter().zip(out.glyph_positions()) {
        glyphs.push((
            skrifa::GlyphId::new(info.glyph_id),
            x + pos.x_offset as f32 * scale,
            pos.y_offset as f32 * scale,
        ));
        x += pos.x_advance as f32 * scale;
    }
    ShapedLine { glyphs, advance: x }
}

/// Feeds skrifa's y-up outline into the rasteriser's y-down pixel grid.
struct Pen<'a> {
    r: &'a mut Rasterizer,
    ox: f32,
    oy: f32,
    last: Point,
    start: Point,
}

impl Pen<'_> {
    fn p(&self, x: f32, y: f32) -> Point {
        Point {
            x: self.ox + x,
            y: self.oy - y,
        }
    }
}

impl OutlinePen for Pen<'_> {
    fn move_to(&mut self, x: f32, y: f32) {
        self.last = self.p(x, y);
        self.start = self.last;
    }
    fn line_to(&mut self, x: f32, y: f32) {
        let p = self.p(x, y);
        self.r.draw_line(self.last, p);
        self.last = p;
    }
    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        let c = self.p(cx0, cy0);
        let p = self.p(x, y);
        self.r.draw_quad(self.last, c, p);
        self.last = p;
    }
    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        let c0 = self.p(cx0, cy0);
        let c1 = self.p(cx1, cy1);
        let p = self.p(x, y);
        self.r.draw_cubic(self.last, c0, c1, p);
        self.last = p;
    }
    fn close(&mut self) {
        if self.last != self.start {
            self.r.draw_line(self.last, self.start);
            self.last = self.start;
        }
    }
}

/// Render `opts.text` with the face at `index` inside raw sfnt `bytes`.
pub fn render_sfnt(bytes: &[u8], index: u32, opts: &RenderOptions) -> Result<Bitmap> {
    let font = FontRef::from_index(bytes, index)?;
    let upm = font.head()?.units_per_em().max(16) as f32;
    let size = opts.size.clamp(4.0, 4096.0);
    let scale = size / upm;

    let mut settings = Vec::with_capacity(opts.variations.len());
    for (t, v) in &opts.variations {
        settings.push((tag(t)?, *v));
    }
    let location = font.axes().location(settings.iter().map(|(t, v)| (*t, *v)));
    let data = harfrust::ShaperData::new(&font);
    let instance = harfrust::ShaperInstance::from_variations(&font, settings.iter());
    let mut features = Vec::with_capacity(opts.features.len());
    for (t, on) in &opts.features {
        features.push(harfrust::Feature::new(tag(t)?, u32::from(*on), ..));
    }

    let metrics = font.metrics(Size::new(size), &location);
    let ascent = metrics.ascent.max(0.0);
    let descent = (-metrics.descent).max(0.0);
    let line_height = (ascent + descent + metrics.leading.max(0.0)).max(1.0);

    let lines: Vec<ShapedLine> = opts
        .text
        .split('\n')
        .map(|l| shape_line(&font, &data, &instance, &features, l, scale))
        .collect();
    let pad = opts.padding as f32;
    let text_width = lines.iter().map(|l| l.advance).fold(0.0, f32::max);
    let mut width = (text_width + 2.0 * pad).ceil().max(1.0) as u32;
    if let Some(max) = opts.max_width {
        width = width.min(max.max(1));
    }
    let height = ((lines.len().max(1) as f32 - 1.0) * line_height + ascent + descent + 2.0 * pad)
        .ceil()
        .max(1.0) as u32;
    if (width as u64) * (height as u64) > 64 * 1024 * 1024 {
        return Err(Error::Other(format!(
            "preview would be {width}x{height} pixels; use a smaller size or shorter text"
        )));
    }

    let outlines = font.outline_glyphs();
    let mut raster = Rasterizer::new(width as usize, height as usize);
    let mut glyphs = 0;
    let mut missing = 0;
    let baseline0 = pad + ascent;
    for (i, line) in lines.iter().enumerate() {
        let baseline = baseline0 + i as f32 * line_height;
        for (gid, x, y) in &line.glyphs {
            glyphs += 1;
            let Some(outline) = outlines.get(*gid) else {
                missing += 1;
                continue;
            };
            let mut pen = Pen {
                r: &mut raster,
                ox: pad + x,
                oy: baseline - y,
                last: Point { x: 0.0, y: 0.0 },
                start: Point { x: 0.0, y: 0.0 },
            };
            if outline
                .draw(DrawSettings::unhinted(Size::new(size), &location), &mut pen)
                .is_err()
            {
                missing += 1;
            }
        }
    }
    let mut coverage = vec![0u8; (width * height) as usize];
    raster.for_each_pixel_2d(|x, y, a| {
        // The accumulation rasteriser leaves float dust far from any outline; below
        // one percent is not ink.
        let a = if a < 0.01 { 0.0 } else { a.clamp(0.0, 1.0) };
        coverage[(y * width + x) as usize] = (a * 255.0).round() as u8;
    });
    Ok(Bitmap {
        width,
        height,
        coverage,
        baseline: baseline0,
        glyphs,
        missing,
    })
}

/// Render with a face's file (any container; WOFF is unwrapped).
pub fn render_face(face: &FaceMetadata, opts: &RenderOptions) -> Result<Bitmap> {
    let path = std::path::Path::new(&face.file.path);
    let bytes = std::fs::read(path).map_err(|e| Error::Io(path.to_path_buf(), e))?;
    let sfnt = crate::container::unwrap(face.file.container, &bytes)?;
    render_sfnt(&sfnt, face.index, opts)
}

/// The glyph ids the text shapes to, in visual order, without rasterising. Useful for
/// "does this font really handle this text" questions: ligatures collapse, and complex
/// scripts come back as contextual forms rather than the isolated glyphs of `cmap`.
pub fn shaped_glyphs(bytes: &[u8], index: u32, text: &str) -> Result<Vec<u32>> {
    let font = FontRef::from_index(bytes, index)?;
    let data = harfrust::ShaperData::new(&font);
    let instance = harfrust::ShaperInstance::default();
    Ok(shape_line(&font, &data, &instance, &[], text, 1.0)
        .glyphs
        .iter()
        .map(|(g, _, _)| g.to_u32())
        .collect())
}
