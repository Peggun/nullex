use alloc::{collections::btree_map::BTreeMap, string::{String, ToString}, vec::Vec};
use hashbrown::HashMap;
use lazy_static::lazy_static;
use spin::Mutex;

use crate::{error::NullexError, serial_println};

pub type SerialCmdFn = fn(&[&str]);

#[derive(Debug, Copy, Clone)]
pub struct SerialCommand {
    pub name: &'static str,
    pub help: &'static str,
    pub func: SerialCmdFn,
}

lazy_static! {
    static ref SERIAL_COMMAND_REGISTRY: Mutex<BTreeMap<String, SerialCommand>> = Mutex::new(BTreeMap::new());
}

pub fn register_serial_command(cmd: SerialCommand) {
    SERIAL_COMMAND_REGISTRY.lock().insert(cmd.name.to_string(), cmd);
}

// same as vga keyboard commands.
pub fn run_serial_command(input: &str) {
    let parts: Vec<&str> = input.split_whitespace().collect();
	if parts.is_empty() {
		return;
	}
	let command = parts[0];
	let args = &parts[1..];

	let cmd_opt = {
		let registry = SERIAL_COMMAND_REGISTRY.lock();
		registry.get(command).copied()
	};

	if let Some(cmd) = cmd_opt {
		
		(cmd.func)(args);
	} else {
		serial_println!("Command not found: {}", command);
	}
}

pub fn init_serial_commands() {
    register_serial_command(SerialCommand {
        name: "echo",
        func: echo,
        help: "Print arguments",
    });
}

pub fn echo(args: &[&str]) {
	serial_println!("{}", args.join(" "));
}