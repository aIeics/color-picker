//! Colors the user saved on purpose, in the order they were added.

use crate::color::Rgb;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Palette {
    colors: Vec<Rgb>,
}

/// What [`Palette::add`] did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Added {
    Yes,
    AlreadyThere,
    Full,
}

impl Palette {
    pub const MAX: usize = 96;

    /// Builds a palette from saved colors, dropping duplicates and anything
    /// past [`Self::MAX`].
    pub fn new(colors: impl IntoIterator<Item = Rgb>) -> Self {
        let mut palette = Self::default();
        for color in colors {
            palette.add(color);
        }
        palette
    }

    pub fn colors(&self) -> &[Rgb] {
        &self.colors
    }

    pub fn contains(&self, color: Rgb) -> bool {
        self.colors.contains(&color)
    }

    pub fn add(&mut self, color: Rgb) -> Added {
        if self.contains(color) {
            Added::AlreadyThere
        } else if self.colors.len() >= Self::MAX {
            Added::Full
        } else {
            self.colors.push(color);
            Added::Yes
        }
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

    #[test]
    fn add_rejects_duplicates_and_overflow() {
        let mut palette = Palette::default();
        assert_eq!(palette.add(Rgb::new(1, 2, 3)), Added::Yes);
        assert_eq!(palette.add(Rgb::new(1, 2, 3)), Added::AlreadyThere);
        for v in 0..Palette::MAX as u8 {
            palette.add(Rgb::new(v, 0, 0));
        }
        assert_eq!(palette.colors().len(), Palette::MAX);
        assert_eq!(palette.add(Rgb::new(0, 0, 255)), Added::Full);
    }

    #[test]
    fn remove_keeps_order() {
        let mut palette = Palette::new([Rgb::new(1, 0, 0), Rgb::new(2, 0, 0), Rgb::new(3, 0, 0)]);
        palette.remove(1);
        palette.remove(9);
        assert_eq!(palette.colors(), [Rgb::new(1, 0, 0), Rgb::new(3, 0, 0)]);
    }
}
