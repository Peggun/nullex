// code from https://github.com/rust-embedded-community/pc-keyboard
// license in THIRD_PARTY_LICENSE


use crate::{drivers::keyboard::error::KeyboardError, io::keyboard::decode::{DecodeState, EXTENDED_KEY_CODE, KeyEvent, KeyState, EXTENDED2_KEY_CODE}};
use alloc::string::{String, ToString};
use lazy_static::lazy_static;

use crate::utils::mutex::SpinMutex;

lazy_static! {
	pub static ref CWD: SpinMutex<String> = SpinMutex::new("/".to_string());
}

// https://kbdlayout.info/kbduk
#[derive(Debug, PartialEq, Eq, Copy, Clone, PartialOrd, Ord)]
#[repr(u8)]
pub enum KeyCode {
	Escape,
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

	PrintScreen,
	SysRq,
	ScrollLock,
	PauseBreak,

	Oem8,
	Key1,
	Key2,
	Key3,
	Key4,
	Key5,
	Key6,
	Key7,
	Key8,
	Key9,
	Key0,
	OemMinus,
	OemPlus,
	Backspace,

	Insert,
	Home,
	PageUp,

	NumpadLock,
	NumpadDivide,
	NumpadMultiply,
	NumpadSubtract,

	Tab,
	Q,
	W,
	E,
	R,
	T,
	Y,
	U,
	I,
	O,
	P,
	Oem4,
	Oem6,
	Oem5,
	Oem7,

	Delete,
	End,
	PageDown,

	Numpad7,
	Numpad8,
	Numpad9,
	NumpadAdd,

	CapsLock,
	A,
	S,
	D,
	F,
	G,
	H,
	J,
	K,
	L,
	Oem1,
	Oem3,

	Return,

	Numpad4,
	Numpad5,
	Numpad6,

	LShift,
	Z,
	X,
	C,
	V,
	B,
	N,
	M,
	OemComma,
	OemPeriod,
	Oem2,
	RShift,

	ArrowUp,

	Numpad1,
	Numpad2,
	Numpad3,
	NumpadEnter,

	LControl,
	LWin,
	LAlt,
	Spacebar,
	RAltGr,
	RWin,
	Apps,
	RControl,

	ArrowLeft,
	ArrowDown,
	ArrowRight,

	Numpad0,
	NumpadPeriod,

	Oem9,
	Oem10,
	Oem11,
	Oem12,
	Oem13,

	PrevTrack,
	NextTrack,
	Mute,
	Calculator,
	Play,
	Stop,
	VolumeDown,
	VolumeUp,
	WWWHome,
	PowerOnTestOk,
	TooManyKeys,
	RControl2,
	RAlt2,

	Unknown,
}

pub trait ScancodeSet {
    fn advance_state(&mut self, code: u8) -> Result<Option<KeyEvent>, KeyboardError>;
}
pub struct ScancodeSet1 {
    state: DecodeState,
}

impl ScancodeSet1 {
    pub const fn new() -> ScancodeSet1 {
        ScancodeSet1 {
            state: DecodeState::Start,
        }
    }

