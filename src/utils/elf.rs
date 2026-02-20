//!
//! elf.rs
//! 
//! ELF binary helpers for the kernel.
//! 

// https://codebrowser.dev/linux/include/elf.h.html

const SHF_WRITE: u64 = 1 << 0;
const SHF_ALLOC: u64 = 1 << 1;
const SHF_EXECINSTR: u64 = 1 << 2;

#[repr(C)]
#[derive(Debug)]
struct Elf32Shdr {
	sh_name: u32,
	sh_type: u32,
	sh_flags: u32,
	sh_addr: u32,
	sh_offset: u32,
	sh_size: u32,
	sh_link: u32,
	sh_info: u32,
	sh_addralign: u32,
	sh_entsize: u32
}

#[repr(C)]
#[derive(Debug)]
struct Elf64Shdr {
	sh_name: u32,
	sh_type: u32,
	sh_flags: u64,
	sh_addr: u64,
	sh_offset: u64,
	sh_size: u64,
	sh_link: u32,
	sh_info: u32,
	sh_addralign: u64,
	sh_entsize: u64
}
