//! Metadata extraction against the OFL fixtures in `/fixtures`. Snapshots live in
//! `tests/snapshots`; review changes with `cargo insta review`.

use std::path::PathBuf;
use unifont_core::model::*;
use unifont_core::{FaceFilter, Index, ScanOptions, load_file};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name)
}

/// The stable, reviewable subset of a face: everything except file paths, hashes and
/// timestamps that vary between machines.
fn snapshot_view(f: &FaceMetadata) -> serde_json::Value {
    serde_json::json!({
        "container": f.file.container,
        "index": f.index,
        "family": f.names.family,
        "subfamily": f.names.subfamily,
        "postscript_name": f.names.postscript_name,
        "designer": f.names.designer,
        "style": f.style,
        "units_per_em": f.metrics.units_per_em,
        "variable": f.variable,
        "features": f.features,
        "scripts": f.coverage.scripts,
        "codepoints": f.coverage.codepoints,
        "capabilities": f.capabilities,
        "license": f.license.spdx,
        "embedding": f.os2.as_ref().map(|o| &o.embedding),
        "glyph_count": f.glyph_count,
    })
}

#[test]
fn amiri_is_arabic_ofl_static() {
    let (file, faces) = load_file(&fixture("Amiri-Regular.ttf")).unwrap();
    assert_eq!(file.container, Container::Ttf);
    assert_eq!(faces.len(), 1);
    let f = &faces[0];
    assert_eq!(f.names.family, "Amiri");
    assert_eq!(f.license.spdx.as_deref(), Some("OFL-1.1"));
    assert!(f.variable.is_none());
    assert_eq!(f.coverage.scripts[0].script, "Arab");
    assert!(f.features.gsub.contains(&"init".to_string()));
    assert!(f.features.scripts.iter().any(|s| s.tag == "arab"));
    insta::assert_json_snapshot!("amiri", snapshot_view(f));
}

#[test]
fn bricolage_is_variable_with_three_axes() {
    let (_, faces) = load_file(&fixture("BricolageGrotesque[opsz,wdth,wght].ttf")).unwrap();
    let f = &faces[0];
    let v = f.variable.as_ref().expect("variable");
    let tags: Vec<&str> = v.axes.iter().map(|a| a.tag.as_str()).collect();
    assert_eq!(tags, ["opsz", "wght", "wdth"]);
    assert_eq!(f.style.css.weight, "200 800");
    assert_eq!(f.style.css.stretch, "75% 100%");
    assert_eq!(v.instances.len(), 7);
    assert!(v.has_stat);
    insta::assert_json_snapshot!("bricolage", snapshot_view(f));
}

#[test]
fn nabla_is_colrv1_and_svg() {
    let (_, faces) = load_file(&fixture("Nabla[EDPT,EHLT].ttf")).unwrap();
    let f = &faces[0];
    assert!(f.capabilities.color.contains(&ColorFormat::Colrv1));
    assert!(f.capabilities.color.contains(&ColorFormat::Svg));
    assert!(f.is_color());
    insta::assert_json_snapshot!("nabla", snapshot_view(f));
}

#[test]
fn source_serif_is_cff() {
    let (file, faces) = load_file(&fixture("SourceSerif4-Regular.otf")).unwrap();
    assert_eq!(file.container, Container::Otf);
    assert_eq!(faces[0].capabilities.outlines, OutlineFormat::Cff);
    assert_eq!(faces[0].style.css.format, "opentype");
    insta::assert_json_snapshot!("source-serif", snapshot_view(&faces[0]));
}

