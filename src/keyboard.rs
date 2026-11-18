//! Keys and modifiers, as used by [`KeyBinding`](crate::KeyBinding).
//!
//! Each editor converts its framework's key events to these types.

use std::ops::{BitOr, BitOrAssign};

/// A key that is not a character.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Named {
    Enter,
    Tab,
    Space,
    Backspace,
    Delete,
    Insert,
    Escape,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
    Home,
    End,
    PageUp,
    PageDown,
    Copy,
    Cut,
    Paste,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
}

/// A logical key: named, or the character(s) it produces.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Key<C = String> {
    Named(Named),
    Character(C),
    Unidentified,
}

impl Key {
    /// A borrowed view, convenient for `match`.
    pub fn as_ref(&self) -> Key<&str> {
        match self {
            Key::Named(named) => Key::Named(*named),
            Key::Character(c) => Key::Character(c.as_str()),
            Key::Unidentified => Key::Unidentified,
        }
    }
}

/// Modifier keys held during a key press or click.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Modifiers(u8);

impl Modifiers {
    pub const SHIFT: Modifiers = Modifiers(1);
    pub const CTRL: Modifiers = Modifiers(2);
    pub const ALT: Modifiers = Modifiers(4);
    /// Windows key / Command key / Super key.
    pub const LOGO: Modifiers = Modifiers(8);
    /// `Ctrl` on Windows and Linux, `Cmd` on macOS.
    pub const COMMAND: Modifiers = if cfg!(target_os = "macos") {
        Modifiers::LOGO
    } else {
        Modifiers::CTRL
    };

    pub const fn empty() -> Modifiers {
        Modifiers(0)
    }

    pub const fn contains(self, other: Modifiers) -> bool {
        self.0 & other.0 == other.0
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub fn shift(self) -> bool {
        self.contains(Modifiers::SHIFT)
    }

    pub fn control(self) -> bool {
        self.contains(Modifiers::CTRL)
    }

    pub fn alt(self) -> bool {
        self.contains(Modifiers::ALT)
    }

    pub fn logo(self) -> bool {
        self.contains(Modifiers::LOGO)
    }

    /// `Ctrl` on Windows and Linux, `Cmd` on macOS.
    pub fn command(self) -> bool {
        self.contains(Modifiers::COMMAND)
    }
}

impl BitOr for Modifiers {
    type Output = Modifiers;

    fn bitor(self, other: Modifiers) -> Modifiers {
        Modifiers(self.0 | other.0)
    }
}

impl BitOrAssign for Modifiers {
    fn bitor_assign(&mut self, other: Modifiers) {
        self.0 |= other.0;
    }
}

/// A wheel movement: lines (mouse wheel) or pixels (touchpad).
/// Positive `y` scrolls up / zooms in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScrollDelta {
    Lines { x: f32, y: f32 },
    Pixels { x: f32, y: f32 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modifier_flags() {
        let both = Modifiers::COMMAND | Modifiers::SHIFT;
        assert!(both.command());
        assert!(both.shift());
        assert!(!both.alt());
        assert!(Modifiers::empty().is_empty());
    }
}
