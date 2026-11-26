//! What tabs ask the app to do. Tabs draw from read-only state and push
//! actions; the app applies them in one place after drawing.

use crate::color::Rgb;

#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    /// Make this the current color (editor, history, palette).
    SetColor(Rgb),
    /// A color picked from the screen or an image: it becomes the current
    /// color, goes into the history and may be copied.
    Picked(Rgb),
    /// Copy text to the clipboard.
    Copy(String),
    StartScreenPick,
    CancelScreenPick,
    AddToPalette(Rgb),
    RemoveFromPalette(usize),
    ClearPalette,
    RemoveFromHistory(usize),
    ClearHistory,
    /// Settings were edited: save them and apply what changed.
    SettingsChanged,
    /// Show a short message in the title bar.
    Toast(String),
}
