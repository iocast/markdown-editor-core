//! Font metrics.
//!
//! The core measures text but ships no fonts: each editor passes the font
//! files it draws with to [`Fonts::new`]. Measuring with the same files keeps
//! the caret, selection and mouse hit-testing exact.

use ab_glyph::{Font as _, FontRef};

/// A concrete font face. The discriminant is the index into the array given
/// to [`Fonts::new`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Face {
    #[default]
    Sans = 0,
    SansBold = 1,
    SansItalic = 2,
    SansBoldItalic = 3,
    Mono = 4,
    MonoBold = 5,
}

impl Face {
    pub fn select(bold: bool, italic: bool, mono: bool) -> Face {
        match (mono, bold, italic) {
            (true, true, _) => Face::MonoBold,
            (true, false, _) => Face::Mono,
            (false, true, true) => Face::SansBoldItalic,
            (false, true, false) => Face::SansBold,
            (false, false, true) => Face::SansItalic,
            (false, false, false) => Face::Sans,
        }
    }
}

/// Glyph metrics of the six faces of an editor.
#[derive(Debug, Clone)]
pub struct Fonts {
    faces: Vec<FontRef<'static>>,
    units_per_em: Vec<f32>,
}

impl Fonts {
    /// Parses the font files, in the order of [`Face`].
    ///
    /// # Panics
    ///
    /// If a file is not a valid font. Editors pass fonts bundled at compile
    /// time, so this is a programming error.
    pub fn new(files: [&'static [u8]; 6]) -> Self {
        let mut faces = Vec::with_capacity(files.len());
        let mut units_per_em = Vec::with_capacity(files.len());

        for bytes in files {
            let face = FontRef::try_from_slice(bytes).expect("bundled font must be valid");
            units_per_em.push(face.units_per_em().unwrap_or(2048.0));
            faces.push(face);
        }

        Self {
            faces,
            units_per_em,
        }
    }

    /// Horizontal advance of `c` in logical pixels.
    pub fn advance(&self, face: Face, c: char, size: f32) -> f32 {
        let index = face as usize;
        let font = &self.faces[index];
        let glyph = font.glyph_id(c);
        font.h_advance_unscaled(glyph) * size / self.units_per_em[index]
    }

    /// Width of a whole string in logical pixels.
    pub fn width(&self, face: Face, text: &str, size: f32) -> f32 {
        text.chars().map(|c| self.advance(face, c, size)).sum()
    }

    /// Glyph id of `c` in `face`.
    pub fn glyph_id(&self, face: Face, c: char) -> u32 {
        u32::from(self.faces[face as usize].glyph_id(c).0)
    }

    /// Distance from the top of a line box of `line_height` to the baseline
    /// for text of `size`: the glyphs' ascent-descent box is centred in the
    /// line.
    pub fn baseline_offset(&self, face: Face, size: f32, line_height: f32) -> f32 {
        let index = face as usize;
        let font = &self.faces[index];
        let scale = size / self.units_per_em[index];
        let ascent = font.ascent_unscaled() * scale;
        let descent = font.descent_unscaled() * scale; // negative
        (line_height - (ascent - descent)) / 2.0 + ascent
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baseline_is_inside_the_line() {
        let offset = crate::test_fonts().baseline_offset(Face::Sans, 14.0, 22.0);
        assert!(offset > 11.0 && offset < 22.0, "{offset}");
    }

    #[test]
    fn width_is_the_sum_of_the_advances() {
        let fonts = crate::test_fonts();
        let sum = fonts.advance(Face::Mono, 'a', 10.0) + fonts.advance(Face::Mono, 'b', 10.0);
        assert!((fonts.width(Face::Mono, "ab", 10.0) - sum).abs() < 1e-4);
    }
}
