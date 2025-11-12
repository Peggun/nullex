// code from https://github.com/rust-embedded-community/pc-keyboard
// license in THIRD_PARTY_LICENSE


use crate::drivers::keyboard::scancode::KeyCode;

pub const KEYCODE_BITS: u8 = 11;
pub const EXTENDED_KEY_CODE: u8 = 0xE0;
pub const EXTENDED2_KEY_CODE: u8 = 0xE1;
pub const KEY_RELEASE_CODE: u8 = 0xF0;

pub const QUO: char = '\'';
pub const SLS: char = '\\';

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum HandleControl {
	Ignore,
    MapLettersToUnicode
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct Modifiers {
    pub lshift: bool,
    pub rshift: bool,
    pub lctrl: bool,
    pub rctrl: bool,
    pub numlock: bool,
    pub capslock: bool,
    pub lalt: bool,
    pub ralt: bool,
    pub rctrl2: bool,
}

impl Modifiers {
    pub const fn is_shifted(&self) -> bool {
        self.lshift | self.rshift
    }

    pub const fn is_ctrl(&self) -> bool {
        self.lctrl | self.rctrl
    }

    pub const fn is_alt(&self) -> bool {
        self.lalt | self.ralt
    }

    pub const fn is_altgr(&self) -> bool {
        self.ralt | (self.lalt & self.is_ctrl())
    }

    pub const fn is_caps(&self) -> bool {
        self.is_shifted() ^ self.capslock
    }

    pub(crate) fn handle_ascii_2(&self, letter: char, handle_ctrl: HandleControl) -> DecodedKey {
        debug_assert!(letter.is_ascii_uppercase());
        if handle_ctrl == HandleControl::MapLettersToUnicode && self.is_ctrl() {
            // Get a Control code, like Ctrl+C => U+0003
            const ASCII_UPPERCASE_START_OFFSET: u8 = 64;
            DecodedKey::Unicode((letter as u8 - ASCII_UPPERCASE_START_OFFSET) as char)
        } else if self.is_caps() {
            // Capital letter
            DecodedKey::Unicode(letter)
        } else {
            // Lowercase letter
            const ASCII_UPPER_TO_LOWER_OFFSET: u8 = 32;
            DecodedKey::Unicode((letter as u8 + ASCII_UPPER_TO_LOWER_OFFSET) as char)
        }
    }

    pub(crate) fn handle_letter2(&self, letter_lower: char, letter_upper: char) -> DecodedKey {
        if self.is_caps() {
            DecodedKey::Unicode(letter_upper)
        } else {
            DecodedKey::Unicode(letter_lower)
        }
    }

    pub(crate) fn handle_ascii_3(
        &self,
        letter_upper: char,
        alt: char,
        handle_ctrl: HandleControl,
    ) -> DecodedKey {
        debug_assert!(letter_upper.is_ascii_uppercase());
        if handle_ctrl == HandleControl::MapLettersToUnicode && self.is_ctrl() {
            // Get a Control code, like Ctrl+C => U+0003
            const ASCII_UPPERCASE_START_OFFSET: u8 = 64;
            DecodedKey::Unicode((letter_upper as u8 - ASCII_UPPERCASE_START_OFFSET) as char)
        } else if self.ralt {
            // Alternate character
            DecodedKey::Unicode(alt)
        } else if self.is_caps() {
            // Capital letter
            DecodedKey::Unicode(letter_upper)
        } else {
            // Lowercase letter
            const ASCII_UPPER_TO_LOWER_OFFSET: u8 = 32;
            DecodedKey::Unicode((letter_upper as u8 + ASCII_UPPER_TO_LOWER_OFFSET) as char)
        }
    }

    pub(crate) fn handle_ascii_4(
        &self,
        letter_upper: char,
        alt_letter_lower: char,
        alt_letter_upper: char,
        handle_ctrl: HandleControl,
    ) -> DecodedKey {
        debug_assert!(letter_upper.is_ascii_uppercase());
        if handle_ctrl == HandleControl::MapLettersToUnicode && self.is_ctrl() {
            const ASCII_UPPERCASE_START_OFFSET: u8 = 64;
            DecodedKey::Unicode((letter_upper as u8 - ASCII_UPPERCASE_START_OFFSET) as char)
        } else if self.ralt && self.is_caps() {
            DecodedKey::Unicode(alt_letter_upper)
        } else if self.ralt {
            DecodedKey::Unicode(alt_letter_lower)
        } else if self.is_caps() {
            DecodedKey::Unicode(letter_upper)
        } else {
            const ASCII_UPPER_TO_LOWER_OFFSET: u8 = 32;
            DecodedKey::Unicode((letter_upper as u8 + ASCII_UPPER_TO_LOWER_OFFSET) as char)
        }
    }

    pub(crate) fn handle_num_pad(&self, letter: char, key: KeyCode) -> DecodedKey {
        if self.numlock {
            DecodedKey::Unicode(letter)
        } else {
            DecodedKey::RawKey(key)
        }
    }

    pub(crate) fn handle_num_del(&self, letter: char, other: char) -> DecodedKey {
        if self.numlock {
            DecodedKey::Unicode(letter)
        } else {
            DecodedKey::Unicode(other)
        }
    }

    pub(crate) fn handle_symbol2(&self, plain: char, shifted: char) -> DecodedKey {
        if self.is_shifted() {
            DecodedKey::Unicode(shifted)
        } else {
            DecodedKey::Unicode(plain)
        }
    }

    pub(crate) fn handle_symbol3(&self, plain: char, shifted: char, alt: char) -> DecodedKey {
        if self.is_altgr() {
            DecodedKey::Unicode(alt)
        } else if self.is_shifted() {
            DecodedKey::Unicode(shifted)
        } else {
            DecodedKey::Unicode(plain)
        }
    }
}

pub enum DecodedKey {
	RawKey(KeyCode),
	Unicode(char),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KeyState {
	Up, 
	Down,
	OneShot
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeyEvent {
	pub code: KeyCode,
	pub state: KeyState
}

impl KeyEvent {
    pub const fn new(code: KeyCode, state: KeyState) -> KeyEvent {
        KeyEvent { code, state }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum DecodeState {
    Start,
    Extended,
    Release,
    ExtendedRelease,
    Extended2,
    Extended2Release,
}