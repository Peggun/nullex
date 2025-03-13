// command.rs

/*
Command handling and definitions module for the kernel.
*/

extern crate alloc;

use alloc::{
	collections::BTreeMap,
	string::{String, ToString},
	vec::Vec
};

use lazy_static::lazy_static;
use spin::Mutex;

use crate::{
	constants::SYSLOG_SINK,
	errors::{
		COMMAND_INVALID_ARGUMENTS,
		COMMAND_MISSING_ARGUMENT,
		COMMAND_NO_ARGUMENTS,
		//COMMAND_NOT_FOUND,
		KernelError,
		SYSTEM_BUSY
	},
	fs::{self, ramfs::Permission},
	print,
	println,
	serial_println,
	syscall,
	task::{ProcessId, executor::EXECUTOR},
	utils::logger::{levels::LogLevel, traits::logger_sink::LoggerSink},
	vga_buffer::WRITER
};

lazy_static! {
	pub static ref CMD_HISTORY: Mutex<Vec<String>> = Mutex::new(Vec::new());
	pub static ref CMD_HISTORY_INDEX: Mutex<usize> = Mutex::new(0);
}

/// A type alias for a command function.
pub type CommandFunction = fn(args: &[&str]) -> Result<(), KernelError>;

/// A command structure containing the command name, the function to call, and
/// help text.
#[derive(Clone, Copy)]
pub struct Command {
	pub name: &'static str,
	pub func: CommandFunction,
	pub help: &'static str
}

lazy_static! {
	static ref COMMAND_REGISTRY: Mutex<BTreeMap<String, Command>> = Mutex::new(BTreeMap::new());
}

/// Register a command in the global command registry.
pub fn register_command(cmd: Command) -> Result<(), KernelError> {
	COMMAND_REGISTRY.lock().insert(cmd.name.to_string(), cmd);
	Ok(())
}

/// Look up and run a command based on input.
pub fn run_command(input: &str) -> Result<(), KernelError> {
	let parts: Vec<&str> = input.split_whitespace().collect();
	if parts.is_empty() {
		return Ok(());
	}
	let command = parts[0];
	let args = &parts[1..];

	// Copy the command out while holding the lock
	let cmd_opt = {
		let registry = COMMAND_REGISTRY.lock();
		registry.get(command).copied() // `copied()` turns &Command into Command
	};

	{
		let mut history = CMD_HISTORY.lock();
		history.push(input.to_string());
		// Reset the history index to the end of the history.
		*CMD_HISTORY_INDEX.lock() = history.len();
	}

	if let Some(cmd) = cmd_opt {
		// Call the command function and propagate its result
		(cmd.func)(args)
	} else {
		println!("Command not found: {}", command);
		Ok(()) // Returning Ok since "command not found" is not a fatal error here
		// Otherwise return a KernelError::CommandError(COMMAND_NOT_FOUND)
	}
}

/// Initialize the default commands for the shell.
pub fn init_commands() -> Result<(), KernelError> {
	SYSLOG_SINK.log("Initializing Keyboard Commands...\n", LogLevel::Info);
	let _ = register_command(Command {
		name: "echo",
		func: echo,
		help: "Print arguments"
	});
	let _ = register_command(Command {
		name: "clear",
		func: clear,
		help: "Clear the screen"
	});
	let _ = register_command(Command {
		name: "help",
		func: help,
		help: "Show available commands"
	});
	let _ = register_command(Command {
		name: "ls",
		func: ls,
		help: "List directory contents"
	});
	let _ = register_command(Command {
		name: "cat",
		func: cat,
		help: "Display file content"
	});
	let _ = register_command(Command {
		name: "cd",
		func: cd,
		help: "Change directory"
	});
	let _ = register_command(Command {
		name: "touch",
		func: touch,
		help: "Create an empty file"
	});
	let _ = register_command(Command {
		name: "mkdir",
		func: mkdir,
		help: "Create a directory"
	});
	let _ = register_command(Command {
		name: "rm",
		func: rm,
		help: "Remove a file"
	});
	let _ = register_command(Command {
		name: "rmdir",
		func: rmdir,
		help: "Remove a directory"
	});
	let _ = register_command(Command {
		name: "write",
		func: write_file,
		help: "Write content to a file"
	});
	let _ = register_command(Command {
		name: "exit",
		func: sys_exit_shell,
		help: "Exit the shell"
	});
	let _ = register_command(Command {
		name: "progs",
		func: progs,
		help: "List running processes"
	});
	let _ = register_command(Command {
		name: "kill",
		func: kill,
		help: "Kill a process"
	});
	SYSLOG_SINK.log("Done.\n", LogLevel::Info);
	Ok(())
}

/// Helper function to resolve a file path relative to the current working
/// directory.
fn resolve_path(path: &str) -> String {
	// We import CWD from the scancode module.
	use crate::task::keyboard::scancode::CWD;
	let mut cwd = CWD.lock().clone();
	let mut result = if path.starts_with('/') {
		String::new()
	} else {
		cwd.push('/');
		cwd
	};
	result.push_str(path);
	normalize_path(&result)
}

fn normalize_path(path: &str) -> String {
	let parts: Vec<&str> = path
		.split('/')
		.filter(|&p| !p.is_empty() && p != ".")
		.collect();
	let mut stack = Vec::new();
	for part in parts {
		if part == ".." {
			if !stack.is_empty() {
				stack.pop();
			}
		} else {
			stack.push(part);
		}
	}
	if stack.is_empty() {
		"/".to_string()
	} else {
		format!("/{}/", stack.join("/"))
	}
}

