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

pub mod check;
pub mod container;
pub mod css;
pub mod error;
pub mod index;
pub mod license;
pub mod model;
pub mod parse;
pub mod scan;
pub mod specimen;
pub mod unicode;

pub use check::{CheckReport, Finding, Severity, check_face};
pub use error::{Error, Result};
pub use index::{
    ActivationRecord, ActivationState, CollectionExport, CollectionFace, CollectionInfo, Conflict,
    DuplicateGroup, FaceFilter, FaceSummary, Facets, Family, ImportReport, Index, Source,
    SourceKind, TagInfo,
};
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

/// JSON Schema for a collection export (`unifont collection export`).
pub fn collection_schema() -> serde_json::Value {
    serde_json::to_value(schemars::schema_for!(CollectionExport)).expect("schema serialises")
}

/// JSON Schema with one definition per type the CLI prints with `--json`.
pub fn cli_output_schema() -> serde_json::Value {
    use schemars::{JsonSchema, SchemaGenerator, generate::SchemaSettings};
    let mut g = SchemaGenerator::new(SchemaSettings::draft2020_12());
    fn add<T: JsonSchema>(g: &mut SchemaGenerator) {
        g.subschema_for::<T>();
    }
    add::<FaceSummary>(&mut g);
    add::<Family>(&mut g);
    add::<Facets>(&mut g);
    add::<DuplicateGroup>(&mut g);
    add::<index::Stats>(&mut g);
    add::<ScanReport>(&mut g);
    add::<CheckReport>(&mut g);
    add::<unicode::BlockCoverage>(&mut g);
    add::<TagInfo>(&mut g);
    add::<CollectionInfo>(&mut g);
    add::<CollectionExport>(&mut g);
    add::<ImportReport>(&mut g);
    add::<Source>(&mut g);
    add::<ActivationRecord>(&mut g);
    add::<Conflict>(&mut g);
    let defs = g.take_definitions(true);
    serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": "unifont CLI output",
        "description": "Every type `unifont --json` prints, one definition each.",
        "$defs": defs,
    })
}
