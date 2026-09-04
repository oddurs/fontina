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

//! Tags, collections, sources, activation state, facets and families against the
//! fixture fonts in an in-memory index.

use fontina_core::{
    ActivationState, CollectionExport, FaceFilter, Freedom, Index, ScanOptions, SourceKind,
};
use std::path::PathBuf;

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

fn indexed() -> Index {
    let mut index = Index::open_in_memory().unwrap();
    let report =
        fontina_core::scan::scan(&mut index, &[fixtures()], &ScanOptions::default()).unwrap();
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
    assert_eq!(export.schema_version, fontina_core::SCHEMA_VERSION);
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
    moved.faces.push(fontina_core::CollectionFace {
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
        schema_version: fontina_core::SCHEMA_VERSION + 1,
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
            Some("/home/u/.local/share/fonts/fontina/Nabla.ttf"),
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
    let report = fontina_core::scan::scan(
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
fn a_parse_failure_keeps_the_user_s_curation() {
    // A font rewritten in place, a truncated download, a file caught mid-copy by the
    // watcher: the parse fails, and the tags, collections and activation the user built
    // by hand must still be there when it parses again.
    let dir = std::env::temp_dir().join(format!("fontina-fail-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // scan canonicalises its roots, and on macOS the temp dir is a symlink.
    let dir = std::fs::canonicalize(&dir).unwrap();
    let font = dir.join("Amiri-Regular.ttf");
    std::fs::copy(fixtures().join("Amiri-Regular.ttf"), &font).unwrap();
    let mut index = Index::open_in_memory().unwrap();
    fontina_core::scan::scan(
        &mut index,
        std::slice::from_ref(&dir),
        &ScanOptions::default(),
    )
    .unwrap();
    let id = id_of(&index, "Amiri");
    index.tag(&[id], "editorial").unwrap();
    index.add_to_collection("Books", &[id]).unwrap();
    index
        .set_activation(&[id], ActivationState::Session, None)
        .unwrap();

    // The file is replaced by something unparseable and rescanned.
    std::fs::write(&font, b"not a font at all").unwrap();
    let report = fontina_core::scan::scan(
        &mut index,
        std::slice::from_ref(&dir),
        &ScanOptions::default(),
    )
    .unwrap();
    assert_eq!(report.failed.len(), 1, "{report:?}");
    let stats = index.stats().unwrap();
    assert_eq!(stats.failed_files, 1);
    let s = index.summaries(&[id]).unwrap().remove(0);
    assert_eq!(s.tags, ["editorial"]);
    assert_eq!(s.activation, Some(ActivationState::Session));
    assert_eq!(index.collection_faces("Books").unwrap()[0].id, id);
    assert_eq!(index.activations().unwrap().len(), 1);

    // And when it parses again, the curation is still attached to the new rows.
    std::fs::copy(fixtures().join("Amiri-Regular.ttf"), &font).unwrap();
    fontina_core::scan::scan(
        &mut index,
        std::slice::from_ref(&dir),
        &ScanOptions::default(),
    )
    .unwrap();
    assert_eq!(index.stats().unwrap().failed_files, 0);
    let id2 = id_of(&index, "Amiri");
    let s = index.summaries(&[id2]).unwrap().remove(0);
    assert_eq!(s.tags, ["editorial"]);
    assert_eq!(s.activation, Some(ActivationState::Session));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn pruning_only_forgets_files_that_are_really_gone() {
    let dir = std::env::temp_dir().join(format!("fontina-prune-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // scan canonicalises its roots, and on macOS the temp dir is a symlink.
    let dir = std::fs::canonicalize(&dir).unwrap();
    for name in ["Amiri-Regular.ttf", "SourceSerif4-Regular.otf"] {
        std::fs::copy(fixtures().join(name), dir.join(name)).unwrap();
    }
    let mut index = Index::open_in_memory().unwrap();
    fontina_core::scan::scan(
        &mut index,
        std::slice::from_ref(&dir),
        &ScanOptions::default(),
    )
    .unwrap();
    assert_eq!(index.stats().unwrap().faces, 2);

    // One file really deleted, with a sibling left: pruned.
    std::fs::remove_file(dir.join("Amiri-Regular.ttf")).unwrap();
    assert_eq!(index.prune_missing(&dir.to_string_lossy()).unwrap(), 1);
    assert_eq!(index.stats().unwrap().faces, 1);

    // A root we cannot read is not a root whose files are gone.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&dir).unwrap().permissions();
        perms.set_mode(0o000);
        std::fs::set_permissions(&dir, perms).unwrap();
        let pruned = index.prune_missing(&dir.to_string_lossy()).unwrap();
        let mut perms = std::fs::metadata(&dir).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&dir, perms).unwrap();
        assert_eq!(pruned, 0, "an unreadable root must prune nothing");
        assert_eq!(index.stats().unwrap().faces, 1);
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_unavailable_directory_is_not_an_empty_one() {
    // An unmounted share leaves the mount point behind as an empty directory, which looks
    // exactly like a directory whose fonts were deleted. Pruning every last file under a
    // root is refused; `remove_under` is how you say you meant it.
    let dir = std::env::temp_dir().join(format!("fontina-unmount-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // scan canonicalises its roots, and on macOS the temp dir is a symlink.
    let dir = std::fs::canonicalize(&dir).unwrap();
    for name in ["Amiri-Regular.ttf", "SourceSerif4-Regular.otf"] {
        std::fs::copy(fixtures().join(name), dir.join(name)).unwrap();
    }
    let mut index = Index::open_in_memory().unwrap();
    fontina_core::scan::scan(
        &mut index,
        std::slice::from_ref(&dir),
        &ScanOptions::default(),
    )
    .unwrap();
    let id = id_of(&index, "Amiri");
    index.tag(&[id], "kept").unwrap();

    for name in ["Amiri-Regular.ttf", "SourceSerif4-Regular.otf"] {
        std::fs::remove_file(dir.join(name)).unwrap();
    }
    assert_eq!(index.prune_missing(&dir.to_string_lossy()).unwrap(), 0);
    assert_eq!(index.stats().unwrap().faces, 2, "nothing was forgotten");
    assert_eq!(index.remove_under(&dir.to_string_lossy()).unwrap(), 2);
    assert_eq!(index.stats().unwrap().faces, 0);
    let _ = std::fs::remove_dir_all(&dir);
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

/// Every fixture is OFL, so the whole index is free and the other three states are
/// empty. The filter and the facet must agree with `freedom::classify` on each face.
#[test]
fn freedom_filters_and_counts_agree() {
    let index = indexed();
    let all = index.list(&FaceFilter::default()).unwrap();
    assert_eq!(all.len(), 6);
    assert!(all.iter().all(|f| f.freedom == Freedom::Free), "{all:?}");

    let free = index
        .list(&FaceFilter {
            freedom: Some(Freedom::Free),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(free.len(), 6);
    for state in [Freedom::Nonfree, Freedom::Unknown, Freedom::Unstated] {
        let rows = index
            .list(&FaceFilter {
                freedom: Some(state),
                ..Default::default()
            })
            .unwrap();
        assert!(rows.is_empty(), "{state} matched {} face(s)", rows.len());
    }

    let facets = index.facets(&FaceFilter::default()).unwrap();
    assert_eq!(facets.freedom.len(), 1);
    assert_eq!(facets.freedom[0].value, "free");
    assert_eq!(facets.freedom[0].count, 6);
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
    let get = |v: &[fontina_core::index::FacetCount], k: &str| {
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
    assert_eq!(fontina_core::index::weight_name(700), "Bold");
    assert_eq!(fontina_core::index::weight_bucket(651.0), 700);
    assert_eq!(fontina_core::index::width_bucket(80.0), 75.0);
    assert_eq!(fontina_core::index::width_name(75.0), "Condensed");
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
    let coll = fontina_core::collection_schema();
    assert_eq!(coll["title"], "CollectionExport");
    let cli = fontina_core::cli_output_schema();
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
    let dir = std::env::temp_dir().join(format!("fontina-watch-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("sub")).unwrap();
    let mut index = Index::open_in_memory().unwrap();
    fontina_core::scan::scan(
        &mut index,
        std::slice::from_ref(&dir),
        &ScanOptions::default(),
    )
    .unwrap();
    let roots = vec![std::fs::canonicalize(&dir).unwrap()];
    let opts = fontina_core::watch::WatchOptions::default();

    // A new file is parsed on its own.
    let amiri = roots[0].join("Amiri-Regular.ttf");
    std::fs::copy(fixtures().join("Amiri-Regular.ttf"), &amiri).unwrap();
    let ev = fontina_core::watch::apply(&mut index, &roots, &opts, BTreeSet::from([amiri.clone()]))
        .unwrap();
    assert_eq!(ev.report.parsed, 1);
    assert_eq!(ev.paths, [amiri.to_string_lossy().into_owned()]);
    assert_eq!(index.stats().unwrap().faces, 1);

    // Non-font and unchanged paths are no-ops.
    let readme = roots[0].join("README.txt");
    std::fs::write(&readme, "hi").unwrap();
    let ev = fontina_core::watch::apply(
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
    let ev = fontina_core::watch::apply(
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
    let ev = fontina_core::watch::apply(&mut index, &roots, &opts, BTreeSet::from([sub])).unwrap();
    assert_eq!(ev.report.removed, 1, "{ev:?}");
    assert_eq!(index.stats().unwrap().faces, 0);

    // The live watcher delivers a batch for a copied file.
    let (tx, rx) = std::sync::mpsc::channel();
    let root_for_thread = roots[0].clone();
    let handle = std::thread::spawn(move || {
        let mut index = Index::open_in_memory().unwrap();
        fontina_core::watch::watch(
            &mut index,
            &[root_for_thread],
            &fontina_core::watch::WatchOptions {
                debounce: std::time::Duration::from_millis(200),
                ..Default::default()
            },
            |ev| {
                // FSEvents on macOS can replay events from just before the stream
                // started, so earlier batches may carry nothing new; keep going until
                // the copied file has been parsed.
                let parsed = ev.report.parsed;
                tx.send(parsed).unwrap();
                parsed == 0
            },
        )
        .unwrap();
    });
    // The watcher runs on its own thread and nothing says when its stream is registered,
    // so copy the file and, if no batch arrives within a couple of seconds, copy it
    // again: a slow start on a loaded runner then cannot lose the only event, and a
    // batch that parsed nothing (an FSEvents replay, or a file caught mid-copy) is just
    // waited past.
    let nabla = roots[0].join("Nabla.ttf");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    loop {
        std::fs::copy(fixtures().join("Nabla[EDPT,EHLT].ttf"), &nabla).unwrap();
        let wait = std::time::Duration::from_secs(2)
            .min(deadline.saturating_duration_since(std::time::Instant::now()));
        match rx.recv_timeout(wait) {
            Ok(parsed) if parsed >= 1 => break,
            Ok(_) => continue,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout)
                if std::time::Instant::now() < deadline =>
            {
                continue;
            }
            Err(e) => panic!("watcher never reported the new file: {e}"),
        }
    }
    handle.join().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn watch_removes_a_vanished_directory_whose_name_has_a_dot() {
    use std::collections::BTreeSet;
    let dir = std::env::temp_dir().join(format!("fontina-watch-dot-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    // A version in the directory's name gives it a file extension as far as
    // `Path::extension` is concerned. It is still a directory.
    std::fs::create_dir_all(dir.join("Inter v4.0")).unwrap();
    std::fs::copy(
        fixtures().join("Amiri-Regular.ttf"),
        dir.join("Inter v4.0").join("Amiri-Regular.ttf"),
    )
    .unwrap();
    let mut index = Index::open_in_memory().unwrap();
    fontina_core::scan::scan(
        &mut index,
        std::slice::from_ref(&dir),
        &ScanOptions::default(),
    )
    .unwrap();
    assert_eq!(index.stats().unwrap().faces, 1);

    let roots = vec![std::fs::canonicalize(&dir).unwrap()];
    let versioned = roots[0].join("Inter v4.0");
    let opts = fontina_core::watch::WatchOptions::default();
    std::fs::remove_dir_all(&versioned).unwrap();
    let ev = fontina_core::watch::apply(
        &mut index,
        &roots,
        &opts,
        BTreeSet::from([versioned.clone()]),
    )
    .unwrap();
    assert_eq!(ev.report.removed, 1, "{ev:?}");
    assert_eq!(index.stats().unwrap().faces, 0, "{ev:?}");
    assert!(
        ev.paths.contains(&versioned.to_string_lossy().into_owned()),
        "{ev:?}"
    );

    // A vanished plain file the index never knew about is still a no-op.
    let ev = fontina_core::watch::apply(
        &mut index,
        &roots,
        &opts,
        BTreeSet::from([roots[0].join("README.txt")]),
    )
    .unwrap();
    assert!(ev.paths.is_empty(), "{ev:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn render_shapes_and_rasterises() {
    use fontina_core::render::{RenderOptions, encode, render_face, shaped_glyphs};
    let (_, faces) = fontina_core::load_file(&fixtures().join("Amiri-Regular.ttf")).unwrap();
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
        fontina_core::load_file(&fixtures().join("BricolageGrotesque[opsz,wdth,wght].ttf"))
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
    let ink = |b: &fontina_core::render::Bitmap| b.coverage.iter().map(|&c| c as u64).sum::<u64>();
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
    let woff = fontina_core::load_file(&fixtures().join("inter-latin-400-normal.woff2")).unwrap();
    let w = render_face(&woff.1[0], &RenderOptions::default()).unwrap();
    assert!(!w.is_blank(), "WOFF2 is unwrapped before rendering");
}
