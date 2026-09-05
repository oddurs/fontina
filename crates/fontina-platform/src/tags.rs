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

//! The operating system's own file tags.
//!
//! fontina keeps tags in its index, which is fast, searchable and survives a font moving.
//! It is also invisible to everything else. A tag written where the desktop keeps its own
//! is a tag your file manager shows, your search indexer finds, and a backup carries —
//! and it stays yours if you stop using fontina, which is the point.
//!
//! | OS | where | shape |
//! |---|---|---|
//! | macOS | `com.apple.metadata:_kMDItemUserTags` | binary property list, an array of strings |
//! | GNU/Linux | `user.xdg.tags` | comma-separated UTF-8 |
//! | Windows | — | [`PlatformError::Unsupported`] |
//!
//! Windows has no per-file tag store for an arbitrary file. The Property System's
//! keywords are per-format — Office documents and some media have them, a font file does
//! not — so there is nowhere honest to put them, and this says so rather than inventing a
//! sidecar file that only fontina would ever read.

use crate::{PlatformError, Result};
use std::path::Path;

/// Where this platform keeps a file's tags, or `None` if it has nowhere.
pub const STORE: Option<&str> = imp::STORE;

/// Whether tags can be written to files at all here.
pub fn supported() -> bool {
    STORE.is_some()
}

/// Why `tag` cannot go in a file's own metadata, if it cannot.
///
/// The two stores disagree about what a tag may contain — a comma ends one on GNU/Linux,
/// a newline introduces the colour on macOS — and a tag that survives on one machine and
/// splits in two on another is worse than one that was refused. So both refuse both.
pub fn unstorable(tag: &str) -> Option<&'static str> {
    if tag.trim().is_empty() {
        Some("a tag with nothing in it")
    } else if tag.contains(',') {
        Some("a comma separates tags in `user.xdg.tags`, so a tag cannot contain one")
    } else if tag.contains('\n') {
        Some("a newline introduces the colour in a Finder tag, so a tag cannot contain one")
    } else {
        None
    }
}

/// The tags the operating system holds for `file`, sorted and without duplicates.
pub fn read(file: &Path) -> Result<Vec<String>> {
    let mut tags = imp::read(file)?;
    tags.sort();
    tags.dedup();
    Ok(tags)
}

/// Replace the operating system's tags for `file`.
///
/// An empty list removes the attribute rather than storing an empty one, so a file
/// fontina has untagged looks the same as one it never touched.
pub fn write(file: &Path, tags: &[String]) -> Result<()> {
    for t in tags {
        if let Some(why) = unstorable(t) {
            return Err(PlatformError::Os(format!("{t:?}: {why}")));
        }
    }
    let mut tags = tags.to_vec();
    tags.sort();
    tags.dedup();
    imp::write(file, &tags)
}

// ---------------------------------------------------------------------------------
// The extended attribute calls, which differ between the two Unixes in their arguments
// rather than in what they do.
// ---------------------------------------------------------------------------------

#[cfg(unix)]
mod xattr {
    use super::{PlatformError, Result};
    use std::ffi::{CString, c_char, c_void};
    use std::os::unix::ffi::OsStrExt;
    use std::path::Path;

    fn cpath(p: &Path) -> Result<CString> {
        CString::new(p.as_os_str().as_bytes())
            .map_err(|_| PlatformError::Os(format!("{} has a NUL byte in its name", p.display())))
    }

    /// `errno` for "this file has no such attribute", which is not an error to us.
    #[cfg(target_os = "macos")]
    const ABSENT: i32 = libc::ENOATTR;
    #[cfg(not(target_os = "macos"))]
    const ABSENT: i32 = libc::ENODATA;

    /// A filesystem with extended attributes turned off or never implemented.
    fn no_tags(path: &Path, e: &std::io::Error) -> Option<PlatformError> {
        matches!(e.raw_os_error(), Some(libc::ENOTSUP))
            .then(|| PlatformError::NoTags(path.to_path_buf()))
    }

    unsafe fn get_raw(
        path: *const c_char,
        name: *const c_char,
        buf: *mut c_void,
        n: usize,
    ) -> isize {
        #[cfg(target_os = "macos")]
        unsafe {
            libc::getxattr(path, name, buf, n, 0, 0)
        }
        #[cfg(not(target_os = "macos"))]
        unsafe {
            libc::getxattr(path, name, buf, n)
        }
    }

