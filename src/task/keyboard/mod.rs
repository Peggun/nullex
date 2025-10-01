pub mod commands;
pub mod scancode;

pub use commands::{Command, init_commands, register_command, run_command};
pub use scancode::{ScancodeStream, print_keypresses};

// kbd special consts for keys
pub const KEYBOARD_BACKSPACE: u8 = 0x0008;
pub const KEYBOARD_TAB: u8 = 0x0009; // b'\t'
pub const KEYBOARD_ENTER: u8 = 0x000a;
pub const KEYBOARD_SPACE: u8 = 0x0020; // b' '
pub const KEYBOARD_ESCAPE: u8 = 0x001b;
pub const KEYBOARD_DELETE: u8 = 0x007f;
pub const KEYBOARD_LEFT_ARROW: u8 = 0x0025;
pub const KEYBOARD_UP_ARROW: u8 = 0x0026;
pub const KEYBOARD_RIGHT_ARROW: u8 = 0x0027;
pub const KEYBOARD_DOWN_ARROW: u8 = 0x0028;

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
