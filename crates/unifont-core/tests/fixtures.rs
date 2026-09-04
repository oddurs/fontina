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

#[test]
fn health_checks_pass_on_well_formed_fixtures() {
    for name in [
        "Amiri-Regular.ttf",
        "BricolageGrotesque[opsz,wdth,wght].ttf",
        "SourceSerif4-Regular.otf",
    ] {
        let (_, faces) = load_file(&fixture(name)).unwrap();
        let r = unifont_core::check_face(&faces[0]);
        assert_eq!(r.errors, 0, "{name}: {:?}", r.findings);
        assert_eq!(r.warnings, 0, "{name}: {:?}", r.findings);
        assert!(r.passed(true));
    }
}

#[test]
fn health_checks_flag_broken_metadata() {
    let (_, mut faces) = load_file(&fixture("Amiri-Regular.ttf")).unwrap();
    let f = &mut faces[0];
    f.names.postscript_name = Some(
        "Has Spaces And Is Far Too Long For A PostScript Name To Be Accepted By Anything".into(),
    );
    f.os2.as_mut().unwrap().weight_class = 1200;
    f.features.scripts.clear(); // Arabic coverage without an arab script: shaping warning
    f.license.spdx = None;
    let r = unifont_core::check_face(f);
    let ids: Vec<&str> = r.findings.iter().map(|x| x.id).collect();
    assert!(ids.contains(&"name/postscript"), "{ids:?}");
    assert!(ids.contains(&"os2/weight-class"), "{ids:?}");
    assert!(ids.contains(&"layout/shaping"), "{ids:?}");
    assert!(ids.contains(&"license/missing"), "{ids:?}");
    assert!(r.errors >= 2);
    assert!(!r.passed(false));
    // Findings are ordered most severe first.
    assert!(
        r.findings
            .windows(2)
            .all(|w| w[0].severity >= w[1].severity)
    );
}

#[test]
fn glyph_map_groups_by_unicode_block() {
    let (_, faces) = load_file(&fixture("Amiri-Regular.ttf")).unwrap();
    let blocks = unifont_core::unicode::glyph_map(&faces[0].coverage.ranges);
    let arabic = blocks
        .iter()
        .find(|b| b.block == "Arabic")
        .expect("Arabic block");
    assert_eq!(arabic.start, 0x0600);
    assert_eq!(arabic.block_size, 256);
    assert!(arabic.codepoints.len() > 200);
    let total: usize = blocks.iter().map(|b| b.codepoints.len()).sum();
    assert_eq!(total as u32, faces[0].coverage.codepoints);
}

#[test]
fn covering_finds_faces_for_text_and_migrates_old_indexes() {
    let mut index = Index::open_in_memory().unwrap();
    unifont_core::scan::scan(&mut index, &[fixture("")], &ScanOptions::default()).unwrap();
    let latin = index
        .covering("Sphinx of black quartz", &FaceFilter::default())
        .unwrap();
    assert!(latin.len() >= 5, "{latin:?}");
    let arabic = index.covering("صِف خَلقَ", &FaceFilter::default()).unwrap();
    assert_eq!(arabic.len(), 1);
    assert_eq!(arabic[0].family, "Amiri");
    let none = index.covering("視野", &FaceFilter::default()).unwrap();
    assert!(none.is_empty());
    assert!(
        index
            .covering("   ", &FaceFilter::default())
            .unwrap()
            .is_empty()
    );

    // A v1 database (no face_ranges) is upgraded and backfilled on open.
    let dir = std::env::temp_dir().join(format!("unifont-migrate-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let db = dir.join("v1.db");
    {
        let mut idx = Index::open(&db).unwrap();
        unifont_core::scan::scan(
            &mut idx,
            &[fixture("Amiri-Regular.ttf")],
            &ScanOptions::default(),
        )
        .unwrap();
    }
    {
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.execute_batch("DROP TABLE face_ranges; PRAGMA user_version = 1;")
            .unwrap();
    }
    let idx = Index::open(&db).unwrap();
    let hits = idx.covering("صِف", &FaceFilter::default()).unwrap();
    assert_eq!(hits.len(), 1);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn specimen_is_self_contained_html() {
    let (_, a) = load_file(&fixture("BricolageGrotesque[opsz,wdth,wght].ttf")).unwrap();
    let (_, b) = load_file(&fixture("Amiri-Regular.ttf")).unwrap();
    let faces = vec![a[0].clone(), b[0].clone()];
    let html =
        unifont_core::specimen::render(&faces, &unifont_core::specimen::SpecimenOptions::default())
            .unwrap();
    assert!(html.starts_with("<!doctype html>"));
    assert_eq!(html.matches("@font-face").count(), 2);
    assert!(html.contains("data:font/ttf;base64,"));
    assert_eq!(
        html.matches("class=\"axis\"").count(),
        3,
        "three axis sliders"
    );
    assert!(html.contains("dir=\"rtl\""), "Arabic sample paragraph");
    assert!(
        html.contains("<section class=\"compare\">"),
        "compare section for two faces"
    );
    assert!(html.contains("font-weight:200 800"));
    let linked = unifont_core::specimen::render(
        &faces[..1],
        &unifont_core::specimen::SpecimenOptions {
            link: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(linked.contains("url(\"file://"));
    assert!(!linked.contains("base64"));
    assert!(!linked.contains("<section class=\"compare\">"));
}
