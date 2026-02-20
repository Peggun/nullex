//!
//! serial_kfunc.rs
//! 
//! Serial kernel functions for the kernel.
//! 
//! At the moment this is not needed, and probably not ever. This was a little
//! idea that I found quickly was completely useless. not removing yet but
//! most likely will in the future.
//! 


use alloc::{
	collections::btree_map::BTreeMap,
	string::{String, ToString},
	vec::Vec
};

use crate::{
	apic::APIC_TICK_COUNT, lazy_static, serial_println, utils::{cpu_utils::get_cpu_clock, mutex::SpinMutex}
};

type SerialCmdFn = fn(&[&str]);

#[derive(Debug, Copy, Clone)]
struct SerialCommand {
	name: &'static str,
	help: &'static str,
	func: SerialCmdFn
}

lazy_static! {
	static ref SERIAL_COMMAND_REGISTRY: SpinMutex<BTreeMap<String, SerialCommand>> =
		SpinMutex::new(BTreeMap::new());
}

fn register_serial_command(cmd: SerialCommand) {
	SERIAL_COMMAND_REGISTRY
		.lock()
		.insert(cmd.name.to_string(), cmd);
}

// same as vga keyboard commands.
/// Runs a serial command.
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

fn init_serial_commands() {
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

fn echo(args: &[&str]) {
	serial_println!("{}", args.join(" "));
}

fn help(_args: &[&str]) {
	// prints the commands, last command i will code.
	serial_println!("");
}

fn uptime(_args: &[&str]) {
	//update_system_uptime();
	let ticks = APIC_TICK_COUNT.load(core::sync::atomic::Ordering::Relaxed);
	serial_println!("ticks: {}", ticks);
}

fn clock(_args: &[&str]) {
	unsafe {
		serial_println!("clock: {}", get_cpu_clock());
	}
}