    unsafe fn set_raw(
        path: *const c_char,
        name: *const c_char,
        buf: *const c_void,
        n: usize,
    ) -> i32 {
        #[cfg(target_os = "macos")]
        unsafe {
            libc::setxattr(path, name, buf, n, 0, 0)
        }
        #[cfg(not(target_os = "macos"))]
        unsafe {
            libc::setxattr(path, name, buf, n, 0)
        }
    }

    unsafe fn remove_raw(path: *const c_char, name: *const c_char) -> i32 {
        #[cfg(target_os = "macos")]
        unsafe {
            libc::removexattr(path, name, 0)
        }
        #[cfg(not(target_os = "macos"))]
        unsafe {
            libc::removexattr(path, name)
        }
    }

    /// The attribute's bytes, or `None` if the file does not have it.
    pub fn get(file: &Path, name: &str) -> Result<Option<Vec<u8>>> {
        let path = cpath(file)?;
        let name = CString::new(name).expect("attribute names are literals without NUL");
        // Ask for the size, then for the value. Another process can grow it in between,
        // so this reads until the buffer it asked for was big enough.
        loop {
            let n = unsafe { get_raw(path.as_ptr(), name.as_ptr(), std::ptr::null_mut(), 0) };
            if n < 0 {
                let e = std::io::Error::last_os_error();
                if e.raw_os_error() == Some(ABSENT) {
                    return Ok(None);
                }
                return Err(no_tags(file, &e).unwrap_or_else(|| PlatformError::Io(file.into(), e)));
            }
            let mut buf = vec![0u8; n as usize];
            let got = unsafe {
                get_raw(
                    path.as_ptr(),
                    name.as_ptr(),
                    buf.as_mut_ptr().cast(),
                    buf.len(),
                )
            };
            if got >= 0 {
                buf.truncate(got as usize);
                return Ok(Some(buf));
            }
            let e = std::io::Error::last_os_error();
            if e.raw_os_error() == Some(ABSENT) {
                return Ok(None);
            }
            // ERANGE means it grew between the two calls; anything else is real.
            if e.raw_os_error() != Some(libc::ERANGE) {
                return Err(no_tags(file, &e).unwrap_or_else(|| PlatformError::Io(file.into(), e)));
            }
        }
    }

    pub fn set(file: &Path, name: &str, value: &[u8]) -> Result<()> {
        let path = cpath(file)?;
        let name = CString::new(name).expect("attribute names are literals without NUL");
        let rc = unsafe {
            set_raw(
                path.as_ptr(),
                name.as_ptr(),
                value.as_ptr().cast(),
                value.len(),
            )
        };
        if rc == 0 {
            return Ok(());
        }
        let e = std::io::Error::last_os_error();
        Err(no_tags(file, &e).unwrap_or_else(|| PlatformError::Io(file.into(), e)))
    }

    /// Remove the attribute. A file that does not have it is already how we want it.
    pub fn remove(file: &Path, name: &str) -> Result<()> {
        let path = cpath(file)?;
        let name = CString::new(name).expect("attribute names are literals without NUL");
        if unsafe { remove_raw(path.as_ptr(), name.as_ptr()) } == 0 {
            return Ok(());
        }
        let e = std::io::Error::last_os_error();
        if e.raw_os_error() == Some(ABSENT) {
            return Ok(());
        }
        Err(no_tags(file, &e).unwrap_or_else(|| PlatformError::Io(file.into(), e)))
    }
}

// ---------------------------------------------------------------------------------
// macOS: a binary property list, read and written by the system's own parser.
// ---------------------------------------------------------------------------------

#[cfg(target_os = "macos")]
mod imp {
    use super::{PlatformError, Result, xattr};
    use core_foundation::array::CFArray;
    use core_foundation::base::{CFType, TCFType};
    use core_foundation::data::CFData;
    use core_foundation::propertylist::{
        CFPropertyList, create_data, create_with_data, kCFPropertyListBinaryFormat_v1_0,
        kCFPropertyListImmutable,
    };
    use core_foundation::string::CFString;
    use std::path::Path;

    pub const STORE: Option<&str> = Some(ATTRIBUTE);
    const ATTRIBUTE: &str = "com.apple.metadata:_kMDItemUserTags";

