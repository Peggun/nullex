pub mod commands;
pub mod scancode;

pub use commands::{init_commands, register_command, run_command, Command};
pub use scancode::{print_keypresses, ScancodeStream};
