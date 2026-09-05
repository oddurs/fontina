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

//! File tags against the real store on the running system.
//!
//! There is no way to test this against a fake: the whole claim is that the tag lands
//! where the desktop already looks, which means the desktop's own attribute on a real
//! file. So these write to a temp file and read it back through the same path a file
//! manager would.

use fontina_platform::{PlatformError, tags};
use std::path::PathBuf;

fn scratch(what: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fontina-tags-{what}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A tag with a comma in it splits in two on GNU/Linux, and one with a newline becomes a
/// colour on macOS. Both are refused everywhere, so a tag set means the same thing on
/// every machine it is carried to.
#[test]
fn a_tag_that_cannot_survive_the_trip_is_refused_on_every_platform() {
    assert!(tags::unstorable("serif").is_none());
    assert!(tags::unstorable("two words").is_none());
    assert!(tags::unstorable("Ελληνικά").is_none());
    assert!(tags::unstorable("serif,sans").is_some());
    assert!(tags::unstorable("serif\nsans").is_some());
    assert!(tags::unstorable("").is_some());
    assert!(tags::unstorable("   ").is_some());
}

#[cfg(windows)]
#[test]
fn windows_says_it_cannot_rather_than_pretending() {
    let dir = scratch("unsupported");
    let file = dir.join("f.ttf");
    std::fs::write(&file, b"not really a font").unwrap();
    assert!(!tags::supported());
    assert_eq!(tags::STORE, None);
    assert!(matches!(
        tags::read(&file),
        Err(PlatformError::Unsupported(_))
    ));
    assert!(matches!(
        tags::write(&file, &["serif".into()]),
        Err(PlatformError::Unsupported(_))
    ));
    let _ = std::fs::remove_dir_all(&dir);
}

#[cfg(unix)]
#[test]
fn a_tag_written_to_a_file_is_there_when_it_is_read_back() {
    let dir = scratch("roundtrip");
    let file = dir.join("f.ttf");
    std::fs::write(&file, b"not really a font").unwrap();

    assert!(tags::supported());
    assert!(tags::STORE.is_some());

    // A file nobody has tagged has no tags, and that is not an error.
    assert_eq!(tags::read(&file).unwrap(), Vec::<String>::new());

    let wanted: Vec<String> = vec!["shortlist".into(), "serif".into(), "Ελληνικά".into()];
    match tags::write(&file, &wanted) {
        Ok(()) => {}
        // A filesystem without extended attributes is a real configuration, and the one
        // thing that must not happen is a panic or a wrong answer.
        Err(PlatformError::NoTags(p)) => {
            eprintln!("skipped: {} has no extended attributes", p.display());
            let _ = std::fs::remove_dir_all(&dir);
            return;
        }
        Err(e) => panic!("writing tags: {e}"),
    }

    let mut expected = wanted.clone();
    expected.sort();
    assert_eq!(tags::read(&file).unwrap(), expected);

    // Writing replaces rather than merges: the reader is the one that decides.
    tags::write(&file, &["display".into()]).unwrap();
    assert_eq!(tags::read(&file).unwrap(), vec!["display".to_string()]);

    // Duplicates collapse, so a caller need not have deduplicated first.
    tags::write(&file, &["a".into(), "a".into(), "b".into()]).unwrap();
    assert_eq!(
        tags::read(&file).unwrap(),
        vec!["a".to_string(), "b".to_string()]
    );

    // Untagging removes the attribute, so a file fontina has untagged looks exactly like
    // one it never touched.
    tags::write(&file, &[]).unwrap();
    assert_eq!(tags::read(&file).unwrap(), Vec::<String>::new());

    let _ = std::fs::remove_dir_all(&dir);
}

#[cfg(unix)]
#[test]
fn a_tag_the_store_cannot_hold_is_refused_before_anything_is_written() {
    let dir = scratch("refuse");
    let file = dir.join("f.ttf");
    std::fs::write(&file, b"not really a font").unwrap();

    if tags::write(&file, &["keep".into()]).is_err() {
        eprintln!("skipped: no extended attributes here");
        let _ = std::fs::remove_dir_all(&dir);
        return;
    }
    let err = tags::write(&file, &["fine".into(), "not,fine".into()]).unwrap_err();
    assert!(format!("{err}").contains("comma"), "{err}");
    assert_eq!(
        tags::read(&file).unwrap(),
        vec!["keep".to_string()],
        "a refused write leaves what was there"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Finder stores a coloured tag as `Name\n<0-7>`. Reading gives the name; writing the
/// same name back must not throw the colour away, or every tag fontina touches goes grey.
///
/// The attribute is written here by `xattr(1)` as an XML property list, which is what a
/// third party writing this attribute may well do — and reading it proves the parser is
/// CoreFoundation's rather than a binary-only reader of our own.
#[cfg(target_os = "macos")]
#[test]
fn a_finder_colour_survives_fontina_writing_the_same_tag_back() {
    const ATTR: &str = "com.apple.metadata:_kMDItemUserTags";
    let dir = scratch("colour");
    let file = dir.join("f.ttf");
    std::fs::write(&file, b"not really a font").unwrap();

    let xml = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
               <plist version=\"1.0\"><array><string>Red\n6</string></array></plist>";
    let out = std::process::Command::new("xattr")
        .args(["-w", ATTR, xml])
        .arg(&file)
        .output();
    match out {
        Ok(o) if o.status.success() => {}
        Ok(o) => {
            eprintln!("skipped: {}", String::from_utf8_lossy(&o.stderr).trim_end());
            let _ = std::fs::remove_dir_all(&dir);
            return;
        }
        Err(e) => {
            eprintln!("skipped: xattr(1) did not run: {e}");
            let _ = std::fs::remove_dir_all(&dir);
            return;
        }
    }

    // The name, without the colour that follows it.
    assert_eq!(tags::read(&file).unwrap(), vec!["Red".to_string()]);

    // Writing it back beside a new tag keeps the colour on the one that had it.
    tags::write(&file, &["Red".into(), "serif".into()]).unwrap();
    let hex = std::process::Command::new("xattr")
        .args(["-px", ATTR])
        .arg(&file)
        .output()
        .expect("xattr(1) runs");
    let digits: String = String::from_utf8_lossy(&hex.stdout)
        .chars()
        .filter(|c| c.is_ascii_hexdigit())
        .collect();
    let bytes: Vec<u8> = digits
        .as_bytes()
        .chunks(2)
        .filter_map(|p| u8::from_str_radix(std::str::from_utf8(p).ok()?, 16).ok())
        .collect();
    let text = String::from_utf8_lossy(&bytes);
    assert!(
        text.contains("Red\n6"),
        "the colour was thrown away: {text:?}"
    );
    assert_eq!(
        tags::read(&file).unwrap(),
        vec!["Red".to_string(), "serif".to_string()]
    );

    let _ = std::fs::remove_dir_all(&dir);
}
