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

//! fontina-core: font parsing, metadata model and a local SQLite index.
//!
//! The crate has no UI dependencies. It is consumed by the `fontina` CLI and the
//! desktop app, and is usable on its own.
//!
//! - [`model`]: the serialisable metadata model (`FaceMetadata` is the root; a JSON
//!   Schema for it is published in `schemas/face.json`).
//! - [`container`]: container detection and WOFF/WOFF2 unwrapping to raw sfnt bytes.
//! - [`parse`]: extraction of metadata from sfnt bytes via fontations.
//! - [`index`]: the SQLite index with full-text search and facet queries.
//! - [`scan`]: parallel directory scanning that feeds the index.
//! - [`css`]: `@font-face` rule generation (CSS Fonts Level 4 is the style model).
//! - [`watch`]: follow directories and keep the index current.
//! - [`render`]: shaped, rasterised previews (harfrust + skrifa) and their terminal encodings.
//! - [`freedom`]: whether a font's license grants the four freedoms.
//! - [`typography`]: the judgements a specimen makes, shared by every client.

pub mod check;
pub mod container;
pub mod css;
pub mod error;
pub mod freedom;
pub mod index;
pub mod license;
pub mod model;
pub mod parse;
pub mod render;
pub mod scan;
pub mod specimen;
pub mod typography;
pub mod unicode;
pub mod watch;

pub use check::{CheckReport, Finding, Severity, check_face};
pub use error::{Error, Result};
pub use freedom::Freedom;
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
    let name = path.to_string_lossy();
    let (mut file, faces) = load_bytes(&bytes, &name)?;
    file.size = meta.len();
    file.mtime = mtime;
    let faces = faces
        .into_iter()
        .map(|mut f| {
            f.file.size = file.size;
            f.file.mtime = file.mtime;
            f
        })
        .collect();
    Ok((file, faces))
}

/// Parse font bytes already in memory, as [`load_file`] does without the file I/O.
///
/// `name` becomes the reported path; `size` is the length of `bytes` and `mtime` is 0,
/// because a slice has no directory entry to ask. This is the whole import path —
/// container detection, WOFF/WOFF2 unwrapping, sfnt parsing — in one call, which makes
/// it what `fuzz/fuzz_targets/parse.rs` drives.
pub fn load_bytes(bytes: &[u8], name: &str) -> Result<(FileInfo, Vec<FaceMetadata>)> {
    let container = Container::detect(bytes).ok_or_else(|| Error::UnknownFormat(name.into()))?;
    let sfnt = container::unwrap(container, bytes)?;
    let mut file = FileInfo {
        path: name.to_owned(),
        size: bytes.len() as u64,
        mtime: 0,
        blake3: blake3::hash(bytes).to_hex().to_string(),
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

/// JSON Schema for a collection export (`fontina collection export`).
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
    add::<watch::WatchEvent>(&mut g);
    let defs = g.take_definitions(true);
    serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": "fontina CLI output",
        "description": "Every type `fontina --json` prints, one definition each.",
        "$defs": defs,
    })
}
