use alloc::{
	collections::btree_map::BTreeMap,
	string::{String, ToString},
	vec::Vec
};

use crate::{
	apic::TICK_COUNT,
	lazy_static,
	serial_println,
	utils::{cpu_utils::get_cpu_clock, mutex::SpinMutex}
};

pub type SerialCmdFn = fn(&[&str]);

#[derive(Debug, Copy, Clone)]
pub struct SerialCommand {
	pub name: &'static str,
	pub help: &'static str,
	pub func: SerialCmdFn
}

lazy_static! {
	static ref SERIAL_COMMAND_REGISTRY: SpinMutex<BTreeMap<String, SerialCommand>> =
		SpinMutex::new(BTreeMap::new());
}

pub fn register_serial_command(cmd: SerialCommand) {
	SERIAL_COMMAND_REGISTRY
		.lock()
		.insert(cmd.name.to_string(), cmd);
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
		help: "Print arguments"
	});
	register_serial_command(SerialCommand {
		name: "uptime",
		help: "Shows how long nullex has been running",
		func: uptime
	});
	register_serial_command(SerialCommand {
		name: "clock",
		help: "Gets the CPU Clock Speed",
		func: clock
	});
}

pub fn echo(args: &[&str]) {
	serial_println!("{}", args.join(" "));
}

pub fn help(_args: &[&str]) {
	// prints the commands, last command i will code.
	serial_println!("");
}

pub fn uptime(_args: &[&str]) {
	//update_system_uptime();
	let ticks = TICK_COUNT.load(core::sync::atomic::Ordering::Relaxed);
	serial_println!("ticks: {}", ticks);
}

pub fn clock(_args: &[&str]) {
	unsafe {
		serial_println!("clock: {}", get_cpu_clock());
	}
}