#[test]
fn woff_and_woff2_decode_to_the_same_face() {
    let (f1, a) = load_file(&fixture("inter-latin-400-normal.woff")).unwrap();
    let (f2, b) = load_file(&fixture("inter-latin-400-normal.woff2")).unwrap();
    assert_eq!(f1.container, Container::Woff);
    assert_eq!(f2.container, Container::Woff2);
    let (a, b) = (&a[0], &b[0]);
    assert_eq!(a.names.postscript_name, b.names.postscript_name);
    assert_eq!(a.names.family, "Inter");
    // The two Fontsource builds are separate subsets, so coverage is close but not equal.
    assert!(a.coverage.codepoints.abs_diff(b.coverage.codepoints) < 10);
    assert_eq!(a.coverage.scripts[0].script, "Latn");
    assert_eq!(a.capabilities.outlines, OutlineFormat::Glyf);
    assert_eq!(b.capabilities.outlines, OutlineFormat::Glyf);
    assert_eq!(a.license.spdx.as_deref(), Some("OFL-1.1"));
    assert_eq!(a.style.css.format, "woff");
    assert_eq!(b.style.css.format, "woff2");
}

#[test]
fn every_face_validates_against_the_schema_shape() {
    // Round-trip through JSON: serialisation must be lossless for storage in the index.
    for name in [
        "Amiri-Regular.ttf",
        "BricolageGrotesque[opsz,wdth,wght].ttf",
        "inter-latin-400-normal.woff2",
    ] {
        let (_, faces) = load_file(&fixture(name)).unwrap();
        let json = serde_json::to_string(&faces[0]).unwrap();
        let back: FaceMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(back.identity_hash, faces[0].identity_hash);
        assert_eq!(back.schema_version, unifont_core::SCHEMA_VERSION);
    }
    let schema = unifont_core::face_schema();
    assert_eq!(schema["title"], "FaceMetadata");
}

#[test]
fn index_scan_list_filter_and_dupes() {
    let mut index = Index::open_in_memory().unwrap();
    let report =
        unifont_core::scan::scan(&mut index, &[fixture("")], &ScanOptions::default()).unwrap();
    assert_eq!(report.parsed, 6, "failures: {:?}", report.failed);
    assert_eq!(report.faces, 6);

    // Second scan is a no-op.
    let again =
        unifont_core::scan::scan(&mut index, &[fixture("")], &ScanOptions::default()).unwrap();
    assert_eq!(again.unchanged, 6);
    assert_eq!(again.parsed, 0);

    let all = index.list(&FaceFilter::default()).unwrap();
    assert_eq!(all.len(), 6);
    let arabic = index
        .list(&FaceFilter {
            script: Some("Arab".into()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(arabic.len(), 1);
    assert_eq!(arabic[0].family, "Amiri");
    let variable = index
        .list(&FaceFilter {
            variable: Some(true),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(variable.len(), 2);
    let fts = index
        .list(&FaceFilter {
            query: Some("bric gro".into()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(fts.len(), 1);
    let by_designer = index
        .list(&FaceFilter {
            query: Some("hosny".into()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(by_designer.len(), 1);
    let ofl = index
        .list(&FaceFilter {
            license: Some("OFL".into()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(ofl.len(), 6);

    let dupes = index.duplicates().unwrap();
    assert_eq!(dupes.len(), 1, "{dupes:?}");
    assert_eq!(dupes[0].faces.len(), 2);
    assert_eq!(dupes[0].key, "Inter-Regular");

    let face = index.get_face(all[0].id).unwrap().unwrap();
    assert_eq!(face.names.family, all[0].family);
    let stats = index.stats().unwrap();
    assert_eq!(stats.faces, 6);
    assert_eq!(stats.families, 5);
}

#[test]
fn malformed_input_is_an_error_not_a_panic() {
    let dir = std::env::temp_dir().join(format!("unifont-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let bad = dir.join("bad.ttf");
    std::fs::write(&bad, b"\x00\x01\x00\x00garbagegarbagegarbage").unwrap();
    assert!(load_file(&bad).is_err());
    let notfont = dir.join("text.otf");
    std::fs::write(&notfont, b"hello").unwrap();
    assert!(matches!(
        load_file(&notfont),
        Err(unifont_core::Error::UnknownFormat(_))
    ));
    let mut index = Index::open_in_memory().unwrap();
    let report = unifont_core::scan::scan(
        &mut index,
        std::slice::from_ref(&dir),
        &ScanOptions::default(),
    )
    .unwrap();
    assert_eq!(report.failed.len(), 2);
    assert_eq!(index.stats().unwrap().failed_files, 2);
    std::fs::remove_dir_all(&dir).ok();
}
