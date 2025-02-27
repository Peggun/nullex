pub mod scancode;
pub mod commands;

pub use scancode::{ScancodeStream, print_keypresses};
pub use commands::{register_command, run_command, init_commands, Command};