    /// The entries as Finder stores them: `Name`, or `Name\n<colour 0-7>`.
    ///
    /// Parsing is `CFPropertyListCreateWithData` rather than a binary-plist reader of our
    /// own. It is the implementation Finder itself uses, so the two cannot disagree, and
    /// it costs no dependency — CoreFoundation is already here for `CTFontManager`.
    fn raw(file: &Path) -> Result<Vec<String>> {
        let Some(bytes) = xattr::get(file, ATTRIBUTE)? else {
            return Ok(Vec::new());
        };
        let (list, _format) =
            create_with_data(CFData::from_buffer(&bytes), kCFPropertyListImmutable).map_err(
                |e| {
                    PlatformError::Os(format!(
                        "{} holds tags that are not a property list: {e}",
                        file.display()
                    ))
                },
            )?;
        let list = unsafe { CFPropertyList::wrap_under_create_rule(list) };
        let Some(array) = list.downcast_into::<CFArray>() else {
            return Err(PlatformError::Os(format!(
                "{} holds tags that are not a list",
                file.display()
            )));
        };
        let mut out = Vec::new();
        for item in array.iter() {
            // Somebody else wrote this file. An entry that is not a string is skipped
            // rather than reinterpreted as one.
            let value = unsafe { CFType::wrap_under_get_rule(*item) };
            if let Some(s) = value.downcast::<CFString>() {
                out.push(s.to_string());
            }
        }
        Ok(out)
    }

    pub fn read(file: &Path) -> Result<Vec<String>> {
        Ok(raw(file)?
            .into_iter()
            .map(|r| r.split('\n').next().unwrap_or_default().to_string())
            .filter(|s| !s.is_empty())
            .collect())
    }

    pub fn write(file: &Path, tags: &[String]) -> Result<()> {
        if tags.is_empty() {
            return xattr::remove(file, ATTRIBUTE);
        }
        // A tag the user gave a colour to in Finder keeps it. Writing the bare name back
        // would silently grey out every tag fontina touched.
        let coloured: Vec<(String, String)> = raw(file)?
            .into_iter()
            .filter_map(|r| r.split_once('\n').map(|(n, _)| (n.to_string(), r.clone())))
            .collect();
        let items: Vec<CFString> = tags
            .iter()
            .map(|t| {
                let stored = coloured
                    .iter()
                    .find(|(name, _)| name == t)
                    .map(|(_, raw)| raw.as_str())
                    .unwrap_or(t.as_str());
                CFString::new(stored)
            })
            .collect();
        let array = CFArray::from_CFTypes(&items);
        let data = create_data(array.as_CFTypeRef(), kCFPropertyListBinaryFormat_v1_0)
            .map_err(|e| PlatformError::Os(format!("writing tags for {}: {e}", file.display())))?;
        xattr::set(file, ATTRIBUTE, data.bytes())
    }
}

// ---------------------------------------------------------------------------------
// GNU/Linux and the other Unixes: the freedesktop attribute, comma-separated.
// ---------------------------------------------------------------------------------

#[cfg(all(unix, not(target_os = "macos")))]
mod imp {
    use super::{PlatformError, Result, xattr};
    use std::path::Path;

    pub const STORE: Option<&str> = Some(ATTRIBUTE);
    const ATTRIBUTE: &str = "user.xdg.tags";

    pub fn read(file: &Path) -> Result<Vec<String>> {
        let Some(bytes) = xattr::get(file, ATTRIBUTE)? else {
            return Ok(Vec::new());
        };
        let text = String::from_utf8(bytes).map_err(|_| {
            PlatformError::Os(format!("{} holds tags that are not UTF-8", file.display()))
        })?;
        Ok(text
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect())
    }

    pub fn write(file: &Path, tags: &[String]) -> Result<()> {
        if tags.is_empty() {
            return xattr::remove(file, ATTRIBUTE);
        }
        xattr::set(file, ATTRIBUTE, tags.join(",").as_bytes())
    }
}

// ---------------------------------------------------------------------------------
// Windows: nowhere to put them.
// ---------------------------------------------------------------------------------

#[cfg(target_os = "windows")]
mod imp {
    use super::{PlatformError, Result};
    use std::path::Path;

    pub const STORE: Option<&str> = None;

    const WHY: &str =
        "file tags — Windows keeps keywords per file format, and a font file has none";

    pub fn read(_file: &Path) -> Result<Vec<String>> {
        Err(PlatformError::Unsupported(WHY))
    }

    pub fn write(_file: &Path, _tags: &[String]) -> Result<()> {
        Err(PlatformError::Unsupported(WHY))
    }
}
