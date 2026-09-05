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

//! "`--json` output validates against `schemas/cli-output.json`", as a test.
//!
//! README.md, the manual and PLAN.md all make that promise, and until this file nothing
//! checked it. CI diffs `schemas/*.json` against what `fontina schema` prints, which
//! proves the files match the Rust types and says nothing about whether any output
//! matches the files. A round-trip through `serde` proves less still: it only shows the
//! type is its own inverse.
//!
//! So: run the real binary against the real fixtures, take the bytes it puts on stdout,
//! and validate them against the schema **files** in `schemas/`. The files are what a
//! person integrating with fontina downloads, so the files are what must be true. A
//! schema regenerated inside the test would only re-prove what CI already diffs.
//!
//! [`every_json_command_is_covered_or_exempt`] walks `fontina --help` and fails when a
//! command grows `--json` without growing a case here, the way `tests/checks.rs` stops
//! the check-id list drifting.
//!
//! Two commands are validated against a different file, because that is what they print:
//! `info` prints `FaceMetadata`, which is `schemas/face.json` and is in no other file,
//! and `collection export` prints a `CollectionExport`, which has its own
//! `schemas/collection.json` as well as a definition in `cli-output.json`. Both types
//! are published, so the substance of the promise holds; the manual's "every type it
//! prints is in `schemas/cli-output.json`" is just loose about which file.

use jsonschema::Validator;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

// ---------------------------------------------------------------------------------
// Running the binary
// ---------------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fixtures() -> PathBuf {
    repo_root().join("fixtures")
}

fn fixture(name: &str) -> String {
    fixtures().join(name).to_string_lossy().into_owned()
}

fn run(db: &Path, args: &[String]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_fontina"))
        .args(["--db", &db.to_string_lossy()])
        .args(args)
        .output()
        .expect("fontina runs")
}

