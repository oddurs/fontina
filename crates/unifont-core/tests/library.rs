//! Tags, collections, sources, activation state, facets and families against the
//! fixture fonts in an in-memory index.

use std::path::PathBuf;
use unifont_core::{ActivationState, CollectionExport, FaceFilter, Index, ScanOptions, SourceKind};

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

fn indexed() -> Index {
    let mut index = Index::open_in_memory().unwrap();
    let report =
        unifont_core::scan::scan(&mut index, &[fixtures()], &ScanOptions::default()).unwrap();
    assert_eq!(report.faces, 6, "{:?}", report.failed);
    index
}

fn id_of(index: &Index, family: &str) -> i64 {
    index
        .list(&FaceFilter {
            family: Some(family.into()),
            ..Default::default()
        })
        .unwrap()[0]
        .id
}

#[test]
fn tags_round_trip_and_filter() {
    let mut index = indexed();
    let amiri = id_of(&index, "Amiri");
    let serif = id_of(&index, "Source Serif 4");
    assert_eq!(index.tag(&[amiri, serif], "serif").unwrap(), 2);
    assert_eq!(index.tag(&[amiri], "serif").unwrap(), 0, "idempotent");
    assert_eq!(index.tag(&[amiri], "Arabic").unwrap(), 1);
    assert!(index.tag(&[amiri], "  ").is_err(), "blank tag rejected");

    let tags = index.tags().unwrap();
    assert_eq!(
        tags.iter()
            .map(|t| (t.name.as_str(), t.faces))
            .collect::<Vec<_>>(),
        [("Arabic", 1), ("serif", 2)]
    );
    let tagged = index
        .list(&FaceFilter {
            tag: Some("SERIF".into()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(tagged.len(), 2);
    assert_eq!(
        tagged[0].tags,
        ["Arabic", "serif"],
        "tags ride on summaries, sorted"
    );

    assert!(index.rename_tag("serif", "Serif Fonts").unwrap());
    assert_eq!(index.untag(&[serif], "serif fonts").unwrap(), 1);
    assert!(index.delete_tag("arabic").unwrap());
    assert!(!index.delete_tag("nope").unwrap());
    assert_eq!(index.summaries(&[amiri]).unwrap()[0].tags, ["Serif Fonts"]);
}

#[test]
fn collections_keep_order_and_export_import() {
    let mut index = indexed();
    let serif = id_of(&index, "Source Serif 4");
    let amiri = id_of(&index, "Amiri");
    let nabla = id_of(&index, "Nabla");
    assert_eq!(
        index
            .add_to_collection("Editorial", &[serif, amiri])
            .unwrap(),
        2
    );
    assert_eq!(
        index
            .add_to_collection("editorial", &[nabla, amiri])
            .unwrap(),
        1
    );
    let faces = index.collection_faces("Editorial").unwrap();
    assert_eq!(
        faces.iter().map(|f| f.id).collect::<Vec<_>>(),
        [serif, amiri, nabla],
        "insertion order, not family order"
    );
    assert_eq!(index.collections().unwrap()[0].faces, 3);
    assert!(index.collection_faces("nope").is_err());
    let filtered = index
        .list(&FaceFilter {
            collection: Some("Editorial".into()),
            italic: Some(false),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(filtered.len(), 3);

    index.tag(&[amiri], "arabic").unwrap();
    let export = index.export_collection("Editorial").unwrap();
    assert_eq!(export.name, "Editorial");
    assert_eq!(export.schema_version, unifont_core::SCHEMA_VERSION);
    assert_eq!(export.faces.len(), 3);
    assert_eq!(export.faces[1].tags, ["arabic"]);
    assert!(export.exported_at.contains('T'), "{}", export.exported_at);
    let json = serde_json::to_string(&export).unwrap();
    let back: CollectionExport = serde_json::from_str(&json).unwrap();

    // Import into a fresh index whose paths differ: identity hashes still match.
    let mut other = indexed();
    let mut moved = back.clone();
    for f in &mut moved.faces {
        f.path = format!("/elsewhere/{}", f.path.rsplit('/').next().unwrap());
    }
    moved.faces.push(unifont_core::CollectionFace {
        family: "Ghost".into(),
        subfamily: "Regular".into(),
        postscript_name: Some("Ghost-Regular".into()),
        identity_hash: "0000".into(),
        blake3: "0000".into(),
        path: "/nowhere/Ghost.ttf".into(),
        index: 0,
        tags: vec![],
    });
    let report = other.import_collection(&moved, None, true).unwrap();
    assert_eq!(report.matched, 3);
    assert_eq!(report.missing.len(), 1);
    assert_eq!(report.missing[0].family, "Ghost");
    assert_eq!(report.tags_applied, 1);
    let imported = other.collection_faces("Editorial").unwrap();
    assert_eq!(imported.len(), 3);
    assert_eq!(imported[1].family, "Amiri");
    assert_eq!(imported[1].tags, ["arabic"]);

    let renamed = other.import_collection(&back, Some("Copy"), false).unwrap();
    assert_eq!(renamed.collection, "Copy");
    assert_eq!(other.collections().unwrap().len(), 2);
    assert!(other.rename_collection("Copy", "Copy 2").unwrap());
    assert!(other.delete_collection("copy 2").unwrap());
    assert_eq!(
        other.remove_from_collection("Editorial", &[serif]).unwrap(),
        1
    );
    assert_eq!(other.collection_faces("Editorial").unwrap().len(), 2);

    let newer = CollectionExport {
        schema_version: unifont_core::SCHEMA_VERSION + 1,
        ..back
    };
    assert!(other.import_collection(&newer, None, false).is_err());
}

#[test]
fn sources_are_recorded_by_scan_and_managed() {
    let mut index = indexed();
    let root = std::fs::canonicalize(fixtures()).unwrap();
    let sources = index.sources().unwrap();
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].path, root.to_string_lossy());
    assert_eq!(sources[0].kind, SourceKind::User);
    assert!(
        sources[0].watch,
        "explicit directories are watched by default"
    );

    assert!(index.set_source_watch(&sources[0].path, false).unwrap());
    assert!(!index.sources().unwrap()[0].watch);
    let sys = index
        .add_source("/nonexistent/system/fonts", false, SourceKind::System)
        .unwrap();
    assert_eq!(sys.kind, SourceKind::System);
    assert_eq!(index.sources().unwrap().len(), 2);
    assert!(
        index
            .remove_source("/nonexistent/system/fonts", false)
            .unwrap()
    );
    assert!(
        !index
            .remove_source("/nonexistent/system/fonts", false)
            .unwrap()
    );

    // Purging a source drops its faces.
    assert!(index.remove_source(&sources[0].path, true).unwrap());
    assert_eq!(index.stats().unwrap().faces, 0);
}

#[test]
fn activation_state_filters_and_survives_rescan() {
    let mut index = indexed();
    let amiri = id_of(&index, "Amiri");
    let nabla = id_of(&index, "Nabla");
    index
        .set_activation(&[amiri], ActivationState::Session, None)
        .unwrap();
    index
        .set_activation(
            &[nabla],
            ActivationState::Installed,
            Some("/home/u/.local/share/fonts/unifont/Nabla.ttf"),
        )
        .unwrap();
    let active = index
        .list(&FaceFilter {
            active: Some(true),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(active.len(), 2);
    assert_eq!(active[0].activation, Some(ActivationState::Session));
    let inactive = index
        .list(&FaceFilter {
            active: Some(false),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(inactive.len(), 4);
    let installed = index
        .list(&FaceFilter {
            activation: Some(ActivationState::Installed),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(installed.len(), 1);
    let rec = index.activation(nabla).unwrap().unwrap();
    assert_eq!(rec.state, ActivationState::Installed);
    assert!(
        rec.installed_path
            .as_deref()
            .unwrap()
            .ends_with("Nabla.ttf")
    );
    assert_eq!(index.activations().unwrap().len(), 2);

    // A forced rescan replaces the rows; user data carries over by (path, face index).
    index.tag(&[amiri], "kept").unwrap();
    index.add_to_collection("Kept", &[amiri]).unwrap();
    let report = unifont_core::scan::scan(
        &mut index,
        &[fixtures()],
        &ScanOptions {
            force: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(report.parsed, 6);
    let amiri2 = id_of(&index, "Amiri");
    assert_ne!(amiri, amiri2, "rows were replaced");
    let s = index.summaries(&[amiri2]).unwrap().remove(0);
    assert_eq!(s.tags, ["kept"]);
    assert_eq!(s.activation, Some(ActivationState::Session));
    assert_eq!(index.collection_faces("Kept").unwrap()[0].id, amiri2);
    assert_eq!(index.activations().unwrap().len(), 2);
    assert_eq!(index.clear_activation(&[amiri2]).unwrap(), 1);
    assert_eq!(index.activations().unwrap().len(), 1);
    assert_eq!(index.file_faces(amiri2).unwrap(), [amiri2]);
}

#[test]
fn conflicts_see_active_and_system_faces_only() {
    let mut index = indexed();
    let woff = index
        .list(&FaceFilter {
            container: Some("woff".into()),
            ..Default::default()
        })
        .unwrap()[0]
        .id;
    let woff2 = index
        .list(&FaceFilter {
            container: Some("woff2".into()),
            ..Default::default()
        })
        .unwrap()[0]
        .id;
    // Same PostScript name, but neither is active or in a system directory: no conflict.
    assert!(index.conflicts(woff, &[]).unwrap().is_empty());
    index
        .set_activation(&[woff2], ActivationState::User, None)
        .unwrap();
    let c = index.conflicts(woff, &[]).unwrap();
    assert_eq!(c.len(), 1);
    assert_eq!(c[0].face.id, woff2);
    assert_eq!(c[0].reason, "same PostScript name, active (user)");
    index.clear_activation(&[woff2]).unwrap();
    // Treat the fixtures directory as a system font directory.
    let root = std::fs::canonicalize(fixtures())
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let c = index.conflicts(woff, &[root]).unwrap();
    assert_eq!(c.len(), 1);
    assert!(c[0].reason.ends_with("present in a system font directory"));
    assert!(index.conflicts(99999, &[]).is_err());
}

#[test]
fn facets_count_every_dimension() {
    let mut index = indexed();
    let amiri = id_of(&index, "Amiri");
    index.tag(&[amiri], "arabic").unwrap();
    index.add_to_collection("Editorial", &[amiri]).unwrap();
    index
        .set_activation(&[amiri], ActivationState::Session, None)
        .unwrap();
    let f = index.facets(&FaceFilter::default()).unwrap();
    assert_eq!(f.faces, 6);
    assert_eq!(f.families, 5);
    assert_eq!(f.variable, 2);
    assert_eq!(f.color, 1);
    let get = |v: &[unifont_core::index::FacetCount], k: &str| {
        v.iter().find(|c| c.value == k).map(|c| c.count)
    };
    assert_eq!(get(&f.weight, "400"), Some(5), "{:?}", f.weight);
    assert_eq!(
        get(&f.weight, "800"),
        Some(1),
        "Bricolage defaults to ExtraBold"
    );
    assert_eq!(get(&f.width, "100"), Some(6), "{:?}", f.width);
    assert_eq!(get(&f.style, "upright"), Some(6));
    assert_eq!(get(&f.container, "ttf"), Some(3));
    assert_eq!(get(&f.container, "woff2"), Some(1));
    assert_eq!(get(&f.script, "Arab"), Some(1));
    assert!(get(&f.script, "Latn").unwrap() >= 5);
    assert_eq!(get(&f.license, "OFL-1.1"), Some(6));
    assert_eq!(get(&f.tag, "arabic"), Some(1));
    assert_eq!(get(&f.collection, "Editorial"), Some(1));
    assert_eq!(get(&f.activation, "session"), Some(1));
    assert_eq!(get(&f.activation, "none"), Some(5));
    assert_eq!(f.source.len(), 1);
    assert_eq!(f.source[0].count, 6);
    assert!(!f.vendor.is_empty());

    // Facets follow the filter.
    let arabic = index
        .facets(&FaceFilter {
            script: Some("Arab".into()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(arabic.faces, 1);
    assert_eq!(get(&arabic.tag, "arabic"), Some(1));
    assert_eq!(get(&arabic.activation, "none"), None);
    assert_eq!(unifont_core::index::weight_name(700), "Bold");
    assert_eq!(unifont_core::index::weight_bucket(651.0), 700);
    assert_eq!(unifont_core::index::width_bucket(80.0), 75.0);
    assert_eq!(unifont_core::index::width_name(75.0), "Condensed");
}

#[test]
fn families_group_faces_and_pick_a_representative() {
    let mut index = indexed();
    let fams = index.families(&FaceFilter::default()).unwrap();
    assert_eq!(fams.len(), 5);
    let inter = fams.iter().find(|f| f.name == "Inter").unwrap();
    assert_eq!(inter.faces, 2, "woff and woff2 of the same face");
    assert_eq!(inter.containers, ["woff", "woff2"]);
    assert_eq!(inter.weights, [400.0, 400.0]);
    let bricolage = fams
        .iter()
        .find(|f| f.name == "Bricolage Grotesque")
        .unwrap();
    assert!(bricolage.variable);
    assert_eq!(bricolage.faces, 1);
    assert_eq!(bricolage.representative, bricolage.ids[0]);
    let nabla = fams.iter().find(|f| f.name == "Nabla").unwrap();
    assert!(nabla.color);
    index.tag(&nabla.ids, "fun").unwrap();
    let limited = index
        .families(&FaceFilter {
            limit: Some(2),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(limited.len(), 2);
    let tagged = index
        .families(&FaceFilter {
            tag: Some("fun".into()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(tagged.len(), 1);
    assert_eq!(tagged[0].tags, ["fun"]);
    let by_ids = index
        .list(&FaceFilter {
            ids: Some(inter.ids.clone()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(by_ids.len(), 2);
    let width = index
        .list(&FaceFilter {
            width: Some((50, 99)),
            ..Default::default()
        })
        .unwrap();
    assert!(width.is_empty());
    let vendor = index
        .list(&FaceFilter {
            vendor: by_ids[0].vendor.clone(),
            ..Default::default()
        })
        .unwrap();
    assert!(vendor.len() >= 2);
}

#[test]
fn schemas_cover_the_new_types() {
    let coll = unifont_core::collection_schema();
    assert_eq!(coll["title"], "CollectionExport");
    let cli = unifont_core::cli_output_schema();
    let defs = cli["$defs"].as_object().unwrap();
    for name in [
        "FaceSummary",
        "Family",
        "Facets",
        "DuplicateGroup",
        "Stats",
        "ScanReport",
        "CheckReport",
        "BlockCoverage",
        "TagInfo",
        "CollectionInfo",
        "CollectionExport",
        "ImportReport",
        "Source",
        "ActivationRecord",
        "Conflict",
        "ActivationState",
    ] {
        assert!(defs.contains_key(name), "missing {name}");
    }
}

#[test]
fn watch_applies_file_and_directory_changes() {
    use std::collections::BTreeSet;
    let dir = std::env::temp_dir().join(format!("unifont-watch-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("sub")).unwrap();
    let mut index = Index::open_in_memory().unwrap();
    unifont_core::scan::scan(
        &mut index,
        std::slice::from_ref(&dir),
        &ScanOptions::default(),
    )
    .unwrap();
    let roots = vec![std::fs::canonicalize(&dir).unwrap()];
    let opts = unifont_core::watch::WatchOptions::default();

    // A new file is parsed on its own.
    let amiri = roots[0].join("Amiri-Regular.ttf");
    std::fs::copy(fixtures().join("Amiri-Regular.ttf"), &amiri).unwrap();
    let ev = unifont_core::watch::apply(&mut index, &roots, &opts, BTreeSet::from([amiri.clone()]))
        .unwrap();
    assert_eq!(ev.report.parsed, 1);
    assert_eq!(ev.paths, [amiri.to_string_lossy().into_owned()]);
    assert_eq!(index.stats().unwrap().faces, 1);

    // Non-font and unchanged paths are no-ops.
    let readme = roots[0].join("README.txt");
    std::fs::write(&readme, "hi").unwrap();
    let ev = unifont_core::watch::apply(
        &mut index,
        &roots,
        &opts,
        BTreeSet::from([readme, amiri.clone()]),
    )
    .unwrap();
    assert_eq!(ev.report.parsed, 0);
    assert_eq!(ev.report.unchanged, 1);

    // A directory event rescans it with pruning; a removed file is dropped.
    let sub = roots[0].join("sub");
    std::fs::copy(
        fixtures().join("SourceSerif4-Regular.otf"),
        sub.join("S.otf"),
    )
    .unwrap();
    std::fs::remove_file(&amiri).unwrap();
    let ev = unifont_core::watch::apply(
        &mut index,
        &roots,
        &opts,
        BTreeSet::from([sub.clone(), amiri.clone()]),
    )
    .unwrap();
    assert_eq!(ev.report.parsed, 1, "{ev:?}");
    assert_eq!(ev.report.removed, 1, "{ev:?}");
    assert_eq!(index.stats().unwrap().faces, 1);

    // A directory that vanished takes its files with it.
    std::fs::remove_dir_all(&sub).unwrap();
    let ev = unifont_core::watch::apply(&mut index, &roots, &opts, BTreeSet::from([sub])).unwrap();
    assert_eq!(ev.report.removed, 1, "{ev:?}");
    assert_eq!(index.stats().unwrap().faces, 0);

    // The live watcher delivers a batch for a copied file.
    let (tx, rx) = std::sync::mpsc::channel();
    let root_for_thread = roots[0].clone();
    let handle = std::thread::spawn(move || {
        let mut index = Index::open_in_memory().unwrap();
        unifont_core::watch::watch(
            &mut index,
            &[root_for_thread],
            &unifont_core::watch::WatchOptions {
                debounce: std::time::Duration::from_millis(200),
                ..Default::default()
            },
            |ev| {
                tx.send(ev.report.parsed).unwrap();
                false
            },
        )
        .unwrap();
    });
    std::thread::sleep(std::time::Duration::from_millis(500));
    std::fs::copy(
        fixtures().join("Nabla[EDPT,EHLT].ttf"),
        roots[0].join("Nabla.ttf"),
    )
    .unwrap();
    let parsed = rx
        .recv_timeout(std::time::Duration::from_secs(15))
        .expect("watcher reported the new file");
    assert_eq!(parsed, 1);
    handle.join().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn render_shapes_and_rasterises() {
    use unifont_core::render::{RenderOptions, encode, render_face, shaped_glyphs};
    let (_, faces) = unifont_core::load_file(&fixtures().join("Amiri-Regular.ttf")).unwrap();
    let bytes = std::fs::read(fixtures().join("Amiri-Regular.ttf")).unwrap();
    // Shaping is real: Arabic letters come back as contextual forms, not the isolated
    // glyphs, and Latin ligatures collapse.
    let word = shaped_glyphs(&bytes, 0, "سلام").unwrap();
    assert_eq!(word.len(), 4);
    let isolated: Vec<u32> = "سلام"
        .chars()
        .map(|c| shaped_glyphs(&bytes, 0, &c.to_string()).unwrap()[0])
        .collect();
    assert_ne!(word, isolated);
    let serif = std::fs::read(fixtures().join("SourceSerif4-Regular.otf")).unwrap();
    assert_eq!(
        shaped_glyphs(&serif, 0, "fi").unwrap().len(),
        1,
        "fi ligature"
    );
    assert_eq!(shaped_glyphs(&serif, 0, "ab").unwrap().len(), 2);

    let bm = render_face(
        &faces[0],
        &RenderOptions {
            text: "سلام\nAmiri".into(),
            size: 32.0,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(
        bm.width > 40 && bm.height > 60,
        "{}x{}",
        bm.width,
        bm.height
    );
    assert!(!bm.is_blank());
    assert_eq!(bm.missing, 0);
    assert!(bm.glyphs >= 8);
    // Ink sits below the first baseline and above the second.
    assert!(bm.baseline > 20.0 && bm.baseline < bm.height as f32);

    let png = encode::png(&bm, [255, 255, 255], None);
    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
    assert_eq!(&png[12..16], b"IHDR");
    assert!(png.ends_with(b"IEND\xaeB`\x82"));
    let opaque = encode::png(&bm, [0, 0, 0], Some([255, 255, 255]));
    assert!(opaque.len() > 100);

    let six = encode::sixel(&bm, [255, 255, 255], [0, 0, 0], 16);
    assert!(six.starts_with("\x1bP0;1;0q\"1;1;"));
    assert!(six.ends_with("\x1b\\"));
    assert!(six.contains('-'), "band separators");

    let blocks = encode::half_blocks(&bm, [255, 255, 255], [0, 0, 0]);
    assert_eq!(blocks.lines().count(), (bm.height as usize).div_ceil(2));
    assert!(blocks.contains('▀'));
    assert!(blocks.contains("\x1b[38;2;"));

    let k = encode::kitty(&png, false);
    assert!(k.starts_with("\x1b_Gf=100,a=T,t=d,q=2,m="));
    assert!(encode::kitty(&png, true).starts_with("\x1bPtmux;\x1b\x1b_G"));
    assert!(encode::iterm(&png, false).starts_with("\x1b]1337;File=inline=1;size="));
    assert_eq!(encode::parse_rgb("#1a2B3c"), Some([0x1a, 0x2b, 0x3c]));
    assert_eq!(encode::parse_rgb("nope"), None);

    // Variable axes and features are honoured.
    let (_, bric) =
        unifont_core::load_file(&fixtures().join("BricolageGrotesque[opsz,wdth,wght].ttf"))
            .unwrap();
    let light = render_face(
        &bric[0],
        &RenderOptions {
            text: "Bold".into(),
            variations: vec![("wght".into(), 200.0)],
            ..Default::default()
        },
    )
    .unwrap();
    let heavy = render_face(
        &bric[0],
        &RenderOptions {
            text: "Bold".into(),
            variations: vec![("wght".into(), 800.0)],
            ..Default::default()
        },
    )
    .unwrap();
    let ink = |b: &unifont_core::render::Bitmap| b.coverage.iter().map(|&c| c as u64).sum::<u64>();
    assert!(
        ink(&heavy) > ink(&light) * 3 / 2,
        "{} vs {}",
        ink(&heavy),
        ink(&light)
    );
    assert!(
        render_face(
            &bric[0],
            &RenderOptions {
                variations: vec![("weight".into(), 1.0)],
                ..Default::default()
            }
        )
        .is_err(),
        "bad tag is an error"
    );
    let woff = unifont_core::load_file(&fixtures().join("inter-latin-400-normal.woff2")).unwrap();
    let w = render_face(&woff.1[0], &RenderOptions::default()).unwrap();
    assert!(!w.is_blank(), "WOFF2 is unwrapped before rendering");
}