    fn map_scancode(code: u8) -> Result<KeyCode, KeyboardError> {
        match code {
            0x01 => Ok(KeyCode::Escape),
            0x02 => Ok(KeyCode::Key1),
            0x03 => Ok(KeyCode::Key2),
            0x04 => Ok(KeyCode::Key3),
            0x05 => Ok(KeyCode::Key4),
            0x06 => Ok(KeyCode::Key5),
            0x07 => Ok(KeyCode::Key6),
            0x08 => Ok(KeyCode::Key7),
            0x09 => Ok(KeyCode::Key8),
            0x0A => Ok(KeyCode::Key9),
            0x0B => Ok(KeyCode::Key0),
            0x0C => Ok(KeyCode::OemMinus),
            0x0D => Ok(KeyCode::OemPlus),
            0x0E => Ok(KeyCode::Backspace),
            0x0F => Ok(KeyCode::Tab),
            0x10 => Ok(KeyCode::Q),
            0x11 => Ok(KeyCode::W),
            0x12 => Ok(KeyCode::E),
            0x13 => Ok(KeyCode::R),
            0x14 => Ok(KeyCode::T),
            0x15 => Ok(KeyCode::Y),
            0x16 => Ok(KeyCode::U),
            0x17 => Ok(KeyCode::I),
            0x18 => Ok(KeyCode::O),
            0x19 => Ok(KeyCode::P),
            0x1A => Ok(KeyCode::Oem4),
            0x1B => Ok(KeyCode::Oem6),
            0x1C => Ok(KeyCode::Return),
            0x1D => Ok(KeyCode::LControl),
            0x1E => Ok(KeyCode::A),
            0x1F => Ok(KeyCode::S),
            0x20 => Ok(KeyCode::D),
            0x21 => Ok(KeyCode::F),
            0x22 => Ok(KeyCode::G),
            0x23 => Ok(KeyCode::H),
            0x24 => Ok(KeyCode::J),
            0x25 => Ok(KeyCode::K),
            0x26 => Ok(KeyCode::L),
            0x27 => Ok(KeyCode::Oem1),
            0x28 => Ok(KeyCode::Oem3),
            0x29 => Ok(KeyCode::Oem8),
            0x2A => Ok(KeyCode::LShift),
            0x2B => Ok(KeyCode::Oem7),
            0x2C => Ok(KeyCode::Z),
            0x2D => Ok(KeyCode::X),
            0x2E => Ok(KeyCode::C),
            0x2F => Ok(KeyCode::V),
            0x30 => Ok(KeyCode::B),
            0x31 => Ok(KeyCode::N),
            0x32 => Ok(KeyCode::M),
            0x33 => Ok(KeyCode::OemComma),
            0x34 => Ok(KeyCode::OemPeriod),
            0x35 => Ok(KeyCode::Oem2),
            0x36 => Ok(KeyCode::RShift),
            0x37 => Ok(KeyCode::NumpadMultiply),
            0x38 => Ok(KeyCode::LAlt),
            0x39 => Ok(KeyCode::Spacebar),
            0x3A => Ok(KeyCode::CapsLock),
            0x3B => Ok(KeyCode::F1),
            0x3C => Ok(KeyCode::F2),
            0x3D => Ok(KeyCode::F3),
            0x3E => Ok(KeyCode::F4),
            0x3F => Ok(KeyCode::F5),
            0x40 => Ok(KeyCode::F6),
            0x41 => Ok(KeyCode::F7),
            0x42 => Ok(KeyCode::F8),
            0x43 => Ok(KeyCode::F9),
            0x44 => Ok(KeyCode::F10),
            0x45 => Ok(KeyCode::NumpadLock),
            0x46 => Ok(KeyCode::ScrollLock),
            0x47 => Ok(KeyCode::Numpad7),
            0x48 => Ok(KeyCode::Numpad8),
            0x49 => Ok(KeyCode::Numpad9),
            0x4A => Ok(KeyCode::NumpadSubtract),
            0x4B => Ok(KeyCode::Numpad4),
            0x4C => Ok(KeyCode::Numpad5),
            0x4D => Ok(KeyCode::Numpad6),
            0x4E => Ok(KeyCode::NumpadAdd),
            0x4F => Ok(KeyCode::Numpad1),
            0x50 => Ok(KeyCode::Numpad2),
            0x51 => Ok(KeyCode::Numpad3),
            0x52 => Ok(KeyCode::Numpad0),
            0x53 => Ok(KeyCode::NumpadPeriod),
            0x54 => Ok(KeyCode::SysRq),
            // 0x55 is unused?
            0x56 => Ok(KeyCode::Oem5),
            0x57 => Ok(KeyCode::F11),
            0x58 => Ok(KeyCode::F12),
            _ => Err(KeyboardError::UnknownKeyCode),
        }
    }