/// Run a command and return its stdout, insisting the exit code is one the command is
/// documented to use. `check` exits 1 when a check errors, and still prints its report.
fn stdout_of(db: &Path, args: &[String]) -> String {
    let out = run(db, args);
    let allowed: &[i32] = if args.first().is_some_and(|a| a == "check") {
        &[0, 1]
    } else {
        &[0]
    };
    assert!(
        out.status.code().is_some_and(|c| allowed.contains(&c)),
        "`fontina {}` exited {:?}: {}",
        args.join(" "),
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("stdout is UTF-8")
}

fn temp_dir(what: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "fontina-schema-{what}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

// ---------------------------------------------------------------------------------
// Validators, built from the files in schemas/
// ---------------------------------------------------------------------------------

fn schema_file(name: &str) -> Value {
    let path = repo_root().join("schemas").join(name);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("parsing {}: {e}", path.display()))
}

/// A validator for one `#/$defs/<name>` of `schemas/cli-output.json`.
///
/// The whole document is kept as the schema root so `$ref`s between definitions still
/// resolve; only a root `$ref` is added to point at the definition under test.
fn cli_output_validator(def: &str) -> Validator {
    let mut doc = schema_file("cli-output.json");
    let obj = doc.as_object_mut().expect("cli-output.json is an object");
    assert!(
        obj["$defs"].get(def).is_some(),
        "schemas/cli-output.json has no definition for {def}"
    );
    obj.insert("$ref".into(), json!(format!("#/$defs/{def}")));
    jsonschema::validator_for(&doc).expect("schemas/cli-output.json compiles")
}

fn whole_file_validator(name: &str) -> Validator {
    jsonschema::validator_for(&schema_file(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// Validate, and on failure say which instance path broke which keyword. A bare
/// "did not validate" sends the reader back to the JSON with no idea where to look.
fn must_validate(label: &str, validator: &Validator, instance: &Value) {
    let errors: Vec<String> = validator
        .iter_errors(instance)
        .map(|e| format!("  at {}: {e}", e.instance_path()))
        .collect();
    assert!(
        errors.is_empty(),
        "{label} does not validate against its schema:\n{}",
        errors.join("\n")
    );
}

// ---------------------------------------------------------------------------------
// The cases
// ---------------------------------------------------------------------------------

/// What a command's stdout is supposed to be.
enum Shape {
    /// One instance of a `schemas/cli-output.json` definition.
    Def(&'static str),
    /// A JSON array of them. Every element is validated, and an empty array is a
    /// perfectly good case: it is the shape a listing takes most often in the wild.
    ArrayOf(&'static str),
    /// A JSON array of `FaceMetadata`, against `schemas/face.json`.
    Faces,
    /// A collection export, against `schemas/collection.json`.
    Collection,
}

struct Case {
    /// The command path exactly as `fontina --help` spells it, so the enumeration test
    /// can match cases to commands.
    command: &'static str,
    args: Vec<String>,
    shape: Shape,
    /// What this case is here to prove.
    why: &'static str,
}

fn case(command: &'static str, args: &[&str], shape: Shape, why: &'static str) -> Case {
    Case {
        command,
        args: args.iter().map(|s| (*s).to_owned()).collect(),
        shape,
        why,
    }
}

/// Commands with `--json` that a hermetic test may not run.
///
/// Every one of them registers a font with the running operating system, copies one into
/// the user's font directory, or writes a login agent into the user's home. CLAUDE.md
/// rules that out, and `tests/json_contract.rs` is where the one deliberate exception
/// lives (a session activation, undone before it returns). `scripts/acceptance` covers
/// these end to end on a throwaway XDG home.
///
/// `activate` and `install` print `ActivationRecord`, which is defined and which
/// `activations` validates here from the same rows. The other five print types the
/// schema does not describe at all — see [`json_output_the_schema_does_not_describe`].
const EXEMPT: &[(&str, &str)] = &[
    ("activate", "registers a font with the running OS"),
    ("deactivate", "unregisters a font from the running OS"),
    ("install", "copies a font into the user's font directory"),
    ("uninstall", "removes a font from the user's font directory"),
    (
        "restore",
        "re-registers every recorded activation with the OS",
    ),
    ("agent install", "writes a login agent into the user's home"),
    (
        "agent uninstall",
        "removes a login agent from the user's home",
    ),
];

/// Commands covered by a test of their own rather than by a [`Case`].
const COVERED_ELSEWHERE: &[(&str, &str)] = &[(
    "watch",
    "never returns on its own; driven by watch_events_validate_line_by_line",
)];

/// Index the fixtures into `work/index.db`, with a tag, two collections and two
/// activation records on top.
///
/// `work` holds everything written during a run: the index, the bundle, the export.
fn build_library(work: &Path) -> PathBuf {
    let db = work.join("index.db");
    let fx = fixtures().to_string_lossy().into_owned();
    let export = work.join("Set.json").to_string_lossy().into_owned();
    let setup: &[&[&str]] = &[
        &["scan", &fx],
        &["tag", "add", "serif", "1"],
        &["tag", "add", "text", "1"],
        &["collection", "create", "Set"],
        &["collection", "add", "Set", "1", "2"],
        // An empty collection is a shape a schema meets on day one and a test rarely
        // reaches: `faces: []`, and no tags on any of them.
        &["collection", "create", "Empty"],
        &["collection", "export", "Set", &export],
    ];
    for args in setup {
        let args: Vec<String> = args.iter().map(|s| (*s).to_owned()).collect();
        let out = run(&db, &args);
        assert!(
            out.status.success(),
            "setup `fontina {}` failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr)
        );
    }

    // Two activation records, written straight to the index.
    //
    // `activate` and `install` are the commands that normally write these, and a
    // hermetic test may not run either: both register a font with the running operating
    // system. What reaches stdout is the index row, not the registration, so writing the
    // row through the core's own API produces exactly the JSON `activations` would print
    // and leaves the system alone. Without it `ActivationRecord`, `Conflict` and
    // `FaceSummary`'s `activation` field are only ever seen as empty arrays and absent
    // keys, which validates nothing.
    //
    // The two Inter fixtures are the same face in two containers, so activating one
    // makes it a conflict for the other.
    let woff = only_id(&db, "woff");
    let woff2 = only_id(&db, "woff2");
    let mut index = fontina_core::Index::open(&db).expect("the index opens");
    index
        .set_activation(&[woff], fontina_core::ActivationState::User, None)
        .expect("recording an activation");
    index
        .set_activation(
            &[woff2],
            fontina_core::ActivationState::Installed,
            Some(&work.join("installed/Inter.woff2").to_string_lossy()),
        )
        .expect("recording an install");
    drop(index);

    // A file that is not a font, under a name that says it is: the only way to see a
    // `ScanFailure`, which is the one part of `ScanReport` a clean library never fills.
    let broken = work.join("broken");
    std::fs::create_dir_all(&broken).unwrap();
    std::fs::write(broken.join("not-a-font.ttf"), b"not a font at all").unwrap();

    db
}

/// The id of the one face in a container, from `fontina list --json`.
fn only_id(db: &Path, container: &str) -> i64 {
    let stdout = stdout_of(
        db,
        &[
            "list".into(),
            "--json".into(),
            "--container".into(),
            container.into(),
        ],
    );
    let faces: Value = serde_json::from_str(&stdout).expect("list prints JSON");
    let faces = faces.as_array().expect("an array");
    assert_eq!(faces.len(), 1, "expected one {container} fixture");
    faces[0]["id"].as_i64().expect("a face id")
}

/// Every case, in the order they are run. Building the list writes nothing but the
/// scratch directories the arguments name, so the enumeration test can call it alone.
fn cases(work: &Path) -> Vec<Case> {
    let fx = fixtures().to_string_lossy().into_owned();
    let empty_dir = work.join("no-fonts");
    std::fs::create_dir_all(&empty_dir).unwrap();
    let empty = empty_dir.to_string_lossy().into_owned();
    let broken = work.join("broken").to_string_lossy().into_owned();
    let bundle = work.join("bundle").to_string_lossy().into_owned();
    let export = work.join("Set.json").to_string_lossy().into_owned();

    // Every fixture, by path, so `info` parses the file rather than reading the index.
    let by_path: Vec<Case> = [
        ("Amiri-Regular.ttf", "an Arabic TrueType face"),
        ("SourceSerif4-Regular.otf", "a CFF outline face"),
        ("BricolageGrotesque[opsz,wdth,wght].ttf", "a variable font"),
        ("Nabla[EDPT,EHLT].ttf", "a colour font, and variable"),
        ("inter-latin-400-normal.woff", "a WOFF1 container"),
        ("inter-latin-400-normal.woff2", "a WOFF2 container"),
    ]
    .iter()
    .map(|(name, why)| Case {
        command: "info",
        args: vec!["info".into(), "--json".into(), fixture(name)],
        shape: Shape::Faces,
        why,
    })
    .collect();

    let mut cases = vec![
        case(
            "scan",
            &["scan", "--json", &fx],
            Shape::Def("ScanReport"),
            "the whole fixture library, rescanned",
        ),
        case(
            "scan",
            &["scan", "--json", &empty],
            Shape::Def("ScanReport"),
            "a directory with no fonts in it: every count zero, `failed` empty",
        ),
        case(
            "scan",
            &["scan", "--json", &broken],
            Shape::Def("ScanReport"),
            "a file named .ttf that is not one, so `failed` holds a ScanFailure",
        ),
        case(
            "list",
            &["list", "--json"],
            Shape::ArrayOf("FaceSummary"),
            "every face in the library",
        ),
        case(
            "list",
            &["list", "--json", "-n", "1"],
            Shape::ArrayOf("FaceSummary"),
            "a single result",
        ),
        case(
            "list",
            &["list", "--json", "--family", "No Such Family"],
            Shape::ArrayOf("FaceSummary"),
            "an empty result",
        ),
        case(
            "list",
            &["list", "--json", "--variable"],
            Shape::ArrayOf("FaceSummary"),
            "the variable fonts, whose axis fields are populated",
        ),
        case(
            "list",
            &["list", "--json", "--color"],
            Shape::ArrayOf("FaceSummary"),
            "the colour font",
        ),
        case(
            "list",
            &["list", "--json", "--container", "woff"],
            Shape::ArrayOf("FaceSummary"),
            "a WOFF container",
        ),
        case(
            "list",
            &["list", "--json", "--tag", "serif"],
            Shape::ArrayOf("FaceSummary"),
            "a tagged face, so `tags` is not empty",
        ),
        case(
            "list",
            &["list", "--json", "--active"],
            Shape::ArrayOf("FaceSummary"),
            "the activated faces, so `activation` is not absent",
        ),
        case(
            "families",
            &["families", "--json"],
            Shape::ArrayOf("Family"),
            "every family",
        ),
        case(
            "families",
            &["families", "--json", "--family", "No Such Family"],
            Shape::ArrayOf("Family"),
            "an empty result",
        ),
        case(
            "facets",
            &["facets", "--json"],
            Shape::Def("Facets"),
            "every facet populated",
        ),
        case(
            "facets",
            &["facets", "--json", "--family", "No Such Family"],
            Shape::Def("Facets"),
            "every facet empty",
        ),
        case(
            "tag list",
            &["tag", "list", "--json"],
            Shape::ArrayOf("TagInfo"),
            "two tags",
        ),
        case(
            "tag sync",
            &["tag", "sync", "--to-files", "--dry-run", "--json"],
            Shape::Def("TagSyncReport"),
            "a dry run, which writes nothing to any file",
        ),
        case(
            "collection list",
            &["collection", "list", "--json"],
            Shape::ArrayOf("CollectionInfo"),
            "one collection with faces and one without",
        ),
        case(
            "collection show",
            &["collection", "show", "Set", "--json"],
            Shape::ArrayOf("FaceSummary"),
            "a collection with faces in it",
        ),
        case(
            "collection show",
            &["collection", "show", "Empty", "--json"],
            Shape::ArrayOf("FaceSummary"),
            "an empty collection",
        ),
        case(
            "collection export",
            &["collection", "export", "Set"],
            Shape::Collection,
            "an export on stdout: schemas/collection.json",
        ),
        case(
            "collection export",
            &["collection", "export", "Empty"],
            Shape::Collection,
            "an export with no faces at all",
        ),
        case(
            "collection export",
            &["collection", "export", "Set", "--bundle", &bundle, "--json"],
            Shape::Def("BundleReport"),
            "`--bundle` reports what it wrote instead of printing the export",
        ),
        case(
            "collection import",
            &[
                "collection",
                "import",
                &export,
                "--name",
                "Imported",
                "--json",
            ],
            Shape::Def("ImportReport"),
            "reading back what `collection export` wrote",
        ),
        case(
            "source list",
            &["source", "list", "--json"],
            Shape::ArrayOf("Source"),
            "the directory the library was scanned from",
        ),
        case(
            "source add",
            &["source", "add", &empty, "--json"],
            Shape::Def("Source"),
            "one source, registered and scanned",
        ),
        case(
            "conflicts",
            &[
                "conflicts",
                "--json",
                &fixture("inter-latin-400-normal.woff2"),
            ],
            Shape::ArrayOf("Conflict"),
            "the same face in another container is active",
        ),
        case(
            "conflicts",
            &["conflicts", "--json", &fixture("Amiri-Regular.ttf")],
            Shape::ArrayOf("Conflict"),
            "nothing shares its name: an empty result",
        ),
        case(
            "activations",
            &["activations", "--json"],
            Shape::ArrayOf("ActivationRecord"),
            "one activated face and one installed one, which carries `installed_path`",
        ),
        case(
            "info",
            &["info", "--json", "1"],
            Shape::Faces,
            "a face by index id",
        ),
        case(
            "dupes",
            &["dupes", "--json"],
            Shape::ArrayOf("DuplicateGroup"),
            "the same Inter face in two containers",
        ),
        case(
            "variants",
            &["variants", "1", "--min", "0.0", "--json"],
            Shape::ArrayOf("Related"),
            "every other face, ranked by coverage overlap",
        ),
        case(
            "variants",
            &["variants", "1", "--min", "1.0", "--json"],
            Shape::ArrayOf("Related"),
            "nothing overlaps perfectly: an empty result",
        ),
        case(
            "stats",
            &["stats", "--json"],
            Shape::Def("Stats"),
            "the index",
        ),
        case(
            "check",
            &[
                "check",
                "--json",
                &fixture("Amiri-Regular.ttf"),
                &fixture("SourceSerif4-Regular.otf"),
                &fixture("BricolageGrotesque[opsz,wdth,wght].ttf"),
                &fixture("Nabla[EDPT,EHLT].ttf"),
                &fixture("inter-latin-400-normal.woff"),
                &fixture("inter-latin-400-normal.woff2"),
            ],
            Shape::ArrayOf("CheckReport"),
            "every fixture, health-checked",
        ),
        case(
            "covers",
            &["covers", "--json", "abc"],
            Shape::ArrayOf("FaceSummary"),
            "faces covering Latin text",
        ),
        case(
            "covers",
            &["covers", "--json", "\u{13000}\u{13001}"],
            Shape::ArrayOf("FaceSummary"),
            "no fixture has Egyptian hieroglyphs: an empty result",
        ),
        case(
            "glyphs",
            &["glyphs", "--json", "1"],
            Shape::ArrayOf("BlockCoverage"),
            "coverage by Unicode block",
        ),
        case(
            "glyphs",
            &[
                "glyphs",
                "--json",
                &fixture("Nabla[EDPT,EHLT].ttf"),
                "--block",
                "Latin",
            ],
            Shape::ArrayOf("BlockCoverage"),
            "one named block of the colour font",
        ),
        case(
            "agent status",
            &["agent", "status", "--json"],
            Shape::Def("AgentStatus"),
            "whether a login agent is installed. Read-only: it writes nothing",
        ),
        case(
            "dirs",
            &["dirs", "--json"],
            Shape::ArrayOf("FontDir"),
            "the operating system's font directories",
        ),
        case(
            "license",
            &["license", "--json", "1"],
            Shape::ArrayOf("LicenseRow"),
            "one face's licence and embedding report",
        ),
    ];
    cases.extend(by_path);
    cases
}

/// Definitions [`library`] names that `schemas/cli-output.json` does not have.
///
/// These are the mismatches, kept out of the conforming run and asserted on their own in
/// [`json_output_the_schema_does_not_describe`] so the gap is documented rather than
/// hidden. Fixing it is not a test's call.
const UNDEFINED: &[&str] = &["AgentStatus", "FontDir", "LicenseRow"];

fn is_undefined(shape: &Shape) -> bool {
    let name = match shape {
        Shape::Def(n) | Shape::ArrayOf(n) => *n,
        Shape::Faces | Shape::Collection => return false,
    };
    UNDEFINED.contains(&name)
}

// ---------------------------------------------------------------------------------
// The tests
// ---------------------------------------------------------------------------------

/// Every `--json` command that has a definition, validated against the schema files.
#[test]
fn json_output_validates_against_the_published_schemas() {
    let work = temp_dir("conform");
    let db = build_library(&work);
    let cases = cases(&work);

    let face = whole_file_validator("face.json");
    let collection = whole_file_validator("collection.json");
    // How many instances of each definition were actually validated. An empty array
    // satisfies any schema, so a run of empty results would pass while proving nothing.
    let mut seen: std::collections::BTreeMap<&str, usize> = Default::default();

    for c in &cases {
        if is_undefined(&c.shape) {
            continue;
        }
        let label = format!("`fontina {}` ({})", c.args.join(" "), c.why);
        let stdout = stdout_of(&db, &c.args);
        match c.shape {
            Shape::Def(def) => {
                let value: Value = serde_json::from_str(&stdout)
                    .unwrap_or_else(|e| panic!("{label} is not JSON: {e}\n{stdout}"));
                must_validate(&label, &cli_output_validator(def), &value);
                *seen.entry(def).or_default() += 1;
            }
            Shape::ArrayOf(def) => {
                let value: Value = serde_json::from_str(&stdout)
                    .unwrap_or_else(|e| panic!("{label} is not JSON: {e}\n{stdout}"));
                let items = value
                    .as_array()
                    .unwrap_or_else(|| panic!("{label} is not a JSON array:\n{stdout}"));
                let validator = cli_output_validator(def);
                for (i, item) in items.iter().enumerate() {
                    must_validate(&format!("{label}, element {i}"), &validator, item);
                }
                *seen.entry(def).or_default() += items.len();
            }
            Shape::Faces => {
                let value: Value = serde_json::from_str(&stdout)
                    .unwrap_or_else(|e| panic!("{label} is not JSON: {e}\n{stdout}"));
                let items = value
                    .as_array()
                    .unwrap_or_else(|| panic!("{label} is not a JSON array:\n{stdout}"));
                assert!(!items.is_empty(), "{label} found no faces");
                for (i, item) in items.iter().enumerate() {
                    must_validate(&format!("{label}, face {i}"), &face, item);
                }
            }
            Shape::Collection => {
                let value: Value = serde_json::from_str(&stdout)
                    .unwrap_or_else(|e| panic!("{label} is not JSON: {e}\n{stdout}"));
                must_validate(&label, &collection, &value);
            }
        }
    }

    for def in cases
        .iter()
        .filter(|c| !is_undefined(&c.shape))
        .filter_map(|c| match c.shape {
            Shape::Def(n) | Shape::ArrayOf(n) => Some(n),
            Shape::Faces | Shape::Collection => None,
        })
    {
        assert!(
            seen.get(def).copied().unwrap_or(0) > 0,
            "every case naming {def} came back empty, so nothing was validated against \
             it. Give one of them something to find."
        );
    }

    // `ScanFailure` and `TagSyncChange` never appear on their own, so the count above
    // cannot see them; both are arranged for by `build_library` and asserted here.
    let broken = work.join("broken").to_string_lossy().into_owned();
    let report: Value =
        serde_json::from_str(&stdout_of(&db, &["scan".into(), "--json".into(), broken]))
            .expect("scan prints JSON");
    assert!(
        !report["failed"]
            .as_array()
            .expect("failed is an array")
            .is_empty(),
        "the file that is not a font parsed anyway, so no ScanFailure was validated"
    );
    let sync: Value = serde_json::from_str(&stdout_of(
        &db,
        &[
            "tag".into(),
            "sync".into(),
            "--to-files".into(),
            "--dry-run".into(),
            "--json".into(),
        ],
    ))
    .expect("tag sync prints JSON");
    assert!(
        !sync["changes"]
            .as_array()
            .expect("changes is an array")
            .is_empty(),
        "the tagged face had nothing to sync, so no TagSyncChange was validated"
    );

    let _ = std::fs::remove_dir_all(&work);
}

/// The validators reject what they should.
///
/// Every other test here passes when a validator is vacuous — a `$ref` that resolved to
/// nothing, a schema that compiled to "anything goes" — and would go on passing for
/// years. This is the control: four instances that must be refused, across all three
/// schema files.
#[test]
fn the_validators_reject_what_the_schemas_forbid() {
    assert!(
        !cli_output_validator("FaceSummary").is_valid(&json!({})),
        "an empty object satisfied FaceSummary: the root $ref into \
         schemas/cli-output.json is not being applied"
    );
    assert!(
        !cli_output_validator("Stats").is_valid(&json!({"files": "six"})),
        "a string satisfied an integer property of Stats"
    );
    assert!(
        !whole_file_validator("face.json").is_valid(&json!({"schema_version": 1})),
        "an object with only a schema_version satisfied schemas/face.json"
    );
    assert!(
        !whole_file_validator("collection.json").is_valid(&json!({"name": "Set"})),
        "an object with only a name satisfied schemas/collection.json"
    );
}

/// A `FaceMetadata` reduced to exactly the properties `schemas/face.json` calls
/// required still validates against it.
///
/// `FaceMetadata` skips `os2` and `variable` when they are `None`, so a face parsed from
/// a font with no `OS/2` table prints neither. No fixture is that impoverished — every
/// one of them has an `OS/2` table — so the shape is built by stripping a real face down
/// to its required properties. If the schema marks something required that the type
/// omits, this is where it shows.
#[test]
fn a_face_with_no_optional_fields_still_validates() {
    let work = temp_dir("minimal");
    let db = work.join("index.db");
    let face = whole_file_validator("face.json");
    let schema = schema_file("face.json");
    let required: BTreeSet<&str> = schema["required"]
        .as_array()
        .expect("face.json lists required properties")
        .iter()
        .map(|v| v.as_str().expect("a property name"))
        .collect();

    let stdout = stdout_of(
        &db,
        &[
            "info".into(),
            "--json".into(),
            fixture("BricolageGrotesque[opsz,wdth,wght].ttf"),
        ],
    );
    let faces: Value = serde_json::from_str(&stdout).expect("info prints JSON");
    let full = faces[0].clone();

    // All of them: the variable font carries `os2` and `variable` both.
    assert!(full.get("os2").is_some() && full.get("variable").is_some());
    must_validate("a face with every optional field set", &face, &full);

    let stripped: Value = Value::Object(
        full.as_object()
            .expect("a face is an object")
            .iter()
            .filter(|(k, _)| required.contains(k.as_str()))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
    );
    assert!(
        stripped.get("os2").is_none() && stripped.get("variable").is_none(),
        "face.json calls the optional fields required, so nothing was stripped"
    );
    must_validate("a face with no optional field set", &face, &stripped);
    let _ = std::fs::remove_dir_all(&work);
}

/// `watch --json` prints one `WatchEvent` per line, and the line validates.
///
/// The only `--json` command that never returns on its own, so it is driven rather than
/// run: start it on an empty directory, drop a font in, read the first line, stop it.
#[test]
fn watch_events_validate_line_by_line() {
    use std::io::BufRead;
    use std::process::Stdio;

    let work = temp_dir("watch");
    let db = work.join("index.db");
    let watched = work.join("watched");
    std::fs::create_dir_all(&watched).unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_fontina"))
        .args(["--db", &db.to_string_lossy()])
        .args(["watch", "--json", "--debounce-ms", "50"])
        .arg(&watched)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("fontina watch starts");
    let stdout = child.stdout.take().expect("piped stdout");

    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut line = String::new();
        if std::io::BufReader::new(stdout).read_line(&mut line).is_ok() {
            let _ = tx.send(line);
        }
    });

    // The watcher needs to be listening before the file lands, and no API says when it
    // is; copying repeatedly costs nothing and removes the race.
    let src = fixtures().join("SourceSerif4-Regular.otf");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    let line = loop {
        std::fs::copy(&src, watched.join("SourceSerif4-Regular.otf")).unwrap();
        match rx.recv_timeout(std::time::Duration::from_millis(500)) {
            Ok(line) => break Some(line),
            Err(_) if std::time::Instant::now() < deadline => {
                let _ = std::fs::remove_file(watched.join("SourceSerif4-Regular.otf"));
            }
            Err(_) => break None,
        }
    };
    let _ = child.kill();
    let _ = child.wait();

    let line = line.expect("`fontina watch --json` printed an event within 30s");
    let value: Value = serde_json::from_str(line.trim())
        .unwrap_or_else(|e| panic!("`fontina watch --json` line is not JSON: {e}\n{line}"));
    must_validate("a watch event", &cli_output_validator("WatchEvent"), &value);
    let _ = std::fs::remove_dir_all(&work);
}

/// Every command that takes `--json` has a case here, is exempt, or the test fails.
///
/// Enumerated from `fontina --help`, not from memory: a new `--json` on a new command is
/// exactly the change that would otherwise quietly leave the promise unchecked again.
#[test]
fn every_json_command_is_covered_or_exempt() {
    let commands = commands_taking_json();
    assert!(
        commands.len() > 20,
        "walking `fontina --help` found only {} commands with `--json`; the help format \
         has probably changed and this test is no longer reading it",
        commands.len()
    );

    let work = temp_dir("enumerate");
    let cases = cases(&work);
    let _ = std::fs::remove_dir_all(&work);

    let covered: BTreeSet<&str> = cases
        .iter()
        .map(|c| c.command)
        .chain(COVERED_ELSEWHERE.iter().map(|(c, _)| *c))
        .collect();
    let exempt: BTreeSet<&str> = EXEMPT.iter().map(|(c, _)| *c).collect();

    let uncovered: Vec<&String> = commands
        .iter()
        .filter(|c| !covered.contains(c.as_str()) && !exempt.contains(c.as_str()))
        .collect();
    assert!(
        uncovered.is_empty(),
        "these commands take `--json` and nothing validates their output: {uncovered:?}\n\
         Add a case to `cases()` in this file, or, if a hermetic test cannot run the \
         command, add it to `EXEMPT` with the reason."
    );

    let stale: Vec<&&str> = covered
        .iter()
        .chain(exempt.iter())
        .filter(|c| !commands.contains(**c))
        .collect();
    assert!(
        stale.is_empty(),
        "these are listed here but no longer take `--json`: {stale:?}"
    );
}

/// Three `--json` commands print a type `schemas/cli-output.json` does not define.
///
/// `dirs` prints `fontina_platform::FontDir`, `license` prints the CLI's own
/// `LicenseRow`, and `agent status` prints an object literal with no Rust type at all.
/// None of the three is in `cli_output_schema()`, so none is in the file, so the promise
/// README.md, the manual and PLAN.md all make — "`--json` output validates against
/// `schemas/cli-output.json`" — is not true of them. CLAUDE.md says the same in the
/// imperative: "Every type printed with `--json` derives `JsonSchema` and is listed in
/// `cli_output_schema()`".
///
/// Four more are in the same position and cannot be run here (see [`EXEMPT`]):
/// `deactivate` and `uninstall` print `Vec<PathBuf>`, `restore` prints the CLI's
/// `RestoreReport`, and `agent install` and `agent uninstall` print object literals.
///
/// Whether the fix is to define the types or to stop promising is a decision for a
/// person; this test only refuses to let the gap go unrecorded. When a definition
/// arrives, this fails and says to move the command into the conforming run.
#[test]
fn json_output_the_schema_does_not_describe() {
    let defs = schema_file("cli-output.json");
    let defs = defs["$defs"].as_object().expect("$defs is an object");

    let work = temp_dir("undefined");
    let db = build_library(&work);
    let cases = cases(&work);

    for name in UNDEFINED {
        assert!(
            !defs.contains_key(*name),
            "schemas/cli-output.json now defines {name}: remove it from UNDEFINED so \
             its command is validated with the rest"
        );
    }

    // Missing by name is the mismatch. What follows is the reason a validator alone
    // would not have caught it: `schemars` leaves `additionalProperties` unset, so every
    // definition in the file accepts extra properties, and an undefined shape can be
    // waved through by an unrelated definition that happens to require a subset of its
    // keys. A `LicenseRow` has a `path` and a `reason`, which is all `TagSyncSkip` asks
    // for. Pinning the coincidences here means a schema change that creates or removes
    // one is noticed rather than absorbed.
    let mut accidental: Vec<(String, Vec<String>)> = Vec::new();
    for c in cases.iter().filter(|c| is_undefined(&c.shape)) {
        let stdout = stdout_of(&db, &c.args);
        let value: Value = serde_json::from_str(&stdout).expect("JSON on stdout");
        let sample = match &value {
            Value::Array(items) => match items.first() {
                Some(first) => first.clone(),
                // An empty array is accepted by anything and proves nothing either way.
                None => continue,
            },
            other => other.clone(),
        };
        accidental.push((
            c.args.join(" "),
            defs.keys()
                .filter(|def| cli_output_validator(def).is_valid(&sample))
                .cloned()
                .collect(),
        ));
    }
    let found: Vec<(&str, Vec<&str>)> = accidental
        .iter()
        .map(|(k, v)| (k.as_str(), v.iter().map(String::as_str).collect()))
        .collect();
    let expected: Vec<(&str, Vec<&str>)> = ACCIDENTAL_ACCEPTANCES
        .iter()
        .map(|(command, defs)| (*command, defs.to_vec()))
        .collect();
    assert_eq!(
        found, expected,
        "which definitions happen to accept an output they do not describe has changed"
    );
    let _ = std::fs::remove_dir_all(&work);
}

/// Definitions that accept one of the [`UNDEFINED`] outputs by coincidence, in the order
/// [`cases`] lists the commands. See [`json_output_the_schema_does_not_describe`].
const ACCIDENTAL_ACCEPTANCES: &[(&str, &[&str])] = &[
    ("agent status --json", &[]),
    ("dirs --json", &[]),
    ("license --json 1", &["TagSyncSkip"]),
];

// ---------------------------------------------------------------------------------
// Walking `fontina --help`
// ---------------------------------------------------------------------------------

/// Every command path whose own `Options:` section lists `--json`.
fn commands_taking_json() -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    walk(&mut Vec::new(), &mut found);
    found
}

fn walk(path: &mut Vec<String>, found: &mut BTreeSet<String>) {
    let help = help_for(path);
    if section(&help, "Options:")
        .iter()
        .any(|l| l.split_whitespace().next() == Some("--json"))
        && !path.is_empty()
    {
        found.insert(path.join(" "));
    }
    for name in section(&help, "Commands:")
        .iter()
        .filter_map(|l| l.strip_prefix("  "))
        .filter(|l| !l.starts_with(' '))
        .filter_map(|l| l.split_whitespace().next())
        .filter(|n| *n != "help")
        .map(str::to_owned)
        .collect::<Vec<_>>()
    {
        path.push(name);
        walk(path, found);
        path.pop();
    }
}

fn help_for(path: &[String]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_fontina"))
        .args(path)
        .arg("--help")
        .output()
        .expect("fontina runs");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// The body of one `clap` help section: every line up to the next unindented heading.
///
/// Blank lines are kept rather than used as the terminator, because `clap` puts one
/// between options whenever a command has long help (`fontina tag sync --help` does),
/// and stopping at the first would hide every option after it.
fn section<'a>(help: &'a str, heading: &str) -> Vec<&'a str> {
    help.lines()
        .skip_while(|l| l.trim_end() != heading)
        .skip(1)
        .take_while(|l| l.trim().is_empty() || l.starts_with(' '))
        .collect()
}