pub fn progs(_args: &[&str]) -> Result<(), KernelError> {
	if let Some(executor) = EXECUTOR.try_lock() {
		executor.list_processes();
		Ok(())
	} else {
		println!("System busy; try again.");
		Err(KernelError::SystemError(SYSTEM_BUSY))
	}
}

pub fn echo(args: &[&str]) -> Result<(), KernelError> {
	println!("{}", args.join(" "));
	Ok(())
}

pub fn clear(_args: &[&str]) -> Result<(), KernelError> {
	WRITER.lock().clear_everything();
	Ok(())
}

pub fn help(_args: &[&str]) -> Result<(), KernelError> {
	println!("Available commands:");
	for cmd in COMMAND_REGISTRY.lock().values() {
		println!("{} - {}", cmd.name, cmd.help);
	}

	Ok(())
}

pub fn ls(args: &[&str]) -> Result<(), KernelError> {
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
	Ok(())
}

pub fn cat(args: &[&str]) -> Result<(), KernelError> {
	if args.is_empty() {
		println!("cat: missing file operand");
		return Err(KernelError::CommandError(COMMAND_NO_ARGUMENTS));
	}
	let path = resolve_path(args[0]);
	fs::with_fs(|fs| match fs.read_file(&path) {
		Ok(content) => {
			let s = String::from_utf8_lossy(content);
			println!("{}", s)
		}
		Err(_) => println!("cat: {}: No such file", path)
	});
	Ok(())
}

pub fn cd(args: &[&str]) -> Result<(), KernelError> {
	let path = if args.is_empty() {
		"/".to_string()
	} else {
		resolve_path(args[0])
	};

	fs::with_fs(|fs| {
		if fs.is_dir(&path) {
			use crate::task::keyboard::scancode::CWD;
			*CWD.lock() = path;
		} else {
			println!("cd: no such directory: {}", args[0]);
		}
	});
	Ok(())
}

pub fn touch(args: &[&str]) -> Result<(), KernelError> {
	if args.is_empty() {
		println!("touch: missing file operand");
		return Err(KernelError::CommandError(COMMAND_MISSING_ARGUMENT));
	}
	for file in args {
		let path = resolve_path(file);
		fs::with_fs(|fs| {
			if fs.read_file(&path).is_err() {
				if let Err(_) = fs.create_file(&path, Permission::all()) {
					println!("touch: cannot create file '{}'", file);
				}
			}
		});
	}
	Ok(())
}

pub fn mkdir(args: &[&str]) -> Result<(), KernelError> {
	if args.is_empty() {
		println!("mkdir: missing operand");
		return Err(KernelError::CommandError(COMMAND_MISSING_ARGUMENT))
	}
	for dir in args {
		let path = resolve_path(dir);
		fs::with_fs(|fs| {
			if let Err(_) = fs.create_dir(&path, Permission::all()) {
				println!("mkdir: cannot create directory '{}'", dir);
			}
		});
	}
	Ok(())
}

pub fn rm(args: &[&str]) -> Result<(), KernelError> {
	if args.is_empty() {
		println!("rm: missing operand");
		return Err(KernelError::CommandError(COMMAND_MISSING_ARGUMENT))
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
	Ok(())
}

pub fn rmdir(args: &[&str]) -> Result<(), KernelError> {
	if args.is_empty() {
		println!("rmdir: missing operand");
		return Err(KernelError::CommandError(COMMAND_MISSING_ARGUMENT))
	}
	let recursive = args.iter().any(|&arg| arg == "-r");
	let dirs: Vec<&str> = args.iter().filter(|&&arg| arg != "-r").cloned().collect();
	if dirs.is_empty() {
		println!("rmdir: missing operand");
		return Err(KernelError::CommandError(COMMAND_MISSING_ARGUMENT)) // Not sure why because of dirs being empty.
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
	Ok(())
}

pub fn write_file(args: &[&str]) -> Result<(), KernelError> {
	if args.len() < 2 {
		println!("Usage: write <file> <content>");
		return Err(KernelError::CommandError(COMMAND_MISSING_ARGUMENT))
	}
	let path = resolve_path(args[0]);
	let content = args[1..].join(" ");
	fs::with_fs(|fs| {
		if let Err(_) = fs.write_file(&path, content.as_bytes()) {
			println!("write: failed to write to '{}'", args[0]);
		}
	});
	Ok(())
}

pub fn sys_exit_shell(_args: &[&str]) -> Result<(), KernelError> {
	syscall::sys_exit(0);
	//Ok(())
}

/// Optional helper: join two paths together.
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

pub fn kill(args: &[&str]) -> Result<(), KernelError> {
	if args.is_empty() {
		println!("kill: missing PID");
		return Err(KernelError::CommandError(COMMAND_MISSING_ARGUMENT))
	}

	let pid = match args[0].parse::<u64>() {
		Ok(pid) => pid,
		Err(_) => {
			println!("kill: invalid PID '{}'", args[0]);
			return Err(KernelError::CommandError(COMMAND_INVALID_ARGUMENTS))
		}
	};

	EXECUTOR.lock().end_process(ProcessId::new(pid), -2);

	// Kill process
	serial_println!("Killed process {}", pid);
	Ok(())
}
