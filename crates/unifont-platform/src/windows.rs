//! Windows: GDI's `AddFontResourceExW` makes a file visible to every process for the
//! session; a value under `HKCU\Software\Microsoft\Windows NT\CurrentVersion\Fonts`
//! makes it persistent for the user (Windows 10 1809 and later load per-user fonts from
//! any path at logon). Persistent install copies into the per-user fonts directory and
//! writes the same registry value. `WM_FONTCHANGE` is broadcast after every change.

use super::{FontActivator, PlatformError, Result, Scope, io, is_under, regular_file, slot_name};
use std::path::{Path, PathBuf};
use windows_sys::Win32::Foundation::{ERROR_NO_MORE_ITEMS, ERROR_SUCCESS};
use windows_sys::Win32::Graphics::Gdi::{
    AddFontResourceExW, AddFontResourceW, RemoveFontResourceExW, RemoveFontResourceW,
};
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ, RegCloseKey,
    RegCreateKeyExW, RegDeleteValueW, RegEnumValueW, RegSetValueExW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    HWND_BROADCAST, SMTO_ABORTIFHUNG, SendMessageTimeoutW, WM_FONTCHANGE,
};

pub struct Gdi;

const FONTS_KEY: &str = "Software\\Microsoft\\Windows NT\\CurrentVersion\\Fonts";

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn path_wide(p: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    p.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

fn broadcast_font_change() {
    let mut result = 0usize;
    // SAFETY: plain Win32 call with valid arguments; the timeout keeps a hung window
    // from blocking us.
    unsafe {
        SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_FONTCHANGE,
            0,
            0,
            SMTO_ABORTIFHUNG,
            1000,
            &mut result,
        );
    }
}

struct Key(HKEY);

impl Key {
    fn fonts() -> Result<Key> {
        let mut hkey: HKEY = std::ptr::null_mut();
        let sub = wide(FONTS_KEY);
        // SAFETY: valid out-pointer and NUL-terminated subkey.
        let status = unsafe {
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                sub.as_ptr(),
                0,
                std::ptr::null(),
                REG_OPTION_NON_VOLATILE,
                KEY_READ | KEY_WRITE,
                std::ptr::null(),
                &mut hkey,
                std::ptr::null_mut(),
            )
        };
        if status != ERROR_SUCCESS {
            return Err(PlatformError::Os(format!(
                "cannot open HKCU\\{FONTS_KEY} (error {status})"
            )));
        }
        Ok(Key(hkey))
    }

    fn set(&self, name: &str, value: &Path) -> Result<()> {
        let n = wide(name);
        let v = path_wide(value);
        // SAFETY: buffers are NUL-terminated and sized in bytes.
        let status = unsafe {
            RegSetValueExW(
                self.0,
                n.as_ptr(),
                0,
                REG_SZ,
                v.as_ptr() as *const u8,
                (v.len() * 2) as u32,
            )
        };
        if status != ERROR_SUCCESS {
            return Err(PlatformError::Os(format!(
                "cannot write registry value {name:?} (error {status})"
            )));
        }
        Ok(())
    }

    /// Delete every value whose data is `value`. Returns how many were removed.
    fn delete_pointing_at(&self, value: &Path) -> Result<usize> {
        let target = value.to_string_lossy().to_lowercase();
        let mut names = Vec::new();
        let mut index = 0u32;
        loop {
            let mut name = vec![0u16; 16_384];
            let mut name_len = name.len() as u32;
            let mut data = vec![0u8; 32_768];
            let mut data_len = data.len() as u32;
            let mut kind = 0u32;
            // SAFETY: buffers and lengths match.
            let status = unsafe {
                RegEnumValueW(
                    self.0,
                    index,
                    name.as_mut_ptr(),
                    &mut name_len,
                    std::ptr::null(),
                    &mut kind,
                    data.as_mut_ptr(),
                    &mut data_len,
                )
            };
            if status == ERROR_NO_MORE_ITEMS {
                break;
            }
            if status != ERROR_SUCCESS {
                return Err(PlatformError::Os(format!(
                    "cannot enumerate registry values (error {status})"
                )));
            }
            index += 1;
            if kind != REG_SZ {
                continue;
            }
            let data_u16: Vec<u16> = data[..data_len as usize]
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .take_while(|&c| c != 0)
                .collect();
            if String::from_utf16_lossy(&data_u16).to_lowercase() == target {
                names.push(name[..name_len as usize].to_vec());
            }
        }
        for n in &names {
            let mut n = n.clone();
            n.push(0);
            // SAFETY: NUL-terminated name.
            unsafe { RegDeleteValueW(self.0, n.as_ptr()) };
        }
        Ok(names.len())
    }
}

