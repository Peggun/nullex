//!
//! command.rs
//!
//! Command handling and definitions module for the kernel.
//! 

use alloc::{
	collections::BTreeMap,
	string::{String, ToString},
	vec::Vec
};

use crate::{
	drivers::keyboard::scancode::CWD,
	fs::{self, ramfs::Permission, resolve_path},
	lazy_static,
	print,
	println,
	rtc::read_rtc_time,
	serial_println,
	task::{ProcessId, executor::EXECUTOR},
	utils::{
		logger::{levels::LogLevel, sinks::SYSLOG_SINK, traits::logger_sink::LoggerSink},
		mutex::SpinMutex
	},
	vga_buffer::WRITER
};

lazy_static! {
	/// Static reference to a list of all of the commands that have been run in this instance.
	pub static ref CMD_HISTORY: SpinMutex<Vec<String>> = SpinMutex::new(Vec::new());
	/// Static reference to the current command history index we are at.
	pub static ref CMD_HISTORY_INDEX: SpinMutex<usize> = SpinMutex::new(0);
}

/// A type alias for a command function.
type CommandFunction = fn(args: &[&str]);

#[derive(Clone, Copy, PartialEq)]
/// Enum representing types of commands.
enum CommandType {
	Generic,
	// this might be used in the future.
	_Application
}

/// A command structure containing the command name, the function to call, and
/// help text.
#[derive(Clone, Copy)]
#[allow(unused)]
pub struct Command {
	name: &'static str,
	func: CommandFunction,
	help: &'static str,
	// this might be used in the future.
	cmd_type: CommandType
}

lazy_static! {
	static ref COMMAND_REGISTRY: SpinMutex<BTreeMap<String, Command>> =
		SpinMutex::new(BTreeMap::new());
}

/// Register a command in the global command registry.
pub fn register_command(cmd: Command) {
	COMMAND_REGISTRY.lock().insert(cmd.name.to_string(), cmd);
}

/// Look up and run a command based on input.
pub fn run_command(input: &str) {
	let parts: Vec<&str> = input.split_whitespace().collect();
	if parts.is_empty() {
		return;
	}
	let command = parts[0];
	let args = &parts[1..];

	// copy the command out while holding the lock
	let cmd_opt = {
		let registry = COMMAND_REGISTRY.lock();
		registry.get(command).copied()
	};

	{
		let mut history = CMD_HISTORY.lock();
		history.push(input.to_string());
		// reset the history index to the end of the history.
		*CMD_HISTORY_INDEX.lock() = history.len();
	}

	if let Some(cmd) = cmd_opt {
		(cmd.func)(args);
	} else {
		println!("Command not found: {}", command);
	}
}

/// Initialize the default commands for the shell.
pub fn init_commands() {
	SYSLOG_SINK.log("Initializing Keyboard Commands...\n", LogLevel::Info);
	register_command(Command {
		name: "echo",
		func: echo,
		help: "Print arguments",
		cmd_type: CommandType::Generic
	});
	register_command(Command {
		name: "clear",
		func: clear,
		help: "Clear the screen",
		cmd_type: CommandType::Generic
	});
	register_command(Command {
		name: "help",
		func: help,
		help: "Show available commands",
		cmd_type: CommandType::Generic
	});
	register_command(Command {
		name: "ls",
		func: ls,
		help: "List directory contents",
		cmd_type: CommandType::Generic
	});
	register_command(Command {
		name: "cat",
		func: cat,
		help: "Display file content",
		cmd_type: CommandType::Generic
	});
	register_command(Command {
		name: "cd",
		func: cd,
		help: "Change directory",
		cmd_type: CommandType::Generic
	});
	register_command(Command {
		name: "touch",
		func: touch,
		help: "Create an empty file",
		cmd_type: CommandType::Generic
	});
	register_command(Command {
		name: "mkdir",
		func: mkdir,
		help: "Create a directory",
		cmd_type: CommandType::Generic
	});
	register_command(Command {
		name: "rm",
		func: rm,
		help: "Remove a file",
		cmd_type: CommandType::Generic
	});
	register_command(Command {
		name: "rmdir",
		func: rmdir,
		help: "Remove a directory",
		cmd_type: CommandType::Generic
	});
	register_command(Command {
		name: "write",
		func: write_file,
		help: "Write content to a file",
		cmd_type: CommandType::Generic
	});
	register_command(Command {
		name: "progs",
		func: progs,
		help: "List running processes",
		cmd_type: CommandType::Generic
	});
	register_command(Command {
		name: "kill",
		func: kill,
		help: "Kill a process",
		cmd_type: CommandType::Generic
	});
	register_command(Command {
		name: "time",
		func: time,
		help: "Current date and time.",
		cmd_type: CommandType::Generic
	});
	register_command(Command {
		name: "testnet",
		func: testnet,
		help: "Test the network.",
		cmd_type: CommandType::Generic
	});
	register_command(Command {
		name: "rslv",
		func: rslv,
		help: "Resolve a hostname.",
		cmd_type: CommandType::Generic
	});
	register_command(Command {
		name: "ping",
		func: ping,
		help: "Ping a hostname",
		cmd_type: CommandType::Generic
	});
	register_command(Command {
		name: "netpoll",
		func: netpoll,
		help: "Poll the RX queue",
		cmd_type: CommandType::Generic
	});

	SYSLOG_SINK.log("Done.\n", LogLevel::Info);
}

fn progs(_args: &[&str]) {
	if let Some(executor) = EXECUTOR.try_lock() {
		executor.list_processes();
	} else {
		println!("System busy; try again.");
	}
}

fn echo(args: &[&str]) {
	println!("{}", args.join(" "));
}

