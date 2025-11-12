// command.rs

/*
Command handling and definitions module for the kernel.
*/

use alloc::{
	collections::BTreeMap,
	string::{String, ToString},
	vec::Vec
};

use lazy_static::lazy_static;

use crate::{
	apic::{TICK_COUNT, to_hrt}, constants::SYSLOG_SINK, drivers::keyboard::scancode::CWD, fs::{self, ramfs::Permission, resolve_path}, print, println, programs::{nedit::app::nedit_app, nulx::run}, serial_println, syscall, task::{ProcessId, executor::EXECUTOR}, utils::{logger::{levels::LogLevel, traits::logger_sink::LoggerSink}, mutex::SpinMutex}, vga_buffer::WRITER
};

lazy_static! {
	pub static ref CMD_HISTORY: SpinMutex<Vec<String>> = SpinMutex::new(Vec::new());
	pub static ref CMD_HISTORY_INDEX: SpinMutex<usize> = SpinMutex::new(0);
}

/// A type alias for a command function.
pub type CommandFunction = fn(args: &[&str]);

#[derive(Clone, Copy, PartialEq)]
pub enum CommandType {
	Generic,
	Application
}

/// A command structure containing the command name, the function to call, and
/// help text.
#[derive(Clone, Copy)]
pub struct Command {
	pub name: &'static str,
	pub func: CommandFunction,
	pub help: &'static str,
	pub cmd_type: CommandType
}

lazy_static! {
	static ref COMMAND_REGISTRY: SpinMutex<BTreeMap<String, Command>> = SpinMutex::new(BTreeMap::new());
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
		name: "exit",
		func: sys_exit_shell,
		help: "Exit the shell",
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

	// to be removed.
	register_command(Command {
		name: "nulx",
		func: run, // nulx_run
		help: "Run the nulx programming language",
		cmd_type: CommandType::Generic
	});
	register_command(Command {
		name: "nedit",
		func: nedit_app,
		help: "Edit any files within Nullex",
		cmd_type: CommandType::Application
	});
	
	register_command(Command {
		name: "uptime",
		func: uptime,
		help: "System uptime.",
		cmd_type: CommandType::Generic
	});
	SYSLOG_SINK.log("Done.\n", LogLevel::Info);
}

pub fn progs(_args: &[&str]) {
	if let Some(executor) = EXECUTOR.try_lock() {
		executor.list_processes();
	} else {
		println!("System busy; try again.");
	}
}

pub fn echo(args: &[&str]) {
	println!("{}", args.join(" "));
}

pub fn clear(_args: &[&str]) {
	WRITER.lock().clear_everything();
}

pub fn help(_args: &[&str]) {
	println!("Available commands:");
	for cmd in COMMAND_REGISTRY.lock().values() {
		println!("{} - {}", cmd.name, cmd.help);
	}
}

pub fn ls(args: &[&str]) {
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

pub fn cat(args: &[&str]) {
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

pub fn cd(args: &[&str]) {
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

pub fn touch(args: &[&str]) {
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

pub fn mkdir(args: &[&str]) {
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

pub fn rm(args: &[&str]) {
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

pub fn rmdir(args: &[&str]) {
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

pub fn write_file(args: &[&str]) {
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

pub fn sys_exit_shell(_args: &[&str]) {
	syscall::sys_exit(0);
}

/// join two paths together.
pub fn join_paths(path: &str, next: &str, out: &mut String) {
	const FS_SEP: char = '/';
	out.clear();
	if !next.starts_with(FS_SEP) {
		out.push_str(path);
		if !path.ends_with(FS_SEP) {
			out.push(FS_SEP);
		}
	}
	out.push_str(next);
	if out.ends_with(FS_SEP) {
		out.pop();
	}
}

pub fn kill(args: &[&str]) {
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

pub fn uptime(_args: &[&str]) {
	let ticks = TICK_COUNT.load(core::sync::atomic::Ordering::Relaxed);
	let time = to_hrt(ticks);
	println!(
		"up: {} days {}:{}:{}.{}",
		time.days, time.hours, time.mins, time.secs, time.ms
	);
}
