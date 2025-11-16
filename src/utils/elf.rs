// https://codebrowser.dev/linux/include/elf.h.html

pub const SHF_WRITE: u64 = 1 << 0;
pub const SHF_ALLOC: u64 = 1 << 1;
pub const SHF_EXECINSTR: u64 = 1 << 2;

#[repr(C)]
#[derive(Debug)]
pub struct Elf32Shdr {
	pub sh_name: u32,
	pub sh_type: u32,
	pub sh_flags: u32,
	pub sh_addr: u32,
	pub sh_offset: u32,
	pub sh_size: u32,
	pub sh_link: u32,
	pub sh_info: u32,
	pub sh_addralign: u32,
	pub sh_entsize: u32
}

#[repr(C)]
#[derive(Debug)]
pub struct Elf64Shdr {
	pub sh_name: u32,
	pub sh_type: u32,
	pub sh_flags: u64,
	pub sh_addr: u64,
	pub sh_offset: u64,
	pub sh_size: u64,
	pub sh_link: u32,
	pub sh_info: u32,
	pub sh_addralign: u64,
	pub sh_entsize: u64
}
