//! The global pick hotkey: modifiers plus one key, its label and its text
//! form in the settings file.

use eframe::egui::Key;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hotkey {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub win: bool,
    pub key: Key,
}

impl Default for Hotkey {
    /// Ctrl + Alt + C: rarely taken by other apps.
    fn default() -> Self {
        Self {
            ctrl: true,
            alt: true,
            shift: false,
            win: false,
            key: Key::C,
        }
    }
}

impl Hotkey {
    fn modifiers(self) -> impl Iterator<Item = &'static str> {
        // Windows order: Win + Ctrl + Alt + Shift.
        [
            (self.win, "Win"),
            (self.ctrl, "Ctrl"),
            (self.alt, "Alt"),
            (self.shift, "Shift"),
        ]
        .into_iter()
        .filter_map(|(on, name)| on.then_some(name))
    }

    fn has_modifier(self) -> bool {
        self.ctrl || self.alt || self.shift || self.win
    }

    /// For display, e.g. `Ctrl + Alt + C`.
    pub fn label(self) -> String {
        let key = match self.key {
            Key::ArrowDown | Key::ArrowLeft | Key::ArrowRight | Key::ArrowUp => self.key.name(),
            Key::Minus => "-",
            key => key.symbol_or_name(),
        };
        self.modifiers()
            .chain(std::iter::once(key))
            .collect::<Vec<_>>()
            .join(" + ")
    }

    /// For the settings file, e.g. `Ctrl+Alt+C`. Keys use egui's names
    /// (`Plus`, `Backtick`), so `+` is only ever a separator.
    pub fn to_text(self) -> String {
        self.modifiers()
            .chain(std::iter::once(self.key.name()))
            .collect::<Vec<_>>()
            .join("+")
    }

    /// Parses [`Self::to_text`]. Case and spaces around parts don't matter.
    pub fn parse(text: &str) -> Option<Self> {
        let mut hotkey = Self {
            ctrl: false,
            alt: false,
            shift: false,
            win: false,
            key: Key::A,
        };
        let mut key = None;
        for part in text.split('+').map(str::trim) {
            match part.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => hotkey.ctrl = true,
                "alt" => hotkey.alt = true,
                "shift" => hotkey.shift = true,
                "win" | "super" | "meta" => hotkey.win = true,
                _ if key.is_none() => key = Some(Key::from_name(part)?),
                _ => return None,
            }
        }
        hotkey.key = key?;
        hotkey.is_valid().then_some(hotkey)
    }

    /// Whether this can be a global hotkey: it needs a modifier (except
    /// F-keys), a Windows key code, and must not be a key we use while
    /// recording or picking.
    pub fn is_valid(self) -> bool {
        let reserved = matches!(self.key, Key::Escape | Key::Backspace | Key::Delete);
        match self.virtual_key() {
            None => false,
            Some(_) if reserved => false,
            // F1-F24 work alone; everything else needs a modifier.
            Some(0x70..=0x87) => true,
            Some(_) => self.has_modifier(),
        }
    }

    /// The Windows virtual-key code, or `None` for keys without one.
    pub fn virtual_key(self) -> Option<u16> {
        virtual_key(self.key)
    }
}

/// Maps an egui key to its Windows virtual-key code. Shifted symbols map to
/// the key that types them on a US layout (`?` is the `/` key).
fn virtual_key(key: Key) -> Option<u16> {
    use Key::*;
    const LETTERS: [Key; 26] = [
        A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    ];
    const DIGITS: [Key; 10] = [Num0, Num1, Num2, Num3, Num4, Num5, Num6, Num7, Num8, Num9];
    const FUNCTION: [Key; 24] = [
        F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12, F13, F14, F15, F16, F17, F18, F19, F20,
        F21, F22, F23, F24,
    ];

    let index_in = |keys: &[Key]| keys.iter().position(|&k| k == key).map(|i| i as u16);
    if let Some(i) = index_in(&LETTERS) {
        return Some(0x41 + i);
    }
    if let Some(i) = index_in(&DIGITS) {
        return Some(0x30 + i);
    }
    if let Some(i) = index_in(&FUNCTION) {
        return Some(0x70 + i);
    }
    Some(match key {
        Backspace => 0x08,
        Tab => 0x09,
        Enter => 0x0D,
        Escape => 0x1B,
        Space => 0x20,
        PageUp => 0x21,
        PageDown => 0x22,
        End => 0x23,
        Home => 0x24,
        ArrowLeft => 0x25,
        ArrowUp => 0x26,
        ArrowRight => 0x27,
        ArrowDown => 0x28,
        Insert => 0x2D,
        Delete => 0x2E,
        Exclamationmark => 0x31,
        Semicolon | Colon => 0xBA,
        Plus | Equals => 0xBB,
        Comma => 0xBC,
        Minus => 0xBD,
        Period => 0xBE,
        Slash | Questionmark => 0xBF,
        Backtick => 0xC0,
        OpenBracket | OpenCurlyBracket => 0xDB,
        Backslash | Pipe => 0xDC,
        CloseBracket | CloseCurlyBracket => 0xDD,
        Quote => 0xDE,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_ctrl_alt_c() {
        let hotkey = Hotkey::default();
        assert_eq!(hotkey.label(), "Ctrl + Alt + C");
        assert_eq!(hotkey.to_text(), "Ctrl+Alt+C");
        assert_eq!(hotkey.virtual_key(), Some(0x43));
        assert!(hotkey.is_valid());
    }

    #[test]
    fn text_round_trip() {
        for text in [
            "Ctrl+Alt+C",
            "Win+Shift+C",
            "Alt+Backtick",
            "Ctrl+Plus",
            "F8",
            "Shift+F12",
        ] {
            let hotkey = Hotkey::parse(text).unwrap_or_else(|| panic!("{text}"));
            assert_eq!(hotkey.to_text(), text);
        }
        assert_eq!(Hotkey::parse(" ctrl + alt + c "), Some(Hotkey::default()));
    }

    #[test]
    fn labels_use_symbols() {
        assert_eq!(
            Hotkey::parse("Alt+Backtick").map(Hotkey::label).as_deref(),
            Some("Alt + `")
        );
        assert_eq!(
            Hotkey::parse("Ctrl+Minus").map(Hotkey::label).as_deref(),
            Some("Ctrl + -")
        );
        assert_eq!(
            Hotkey::parse("Ctrl+Up").map(Hotkey::label).as_deref(),
            Some("Ctrl + Up")
        );
    }

    #[test]
    fn rejects_bad_hotkeys() {
        for text in [
            "",
            "C",
            "Ctrl",
            "Ctrl+",
            "Ctrl+Alt",
            "Ctrl+C+D",
            "Ctrl+Nope",
            "Escape",
            "Ctrl+Escape",
            "Alt+F35",
        ] {
            assert_eq!(Hotkey::parse(text), None, "accepted {text:?}");
        }
    }

    #[test]
    fn function_keys_alone_are_valid() {
        assert!(Hotkey::parse("F1").is_some());
        assert!(Hotkey::parse("F24").is_some());
        assert_eq!(
            Hotkey::parse("F24").and_then(Hotkey::virtual_key),
            Some(0x87)
        );
    }
}
