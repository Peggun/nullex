use crate::{fs, process::elf::parse_elf, serial_println};

// this runs a elf binary in kernel mode.
pub fn sys_run(path: &str) -> i32 {
	let maybe_bytes = fs::with_fs(|fs| fs.read_file_to_vec(path).ok());
	let elf_bytes = match maybe_bytes {
		Some(b) => b,
		None => {
			serial_println!("sys_run: file not found: {}", path);
			return -1;
		}
	};
	let _e = parse_elf(&elf_bytes);
	0
}