    fn map_extended_scancode(code: u8) -> Result<KeyCode, KeyboardError> {
        match code {
            0x10 => Ok(KeyCode::PrevTrack),
            //0x11
            //0x12
            //0x13
            //0x14
            //0x15
            //0x16
            //0x17
            //0x18
            0x19 => Ok(KeyCode::NextTrack),
            //0x1A
            //0x1B
            0x1C => Ok(KeyCode::NumpadEnter),
            0x1D => Ok(KeyCode::RControl),
            //0x1E
            //0x1F
            0x20 => Ok(KeyCode::Mute),
            0x21 => Ok(KeyCode::Calculator),
            0x22 => Ok(KeyCode::Play),
            //0x23
            0x24 => Ok(KeyCode::Stop),
            //0x25
            //0x26
            //0x27
            //0x28
            //0x29
            0x2A => Ok(KeyCode::RAlt2),
            //0x2B
            //0x2C
            //0x2D
            0x2E => Ok(KeyCode::VolumeDown),
            //0x2F
            0x30 => Ok(KeyCode::VolumeUp),
            //0x31
            0x32 => Ok(KeyCode::WWWHome),
            //0x33
            //0x34
            0x35 => Ok(KeyCode::NumpadDivide),
            //0x36
            0x37 => Ok(KeyCode::PrintScreen),
            0x38 => Ok(KeyCode::RAltGr),
            //0x39
            //0x3A
            //0x3B
            //0x3C
            //0x3D
            //0x3E
            //0x3F
            //0x40
            //0x41
            //0x42
            //0x43
            //0x44
            //0x45
            //0x46
            0x47 => Ok(KeyCode::Home),
            0x48 => Ok(KeyCode::ArrowUp),
            0x49 => Ok(KeyCode::PageUp),
            //0x4A
            0x4B => Ok(KeyCode::ArrowLeft),
            //0x4C
            0x4D => Ok(KeyCode::ArrowRight),
            //0x4E
            0x4F => Ok(KeyCode::End),
            0x50 => Ok(KeyCode::ArrowDown),
            0x51 => Ok(KeyCode::PageDown),
            0x52 => Ok(KeyCode::Insert),
            0x53 => Ok(KeyCode::Delete),
            0x5B => Ok(KeyCode::LWin),
            0x5C => Ok(KeyCode::RWin),
            0x5D => Ok(KeyCode::Apps),
            // 0x5E ACPI Power
            // 0x5F ACPI Sleep
            // 0x60
            // 0x61
            // 0x62
            // 0x63 ACPI Wake
            // 0x64
            // 0x65 WWW Search
            // 0x66 WWW Favourites
            // 0x67 WWW Refresh
            // 0x68 WWW Stop
            // 0x69 WWW Forward
            // 0x6A WWW Back
            // 0x6B My Computer
            // 0x6C Email
            // 0x6D Media Select
            0x70 => Ok(KeyCode::Oem11),
            0x73 => Ok(KeyCode::Oem12),
            0x79 => Ok(KeyCode::Oem10),
            0x7B => Ok(KeyCode::Oem9),
            0x7D => Ok(KeyCode::Oem13),
            _ => Err(KeyboardError::UnknownKeyCode),
        }
    }

    /// Implements the extended byte codes for set 1 (prefixed with E1)
    fn map_extended2_scancode(code: u8) -> Result<KeyCode, KeyboardError> {
        match code {
            0x1D => Ok(KeyCode::RControl2),
            _ => Err(KeyboardError::UnknownKeyCode),
        }
    }
}

impl ScancodeSet for ScancodeSet1 {
    fn advance_state(&mut self, code: u8) -> Result<Option<KeyEvent>, KeyboardError> {
        match self.state {
            DecodeState::Start => {
                match code {
                    EXTENDED_KEY_CODE => {
                        self.state = DecodeState::Extended;
                        Ok(None)
                    }
                    EXTENDED2_KEY_CODE => {
                        self.state = DecodeState::Extended2;
                        Ok(None)
                    }
                    0x80..=0xFF => {
                        Ok(Some(KeyEvent::new(
                            Self::map_scancode(code - 0x80)?,
                            KeyState::Up,
                        )))
                    }
                    _ => {
                        Ok(Some(KeyEvent::new(
                            Self::map_scancode(code)?,
                            KeyState::Down,
                        )))
                    }
                }
            }
            DecodeState::Extended => {
                self.state = DecodeState::Start;
                match code {
                    0x80..=0xFF => {
                        Ok(Some(KeyEvent::new(
                            Self::map_extended_scancode(code - 0x80)?,
                            KeyState::Up,
                        )))
                    }
                    _ => {
                        Ok(Some(KeyEvent::new(
                            Self::map_extended_scancode(code)?,
                            KeyState::Down,
                        )))
                    }
                }
            }
            DecodeState::Extended2 => {
                self.state = DecodeState::Start;
                match code {
                    0x80..=0xFF => {
                        Ok(Some(KeyEvent::new(
                            Self::map_extended2_scancode(code - 0x80)?,
                            KeyState::Up,
                        )))
                    }
                    _ => {
                        Ok(Some(KeyEvent::new(
                            Self::map_extended2_scancode(code)?,
                            KeyState::Down,
                        )))
                    }
                }
            }
            _ => {
                // cant get in to this state
                unimplemented!();
            }
        }
    }
}

impl Default for ScancodeSet1 {
    fn default() -> Self {
        ScancodeSet1::new()
    }
}