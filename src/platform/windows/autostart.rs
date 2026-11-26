//! Starting with Windows: a value under the current user's `Run` key. No
//! admin rights needed; it launches the app with `--startup`, which starts
//! it quietly in the tray.

use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
use windows_sys::Win32::System::Registry::{
    HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_SZ, RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW,
};

use super::wide;

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE: &str = "Color Picker";
/// The argument the `Run` entry passes.
pub const STARTUP_ARG: &str = "--startup";

pub fn is_enabled() -> bool {
    let (key, value) = (wide(RUN_KEY), wide(VALUE));
    let mut size = 0u32;
    // SAFETY: asks only for the value's size; strings are NUL-terminated.
    unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut size,
        ) == ERROR_SUCCESS
    }
}

/// Adds or removes the `Run` entry. Adding also refreshes the path, so the
/// entry follows the .exe if it was moved.
pub fn set(enabled: bool) -> Result<(), String> {
    let (key, value) = (wide(RUN_KEY), wide(VALUE));
    let status = if enabled {
        let exe = std::env::current_exe().map_err(|e| format!("Can't find the app: {e}"))?;
        let command = wide(&format!("\"{}\" {STARTUP_ARG}", exe.display()));
        // SAFETY: REG_SZ data is the NUL-terminated UTF-16 string, size in bytes.
        unsafe {
            RegSetKeyValueW(
                HKEY_CURRENT_USER,
                key.as_ptr(),
                value.as_ptr(),
                REG_SZ,
                command.as_ptr().cast(),
                (command.len() * 2) as u32,
            )
        }
    } else {
        // SAFETY: NUL-terminated strings.
        match unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, key.as_ptr(), value.as_ptr()) } {
            ERROR_FILE_NOT_FOUND => ERROR_SUCCESS,
            status => status,
        }
    };
    if status == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(format!("Windows refused (error {status})"))
    }
}