impl Drop for Key {
    fn drop(&mut self) {
        // SAFETY: handle came from RegCreateKeyExW.
        unsafe { RegCloseKey(self.0) };
    }
}

/// The registry value name Windows shows for a font file: the stem plus the format.
fn value_name(file: &Path) -> String {
    let stem = file
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Font".into());
    let kind = match file
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .as_deref()
    {
        Some("otf") | Some("otc") => "OpenType",
        _ => "TrueType",
    };
    format!("{stem} ({kind})")
}

pub fn user_fonts_dir() -> Result<PathBuf> {
    std::env::var_os("LOCALAPPDATA")
        .map(|l| {
            PathBuf::from(l)
                .join("Microsoft")
                .join("Windows")
                .join("Fonts")
        })
        .ok_or(PlatformError::NoUserDir)
}

fn add_resource(file: &Path, persistent: bool) -> Result<()> {
    let w = path_wide(file);
    // SAFETY: NUL-terminated path.
    let added = unsafe {
        if persistent {
            AddFontResourceW(w.as_ptr())
        } else {
            AddFontResourceExW(w.as_ptr(), 0, std::ptr::null())
        }
    };
    if added == 0 {
        return Err(PlatformError::Os(format!(
            "{}: AddFontResource failed (not a usable font file?)",
            file.display()
        )));
    }
    Ok(())
}

fn remove_resource(file: &Path) {
    let w = path_wide(file);
    // GDI reference-counts font resources; remove until it reports none left.
    for _ in 0..64 {
        // SAFETY: NUL-terminated path.
        let ok = unsafe { RemoveFontResourceExW(w.as_ptr(), 0, std::ptr::null()) };
        if ok == 0 {
            break;
        }
    }
    for _ in 0..64 {
        // SAFETY: NUL-terminated path.
        let ok = unsafe { RemoveFontResourceW(w.as_ptr()) };
        if ok == 0 {
            break;
        }
    }
}

impl FontActivator for Gdi {
    fn install(&self, file: &Path) -> Result<PathBuf> {
        let file = regular_file(file)?;
        let dir = user_fonts_dir()?;
        std::fs::create_dir_all(&dir).map_err(io(&dir))?;
        let same_bytes = |p: &Path| {
            std::fs::read(p).ok().map(|b| blake3::hash(&b))
                == std::fs::read(&file).ok().map(|b| blake3::hash(&b))
        };
        let slot = slot_name(&dir, &file, same_bytes);
        if !slot.exists() {
            std::fs::copy(&file, &slot).map_err(io(&slot))?;
        }
        Key::fonts()?.set(&value_name(&slot), &slot)?;
        add_resource(&slot, true)?;
        broadcast_font_change();
        Ok(slot)
    }

    fn uninstall(&self, installed: &Path) -> Result<()> {
        let dir = user_fonts_dir()?;
        if !is_under(installed, &dir) {
            return Err(PlatformError::NotManaged(installed.to_path_buf(), dir));
        }
        remove_resource(installed);
        Key::fonts()?.delete_pointing_at(installed)?;
        match std::fs::remove_file(installed) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(PlatformError::Io(installed.to_path_buf(), e)),
        }
        broadcast_font_change();
        Ok(())
    }

    fn activate(&self, file: &Path, scope: Scope) -> Result<()> {
        let file = regular_file(file)?;
        add_resource(&file, false)?;
        if scope == Scope::User {
            Key::fonts()?.set(&value_name(&file), &file)?;
        }
        broadcast_font_change();
        Ok(())
    }

    fn deactivate(&self, file: &Path) -> Result<()> {
        let file = std::fs::canonicalize(file).unwrap_or_else(|_| file.to_path_buf());
        remove_resource(&file);
        Key::fonts()?.delete_pointing_at(&file)?;
        broadcast_font_change();
        Ok(())
    }
}

#[cfg(all(test, feature = "platform-tests"))]
mod tests {
    use super::*;

    fn fixture() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/Amiri-Regular.ttf")
    }

    #[test]
    fn session_activation_round_trips() {
        let a = Gdi;
        a.activate(&fixture(), Scope::Session).unwrap();
        a.deactivate(&fixture()).unwrap();
        a.deactivate(&fixture()).unwrap();
        assert!(
            a.activate(Path::new("C:\\nonexistent.ttf"), Scope::Session)
                .is_err()
        );
    }

    #[test]
    fn install_copies_and_registers() {
        let a = Gdi;
        let installed = a.install(&fixture()).unwrap();
        assert!(installed.starts_with(user_fonts_dir().unwrap()));
        assert_eq!(a.install(&fixture()).unwrap(), installed);
        a.uninstall(&installed).unwrap();
        assert!(!installed.exists());
        assert!(matches!(
            a.uninstall(&fixture()),
            Err(PlatformError::NotManaged(..))
        ));
    }
}
