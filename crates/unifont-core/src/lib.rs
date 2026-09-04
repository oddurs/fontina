//! unifont-core: font parsing, metadata model and a local SQLite index.
//!
//! The crate has no UI dependencies. It is consumed by the `unifont` CLI and the
//! desktop app, and is usable on its own.
//!
//! - [`model`]: the serialisable metadata model (`FaceMetadata` is the root; a JSON
//!   Schema for it is published in `schemas/face.json`).
//! - [`container`]: container detection and WOFF/WOFF2 unwrapping to raw sfnt bytes.
//! - [`parse`]: extraction of metadata from sfnt bytes via fontations.
//! - [`index`]: the SQLite index with full-text search and facet queries.
//! - [`scan`]: parallel directory scanning that feeds the index.
//! - [`css`]: `@font-face` rule generation (CSS Fonts Level 4 is the style model).

pub mod container;
pub mod css;
pub mod error;
pub mod index;
pub mod license;
pub mod model;
pub mod parse;
pub mod scan;
mod unicode;

pub use error::{Error, Result};
pub use index::{DuplicateGroup, FaceFilter, FaceSummary, Index};
pub use model::{Container, FaceMetadata, FileInfo};
pub use scan::{ScanOptions, ScanReport};

/// Version of the `FaceMetadata` JSON shape. Bump when the schema changes incompatibly.
pub const SCHEMA_VERSION: u32 = 1;

/// Parse a font file on disk into one `FaceMetadata` per face (collections yield several).
pub fn load_file(path: &std::path::Path) -> Result<(FileInfo, Vec<FaceMetadata>)> {
    let bytes = std::fs::read(path).map_err(|e| Error::Io(path.to_path_buf(), e))?;
    let meta = std::fs::metadata(path).map_err(|e| Error::Io(path.to_path_buf(), e))?;
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let container =
        Container::detect(&bytes).ok_or_else(|| Error::UnknownFormat(path.to_path_buf()))?;
    let sfnt = container::unwrap(container, &bytes)?;
    let mut file = FileInfo {
        path: path.to_string_lossy().into_owned(),
        size: meta.len(),
        mtime,
        blake3: blake3::hash(&bytes).to_hex().to_string(),
        container,
        face_count: 0,
    };
    let faces = parse::parse_sfnt(&sfnt, &file)?;
    file.face_count = faces.len() as u32;
    let faces = faces
        .into_iter()
        .map(|mut f| {
            f.file.face_count = file.face_count;
            f
        })
        .collect();
    Ok((file, faces))
}

/// JSON Schema (draft 2020-12) for `FaceMetadata`.
pub fn face_schema() -> serde_json::Value {
    serde_json::to_value(schemars::schema_for!(FaceMetadata)).expect("schema serialises")
}
