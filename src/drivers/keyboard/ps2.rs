// code from https://github.com/rust-embedded-community/pc-keyboard
// license in THIRD_PARTY_LICENSE

use crate::{drivers::keyboard::{error::KeyboardError, layout::KeyboardLayout, scancode::{KeyCode, ScancodeSet}}, io::keyboard::decode::{DecodedKey, HandleControl, KEYCODE_BITS, KeyEvent, KeyState, Modifiers}};

#[derive(Debug)]
pub struct Ps2Decoder {
	pub reg: u16,
	pub num_bits: u8,
}

impl Ps2Decoder {
	pub const fn new() -> Ps2Decoder {
		Ps2Decoder { 
			reg: 0, 
			num_bits: 0
		}
	}

	pub fn clear(&mut self) {
		self.reg = 0;
		self.num_bits = 0;
	}

    pub fn add_bit(&mut self, bit: bool) -> Result<Option<u8>, KeyboardError> {
        self.reg |= (bit as u16) << self.num_bits;
        self.num_bits += 1;
        if self.num_bits == KEYCODE_BITS {
            let word = self.reg;
            self.reg = 0;
            self.num_bits = 0;
            let byte = Self::check_word(word)?;
            Ok(Some(byte))
        } else {
            Ok(None)
        }
    }

    pub fn add_word(&self, word: u16) -> Result<u8, KeyboardError> {
        Self::check_word(word)
    }

    const fn check_word(word: u16) -> Result<u8, KeyboardError> {
        let start_bit = Self::get_bit(word, 0);
        let parity_bit = Self::get_bit(word, 9);
        let stop_bit = Self::get_bit(word, 10);
        let data = ((word >> 1) & 0xFF) as u8;

        if start_bit {
            return Err(KeyboardError::BadStartBit);
        }

        if !stop_bit {
            return Err(KeyboardError::BadStopBit);
        }

        let need_parity = Self::has_even_number_bits(data);

        if need_parity != parity_bit {
            return Err(KeyboardError::ParityError);
        }

        Ok(data)
    }

    const fn get_bit(word: u16, offset: usize) -> bool {
        ((word >> offset) & 0x0001) != 0
    }

    const fn has_even_number_bits(data: u8) -> bool {
        (data.count_ones() % 2) == 0
    }
}

#[derive(Debug)]
pub struct EventDecoder<L> 
where 
	L: KeyboardLayout, 
{
	handle_ctrl: HandleControl,
	modifiers: Modifiers,
	layout: L
}

