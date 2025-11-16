// code from https://github.com/rust-embedded-community/pc-keyboard
// license in THIRD_PARTY_LICENSE

use crate::{
	drivers::keyboard::{
		layout::{KeyboardLayout, PhysicalKeyboard},
		scancode::KeyCode
	},
	io::keyboard::decode::{DecodedKey, HandleControl, Modifiers, QUO, SLS}
};

pub struct Us104Key;

impl KeyboardLayout for Us104Key {
	#[rustfmt::skip]
	fn map_keycode(
        &self,
        keycode: KeyCode,
        modifiers: &Modifiers,
        handle_ctrl: HandleControl,
    ) -> DecodedKey {
        match keycode {
            KeyCode::Oem8            => modifiers.handle_symbol2('`', '~'),
            KeyCode::Escape          => DecodedKey::Unicode('\u{001B}'),
            KeyCode::Key1            => modifiers.handle_symbol2('1', '!'),
            KeyCode::Key2            => modifiers.handle_symbol2('2', '@'),
            KeyCode::Key3            => modifiers.handle_symbol2('3', '#'),
            KeyCode::Key4            => modifiers.handle_symbol2('4', '$'),
            KeyCode::Key5            => modifiers.handle_symbol2('5', '%'),
            KeyCode::Key6            => modifiers.handle_symbol2('6', '^'),
            KeyCode::Key7            => modifiers.handle_symbol2('7', '&'),
            KeyCode::Key8            => modifiers.handle_symbol2('8', '*'),
            KeyCode::Key9            => modifiers.handle_symbol2('9', '('),
            KeyCode::Key0            => modifiers.handle_symbol2('0', ')'),
            KeyCode::OemMinus        => modifiers.handle_symbol2('-', '_'),
            KeyCode::OemPlus         => modifiers.handle_symbol2('=', '+'),
            KeyCode::Backspace       => DecodedKey::Unicode('\u{0008}'),

            KeyCode::Tab             => DecodedKey::Unicode('\u{0009}'),
            KeyCode::Q               => modifiers.handle_ascii_2('Q', handle_ctrl),
            KeyCode::W               => modifiers.handle_ascii_2('W', handle_ctrl),
            KeyCode::E               => modifiers.handle_ascii_2('E', handle_ctrl),
            KeyCode::R               => modifiers.handle_ascii_2('R', handle_ctrl),
            KeyCode::T               => modifiers.handle_ascii_2('T', handle_ctrl),
            KeyCode::Y               => modifiers.handle_ascii_2('Y', handle_ctrl),
            KeyCode::U               => modifiers.handle_ascii_2('U', handle_ctrl),
            KeyCode::I               => modifiers.handle_ascii_2('I', handle_ctrl),
            KeyCode::O               => modifiers.handle_ascii_2('O', handle_ctrl),
            KeyCode::P               => modifiers.handle_ascii_2('P', handle_ctrl),
            KeyCode::Oem4            => modifiers.handle_symbol2('[', '{'),
            KeyCode::Oem6            => modifiers.handle_symbol2(']', '}'),
            KeyCode::Oem7            => modifiers.handle_symbol2(SLS, '|'),

            KeyCode::A               => modifiers.handle_ascii_2('A', handle_ctrl),
            KeyCode::S               => modifiers.handle_ascii_2('S', handle_ctrl),
            KeyCode::D               => modifiers.handle_ascii_2('D', handle_ctrl),
            KeyCode::F               => modifiers.handle_ascii_2('F', handle_ctrl),
            KeyCode::G               => modifiers.handle_ascii_2('G', handle_ctrl),
            KeyCode::H               => modifiers.handle_ascii_2('H', handle_ctrl),
            KeyCode::J               => modifiers.handle_ascii_2('J', handle_ctrl),
            KeyCode::K               => modifiers.handle_ascii_2('K', handle_ctrl),
            KeyCode::L               => modifiers.handle_ascii_2('L', handle_ctrl),
            KeyCode::Oem1            => modifiers.handle_symbol2(';', ':'),
            KeyCode::Oem3            => modifiers.handle_symbol2(QUO, '"'),
            KeyCode::Return          => DecodedKey::Unicode('\u{000A}'),

            KeyCode::Z               => modifiers.handle_ascii_2('Z', handle_ctrl),
            KeyCode::X               => modifiers.handle_ascii_2('X', handle_ctrl),
            KeyCode::C               => modifiers.handle_ascii_2('C', handle_ctrl),
            KeyCode::V               => modifiers.handle_ascii_2('V', handle_ctrl),
            KeyCode::B               => modifiers.handle_ascii_2('B', handle_ctrl),
            KeyCode::N               => modifiers.handle_ascii_2('N', handle_ctrl),
            KeyCode::M               => modifiers.handle_ascii_2('M', handle_ctrl),
            KeyCode::OemComma        => modifiers.handle_symbol2(',', '<'),
            KeyCode::OemPeriod       => modifiers.handle_symbol2('.', '>'),
            KeyCode::Oem2            => modifiers.handle_symbol2('/', '?'),

            KeyCode::Spacebar        => DecodedKey::Unicode(' '),
            KeyCode::Delete          => DecodedKey::Unicode('\u{007f}'),

            KeyCode::NumpadDivide    => DecodedKey::Unicode('/'),
            KeyCode::NumpadMultiply  => DecodedKey::Unicode('*'),
            KeyCode::NumpadSubtract  => DecodedKey::Unicode('-'),
            KeyCode::Numpad7         => modifiers.handle_num_pad('7', KeyCode::Home),
            KeyCode::Numpad8         => modifiers.handle_num_pad('8', KeyCode::ArrowUp),
            KeyCode::Numpad9         => modifiers.handle_num_pad('9', KeyCode::PageUp),
            KeyCode::NumpadAdd       => DecodedKey::Unicode('+'),
            KeyCode::Numpad4         => modifiers.handle_num_pad('4', KeyCode::ArrowLeft),
            KeyCode::Numpad5         => DecodedKey::Unicode('5'),
            KeyCode::Numpad6         => modifiers.handle_num_pad('6', KeyCode::ArrowRight),
            KeyCode::Numpad1         => modifiers.handle_num_pad('1', KeyCode::End),
            KeyCode::Numpad2         => modifiers.handle_num_pad('2', KeyCode::ArrowDown),
            KeyCode::Numpad3         => modifiers.handle_num_pad('3', KeyCode::PageDown),
            KeyCode::Numpad0         => modifiers.handle_num_pad('0', KeyCode::Insert),
            KeyCode::NumpadPeriod    => modifiers.handle_num_del('.', '\u{007f}'),
            KeyCode::NumpadEnter     => DecodedKey::Unicode('\u{000A}'),
            // fall back
            k                        => DecodedKey::RawKey(k),
        }
    }

	fn get_physical(&self) -> PhysicalKeyboard {
		PhysicalKeyboard::Ansi
	}
}
