// code from https://github.com/rust-embedded-community/pc-keyboard
// license in THIRD_PARTY_LICENSE


use crate::{drivers::keyboard::scancode::KeyCode, io::keyboard::decode::{DecodedKey, HandleControl, Modifiers}};

pub enum PhysicalKeyboard {
    Iso,
    Ansi,
    Jis,
}

pub trait KeyboardLayout {
	fn map_keycode(&self, keycode: KeyCode, modifiers: &Modifiers, handle_ctrl: HandleControl) -> DecodedKey;
    fn get_physical(&self) -> PhysicalKeyboard;
}