fn clear(_args: &[&str]) {
	WRITER.lock().clear_everything();
}

fn help(_args: &[&str]) {
	println!("Available commands:");
	for cmd in COMMAND_REGISTRY.lock().values() {
		println!("{} - {}", cmd.name, cmd.help);
	}
}

fn ls(args: &[&str]) {
	let path = resolve_path(if args.is_empty() { "." } else { args[0] });
	fs::with_fs(|fs| match fs.list_dir(&path) {
		Ok(entries) => {
			for entry in entries {
				print!("{} ", entry);
			}
			println!();
		}
		Err(_) => println!("ls: cannot access '{}'", path)
	});
}

fn cat(args: &[&str]) {
	if args.is_empty() {
		println!("cat: missing file operand");
		return;
	}
	let path = resolve_path(args[0]);
	fs::with_fs(|fs| match fs.read_file(&path) {
		Ok(content) => {
			let s = String::from_utf8_lossy(content);
			println!("{}", s)
		}
		Err(_) => println!("cat: {}: No such file ", path)
	});
}

fn cd(args: &[&str]) {
	let path = if args.is_empty() {
		"/".to_string()
	} else {
		resolve_path(args[0])
	};

	fs::with_fs(|fs| {
		if fs.is_dir(&path) {
			*CWD.lock() = path;
		} else {
			println!("cd: no such directory: {}", args[0]);
		}
	});
}

fn touch(args: &[&str]) {
	if args.is_empty() {
		println!("touch: missing file operand");
		return;
	}
	for file in args {
		let path = resolve_path(file);
		fs::with_fs(|fs| {
			if fs.read_file(&path).is_err() && fs.create_file(&path, Permission::all()).is_err() {
				println!("touch: cannot create file '{}'", file);
			}
		});
	}
}

fn mkdir(args: &[&str]) {
	if args.is_empty() {
		println!("mkdir: missing operand");
		return;
	}
	for dir in args {
		let path = resolve_path(dir);
		fs::with_fs(|fs| {
			if fs.create_dir(&path, Permission::all()).is_err() {
				println!("mkdir: cannot create directory '{}'", dir);
			}
		});
	}
}

fn rm(args: &[&str]) {
	if args.is_empty() {
		println!("rm: missing operand");
		return;
	}
	for arg in args {
		let path = resolve_path(arg);
		fs::with_fs(|fs| {
			if fs.is_dir(&path) {
				println!("rm: cannot remove '{}': Is a directory", arg);
			} else {
				match fs.remove(&path, false, false) {
					Ok(_) => {}
					Err(_) => println!("rm: cannot remove '{}': No such file", arg)
				}
			}
		});
	}
}

fn rmdir(args: &[&str]) {
	if args.is_empty() {
		println!("rmdir: missing operand");
		return;
	}
	let recursive = args.contains(&"-r");
	let dirs: Vec<&str> = args.iter().filter(|&&arg| arg != "-r").cloned().collect();
	if dirs.is_empty() {
		println!("rmdir: missing operand");
		return;
	}
	for dir in dirs {
		let path = resolve_path(dir);
		fs::with_fs(|fs| {
			if fs.is_dir(&path) {
				match fs.remove(&path, true, recursive) {
					Ok(_) => {}
					Err(_) => println!("rmdir: failed to remove '{}'", dir)
				}
			} else {
				println!("rmdir: failed to remove '{}': Not a directory", dir);
			}
		});
	}
}

fn write_file(args: &[&str]) {
	if args.len() < 2 {
		println!("Usage: write <file> <content>");
		return;
	}
	let path = resolve_path(args[0]);
	let content = args[1..].join(" ");
	fs::with_fs(|fs| {
		if fs.write_file(&path, content.as_bytes(), false).is_err() {
			println!("write: failed to write to '{}'", args[0]);
		}
	});
}

fn kill(args: &[&str]) {
	if args.is_empty() {
		println!("kill: missing PID");
		return;
	}

	let pid = match args[0].parse::<u64>() {
		Ok(pid) => pid,
		Err(_) => {
			println!("kill: invalid PID '{}'", args[0]);
			return;
		}
	};

	EXECUTOR.lock().end_process(ProcessId::new(pid), -2);

	// Kill process
	serial_println!("Killed process {}", pid);
}

fn time(_args: &[&str]) {
	let time = read_rtc_time();

	println!("{}", time);
}

fn testnet(_args: &[&str]) {
	match crate::net::send_arp_request(crate::net::GATEWAY_IP) {
		Ok(()) => println!("ARP request sent to gateway"),
		Err(e) => println!("Failed to send ARP: {}", e)
	}
}

fn rslv(args: &[&str]) {
	if args.is_empty() {
		println!("rslv: need hostname to resolve to.");
		return;
	}

	let hn = args[0];
	match crate::net::dns::resolve(hn) {
		Ok(ip) => {
			println!(
				"hostname: {} resolved to {}.{}.{}.{}",
				hn, ip[0], ip[1], ip[2], ip[3]
			);
		}
		Err(e) => println!("DNS Error: {}", e)
	}
}

fn ping(args: &[&str]) {
	if args.is_empty() {
		println!("ping: need hostname to ping to");
		return;
	}

	let hn = args[0];
	match crate::net::dns::resolve(hn) {
		Ok(ip) => match crate::net::send_ping(ip, 1) {
			Ok(()) => println!("Ping sent to {}!", hn),
			Err(e) => println!("Ping error: {}", e)
		},
		Err(e) => println!("DNS Error: {}", e)
	}
}

fn netpoll(_args: &[&str]) {
	println!("=== Manual Network Poll ===");
	crate::drivers::virtio::net::rx_poll();
	println!("=== Poll Complete ===");
}
