//! `@font-face` generation. One rule per face, CSS Fonts Level 4 descriptors.

use crate::model::FaceMetadata;
use crate::parse::unicode_range;

/// Render one `@font-face` rule. `src_url` overrides the file path when given (for a
/// custom protocol in the desktop app, or a relative web path).
pub fn font_face_rule(face: &FaceMetadata, src_url: Option<&str>) -> String {
    let css = &face.style.css;
    let url = src_url
        .map(String::from)
        .unwrap_or_else(|| format!("file://{}", face.file.path.replace('\\', "/")));
    let mut src = format!("url(\"{}\") format(\"{}\")", url, css.format);
    if face.file.face_count > 1 {
        // Collections: the fragment selects the face per CSS Fonts 4 §4.3.
        src = format!("url(\"{}#{}\") format(\"{}\")", url, face.index, css.format);
    }
    let mut out = String::new();
    out.push_str("@font-face {\n");
    out.push_str(&format!(
        "  font-family: \"{}\";\n",
        css.family.replace('"', "\\\"")
    ));
    out.push_str(&format!("  font-style: {};\n", css.style));
    out.push_str(&format!("  font-weight: {};\n", css.weight));
    out.push_str(&format!("  font-stretch: {};\n", css.stretch));
    out.push_str("  font-display: swap;\n");
    out.push_str(&format!("  src: {src};\n"));
    let range = unicode_range(face);
    if !range.is_empty() && face.coverage.ranges.len() <= 512 {
        out.push_str(&format!("  unicode-range: {range};\n"));
    }
    out.push_str("}\n");
    out
}
