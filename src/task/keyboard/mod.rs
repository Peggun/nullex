//!
//! src/task/keyboard/mod.rs
//! 
//! Task keyboard handling module defintion.
//! 

pub mod commands;

pub use commands::{Command, init_commands, register_command, run_command};

// kbd special consts for keys
const KEYBOARD_BACKSPACE: u8 = 0x0008;
const KEYBOARD_TAB: u8 = 0x0009; // b'\t'
const KEYBOARD_ENTER: u8 = 0x000a;
const KEYBOARD_SPACE: u8 = 0x0020; // b' '
const KEYBOARD_ESCAPE: u8 = 0x001b;
const KEYBOARD_DELETE: u8 = 0x007f;
const KEYBOARD_LEFT_ARROW: u8 = 0x0025;
const KEYBOARD_UP_ARROW: u8 = 0x0026;
const KEYBOARD_RIGHT_ARROW: u8 = 0x0027;
const KEYBOARD_DOWN_ARROW: u8 = 0x0028;

/// Array representing all QWERTY Us104Key Raw keys.
pub static KEYBOARD_RAW_KEYS: [u8; 10] = [
	KEYBOARD_BACKSPACE,
	KEYBOARD_TAB,
	KEYBOARD_ENTER,
	KEYBOARD_SPACE,
	KEYBOARD_ESCAPE,
	KEYBOARD_DELETE,
	KEYBOARD_LEFT_ARROW,
	KEYBOARD_UP_ARROW,
	KEYBOARD_RIGHT_ARROW,
	KEYBOARD_DOWN_ARROW
];