impl<L> EventDecoder<L> 
where 
	L: KeyboardLayout,
{
	pub const fn new(layout: L, handle_ctrl: HandleControl) -> EventDecoder<L> {
        EventDecoder {
            handle_ctrl,
            modifiers: Modifiers {
                lshift: false,
                rshift: false,
                lctrl: false,
                rctrl: false,
                numlock: true,
                capslock: false,
                lalt: false,
                ralt: false,
                rctrl2: false,
            },
            layout,
        }
    }

	pub fn process_keyevent(&mut self, ev: KeyEvent) -> Option<DecodedKey> {
        match ev {
            KeyEvent {
                code: KeyCode::LShift,
                state: KeyState::Down,
            } => {
                self.modifiers.lshift = true;
                Some(DecodedKey::RawKey(KeyCode::LShift))
            }
            KeyEvent {
                code: KeyCode::RShift,
                state: KeyState::Down,
            } => {
                self.modifiers.rshift = true;
                Some(DecodedKey::RawKey(KeyCode::RShift))
            }
            KeyEvent {
                code: KeyCode::LShift,
                state: KeyState::Up,
            } => {
                self.modifiers.lshift = false;
                None
            }
            KeyEvent {
                code: KeyCode::RShift,
                state: KeyState::Up,
            } => {
                self.modifiers.rshift = false;
                None
            }
            KeyEvent {
                code: KeyCode::CapsLock,
                state: KeyState::Down,
            } => {
                self.modifiers.capslock = !self.modifiers.capslock;
                Some(DecodedKey::RawKey(KeyCode::CapsLock))
            }
            KeyEvent {
                code: KeyCode::NumpadLock,
                state: KeyState::Down,
            } => {
                if self.modifiers.rctrl2 {
                    // It's a Pause key because we got the 'hidden' rctrl2
                    // sequence first.
                    Some(DecodedKey::RawKey(KeyCode::PauseBreak))
                } else {
                    // It's a numlock toggle
                    self.modifiers.numlock = !self.modifiers.numlock;
                    Some(DecodedKey::RawKey(KeyCode::NumpadLock))
                }
            }
            KeyEvent {
                code: KeyCode::LControl,
                state: KeyState::Down,
            } => {
                self.modifiers.lctrl = true;
                Some(DecodedKey::RawKey(KeyCode::LControl))
            }
            KeyEvent {
                code: KeyCode::LControl,
                state: KeyState::Up,
            } => {
                self.modifiers.lctrl = false;
                None
            }
            KeyEvent {
                code: KeyCode::RControl,
                state: KeyState::Down,
            } => {
                self.modifiers.rctrl = true;
                Some(DecodedKey::RawKey(KeyCode::RControl))
            }
            KeyEvent {
                code: KeyCode::RControl,
                state: KeyState::Up,
            } => {
                self.modifiers.rctrl = false;
                None
            }
            KeyEvent {
                code: KeyCode::LAlt,
                state: KeyState::Down,
            } => {
                self.modifiers.lalt = true;
                Some(DecodedKey::RawKey(KeyCode::LAlt))
            }
            KeyEvent {
                code: KeyCode::LAlt,
                state: KeyState::Up,
            } => {
                self.modifiers.lalt = false;
                None
            }
            KeyEvent {
                code: KeyCode::RAltGr,
                state: KeyState::Down,
            } => {
                self.modifiers.ralt = true;
                Some(DecodedKey::RawKey(KeyCode::RAltGr))
            }
            KeyEvent {
                code: KeyCode::RAltGr,
                state: KeyState::Up,
            } => {
                self.modifiers.ralt = false;
                None
            }
            KeyEvent {
                code: KeyCode::RControl2,
                state: KeyState::Down,
            } => {
                self.modifiers.rctrl2 = true;
                Some(DecodedKey::RawKey(KeyCode::RControl2))
            }
            KeyEvent {
                code: KeyCode::RControl2,
                state: KeyState::Up,
            } => {
                self.modifiers.rctrl2 = false;
                None
            }
            KeyEvent {
                code: c,
                state: KeyState::Down,
            } => Some(
                self.layout
                    .map_keycode(c, &self.modifiers, self.handle_ctrl),
            ),
            _ => None,
        }
	}

	pub fn set_ctrl_handling(&mut self, new_value: HandleControl) {
        self.handle_ctrl = new_value;
    }

    pub const fn get_ctrl_handling(&self) -> HandleControl {
        self.handle_ctrl
    }
}

pub struct Keyboard<L, S> 
where 
	S: ScancodeSet,
	L: KeyboardLayout,
{
	ps2_decoder: Ps2Decoder,
	scancode_set: S,
	event_decoder: EventDecoder<L>
}

impl<L, S> Keyboard<L, S>
where 
	L: KeyboardLayout,
	S: ScancodeSet,
{
	pub const fn new(scancode_set: S, layout: L, handle_ctrl: HandleControl) -> Keyboard<L, S> {
        Keyboard {
            ps2_decoder: Ps2Decoder::new(),
            scancode_set,
            event_decoder: EventDecoder::new(layout, handle_ctrl),
        }
    }

    pub const fn get_modifiers(&self) -> &Modifiers {
        &self.event_decoder.modifiers
    }

    pub fn set_ctrl_handling(&mut self, new_value: HandleControl) {
        self.event_decoder.set_ctrl_handling(new_value);
    }

    pub const fn get_ctrl_handling(&self) -> HandleControl {
        self.event_decoder.get_ctrl_handling()
    }

    pub fn clear(&mut self) {
        self.ps2_decoder.clear();
    }

    pub fn add_word(&mut self, word: u16) -> Result<Option<KeyEvent>, KeyboardError> {
        let byte = self.ps2_decoder.add_word(word)?;
        self.add_byte(byte)
    }

    pub fn add_byte(&mut self, byte: u8) -> Result<Option<KeyEvent>, KeyboardError> {
        self.scancode_set.advance_state(byte)
    }

    pub fn add_bit(&mut self, bit: bool) -> Result<Option<KeyEvent>, KeyboardError> {
        if let Some(byte) = self.ps2_decoder.add_bit(bit)? {
            self.scancode_set.advance_state(byte)
        } else {
            Ok(None)
        }
    }

    pub fn process_keyevent(&mut self, ev: KeyEvent) -> Option<DecodedKey> {
        self.event_decoder.process_keyevent(ev)
    }
}