//! The Windows Open dialog. It runs on its own thread, so the app keeps
//! drawing while it is open; the main window is its owner, so it stays in
//! front of it.

use std::ffi::OsString;
use std::mem::{size_of, zeroed};
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;
use std::ptr::{null, null_mut};
use std::sync::mpsc::{self, Receiver};

use eframe::egui;
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::System::Com::{
    COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoInitializeEx, CoUninitialize,
};
use windows_sys::Win32::UI::Controls::Dialogs::{
    GetOpenFileNameW, OFN_EXPLORER, OFN_FILEMUSTEXIST, OFN_HIDEREADONLY, OFN_NOCHANGEDIR,
    OFN_PATHMUSTEXIST, OPENFILENAMEW,
};

use super::{MainWindow, wide};

/// Shows the Open dialog for images. The receiver gets the chosen file, or
/// `None` if the dialog was cancelled.
pub fn open_image_dialog(
    ctx: &egui::Context,
    owner: Option<MainWindow>,
) -> Receiver<Option<PathBuf>> {
    let (tx, rx) = mpsc::channel();
    let ctx = ctx.clone();
    let owner = owner.map(MainWindow::raw);
    std::thread::spawn(move || {
        let _ = tx.send(run(owner));
        ctx.request_repaint();
    });
    rx
}

fn run(owner: Option<isize>) -> Option<PathBuf> {
    // Pairs of name and pattern, each NUL-terminated, ending with an extra NUL.
    let filter: Vec<u16> = [
        "Images (PNG, JPEG, BMP, GIF, WebP)",
        "*.png;*.jpg;*.jpeg;*.bmp;*.gif;*.webp",
        "All files",
        "*.*",
    ]
    .iter()
    .flat_map(|part| wide(part))
    .chain(std::iter::once(0))
    .collect();
    let title = wide("Open an image");
    let mut file = vec![0u16; 32 * 1024];

    // SAFETY: COM is set up for this thread only; every pointer in the
    // struct points to a local that outlives the call.
    let chosen = unsafe {
        let com = CoInitializeEx(
            null(),
            (COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) as _,
        );
        let mut ofn: OPENFILENAMEW = zeroed();
        ofn.lStructSize = size_of::<OPENFILENAMEW>() as u32;
        ofn.hwndOwner = owner.map_or(null_mut(), |h| h as HWND);
        ofn.lpstrFilter = filter.as_ptr();
        ofn.lpstrFile = file.as_mut_ptr();
        ofn.nMaxFile = file.len() as u32;
        ofn.lpstrTitle = title.as_ptr();
        ofn.Flags = OFN_EXPLORER
            | OFN_FILEMUSTEXIST
            | OFN_PATHMUSTEXIST
            | OFN_HIDEREADONLY
            | OFN_NOCHANGEDIR;
        let chosen = GetOpenFileNameW(&mut ofn) != 0;
        if com >= 0 {
            CoUninitialize();
        }
        chosen
    };
    if !chosen {
        return None;
    }
    let len = file.iter().position(|&c| c == 0).unwrap_or(file.len());
    Some(PathBuf::from(OsString::from_wide(&file[..len])))
}
