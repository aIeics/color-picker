//! Recently picked colors, newest first.

pub mod ui;

use crate::color::Rgb;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct History {
    colors: Vec<Rgb>,
}

impl History {
    /// Older colors drop off the end.
    pub const MAX: usize = 48;

    /// Builds a history from saved colors (newest first), dropping
    /// duplicates and anything past [`Self::MAX`].
    pub fn new(colors: impl IntoIterator<Item = Rgb>) -> Self {
        let mut history = Self::default();
        for color in colors {
            if !history.colors.contains(&color) {
                history.colors.push(color);
            }
        }
        history.colors.truncate(Self::MAX);
        history
    }

    pub fn colors(&self) -> &[Rgb] {
        &self.colors
    }

    /// Adds `color` as the newest entry. A color already in the history
    /// moves to the front instead of appearing twice.
    pub fn push(&mut self, color: Rgb) {
        self.colors.retain(|&c| c != color);
        self.colors.insert(0, color);
        self.colors.truncate(Self::MAX);
    }

    pub fn remove(&mut self, index: usize) {
        if index < self.colors.len() {
            self.colors.remove(index);
        }
    }

    pub fn clear(&mut self) {
        self.colors.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gray(v: u8) -> Rgb {
        Rgb::new(v, v, v)
    }

    #[test]
    fn push_moves_duplicates_to_front() {
        let mut history = History::default();
        history.push(gray(1));
        history.push(gray(2));
        history.push(gray(1));
        assert_eq!(history.colors(), [gray(1), gray(2)]);
    }

    #[test]
    fn keeps_at_most_max() {
        let mut history = History::default();
        for v in 0..=u8::MAX {
            history.push(gray(v));
        }
        assert_eq!(history.colors().len(), History::MAX);
        assert_eq!(history.colors()[0], gray(255));
    }

    #[test]
    fn new_drops_duplicates_and_extra() {
        let history = History::new((0..100).map(|v| gray(v % 60)));
        assert_eq!(history.colors().len(), History::MAX);
        assert_eq!(history.colors()[..2], [gray(0), gray(1)]);
    }

    #[test]
    fn remove_ignores_bad_index() {
        let mut history = History::new([gray(1), gray(2)]);
        history.remove(5);
        history.remove(0);
        assert_eq!(history.colors(), [gray(2)]);
        history.clear();
        assert!(history.colors().is_empty());
    }
}
