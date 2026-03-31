//!
//! elf.rs
//! 
//! ELF binary helpers for the kernel.
//! 

// https://codebrowser.dev/linux/include/elf.h.html

use core::{arch::asm, ptr::{copy_nonoverlapping, write_bytes}};

use alloc::{sync::Arc, vec::Vec};
use x86_64::{VirtAddr, registers::control::{Cr3, Cr3Flags}, structures::paging::{FrameAllocator, Mapper, OffsetPageTable, Page, PageTable, PageTableFlags, page::PageRange}};

use crate::{allocator::ALLOCATOR_INFO, arch::x86_64::user::{enter_user_process, setup_user_stack}, error::NullexError, fs::{self, resolve_path}, memory::{map_range, phys_to_virt}, println, serial_println, task::{AddressSpace, Process, ProcessState, UserContext}, utils::process::{spawn_process, spawn_user_process}};

const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];

const EI_NIDENT: usize = 16;

pub(crate) const HELLO_ELF: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/build/userspace/hello/hello.elf"));
//pub const BARE_ELF: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/build/userspace/bare/bare.elf"));

// these are the same for 32bit and 64-bit. 
// so its probably concise to use a generic name
// naming-wise, probably Elf32 and Elf64 are better i guess.
type ElfHalf = u16;
type ElfWord = u32;
type ElfSword = i32;
type ElfXword = u64;
type ElfSxword = i64;

type Elf64Addr = u64;
type Elf64Off = u64;

type ElfSection = u16;

type ElfVersym = ElfHalf;

const PT_LOAD: ElfWord = 1;

const PF_X: u32 = 0x1;
const PF_W: u32 = 0x2;
const PF_R: u32 = 0x4;

/// ELF file header
#[repr(C)]
#[derive(Debug)]
pub struct Elf64Ehdr {
	e_ident: [u8; EI_NIDENT],
	e_type: ElfHalf,
	e_machine: ElfHalf,
	e_version: ElfWord,
	e_entry: Elf64Addr,
	e_phoff: Elf64Off,
	e_shoff: Elf64Off,
	e_flags: ElfWord,
	e_ehsize: ElfHalf,
	e_phentsize: ElfHalf,
	e_phnum: ElfHalf,
	e_shentsize: ElfHalf,
	e_shnum: ElfHalf,
	e_shstrrndx: ElfHalf,
}

/// ELF file section header
#[repr(C)]
#[derive(Debug)]
pub struct Elf64Shdr {
	sh_name: ElfWord,
	sh_type: ElfWord,
	sh_flags: ElfXword,
	sh_addr: Elf64Addr,
	sh_offset: Elf64Off,
	sh_size: ElfXword,
	sh_link: ElfWord,
	sh_info: ElfWord,
	sh_addralign: ElfXword,
	sh_entsize: ElfXword
}

/// ELF file program header.
#[repr(C)]
#[derive(Debug, Default)]
pub struct Elf64Phdr {
	p_type: ElfWord,
	p_flags: ElfWord,
	p_offset: Elf64Off,
	p_vaddr: Elf64Addr,
	p_paddr: Elf64Addr,
	p_filesz: ElfXword,
	p_memsz: ElfXword,
	p_align: ElfXword,
}
#[repr(C)]
/// A PT_LOAD Segment of a ELF binary.
pub struct LoadSegment {
	vaddr: u64,
	offset: u64,
	filesz: u64,
	memsz: u64,
	flags: u32,
}

/// Structure representing a ELF binary.
pub struct ElfImage {
	/// Entry point of the ELF binary.
	pub entry: Elf64Addr,
	/// PT_LOAD Segments of the ELF binary.
	pub segments: Vec<LoadSegment>
}

/// Parse an ELF file.
pub fn parse_elf(bytes: &[u8]) -> Result<ElfImage, NullexError> {
	if bytes.len() < core::mem::size_of::<Elf64Ehdr>() {
		return Err(NullexError::ElfMagicIncorrect);
	}

	let e_header = unsafe { &*(bytes.as_ptr() as *const Elf64Ehdr) };

	if e_header.e_ident[0..4] != ELF_MAGIC {
		println!("invalid elf magic number");
		return Err(NullexError::ElfMagicIncorrect);
	}

	let mut load_segs: Vec<LoadSegment> = Vec::new();

	for i in 0..e_header.e_phnum {
		let start = (e_header.e_phoff + (i * e_header.e_phentsize) as u64) as usize;
		let end = start + e_header.e_phentsize as usize;

		if end <= bytes.len() {
			let mut phdr = Elf64Phdr::default();
			let phdr_size = core::mem::size_of::<Elf64Phdr>();

			unsafe {
				core::ptr::copy_nonoverlapping(
					bytes.as_ptr().add(start),
					&mut phdr as *mut Elf64Phdr as *mut u8,
					phdr_size.min(e_header.e_phentsize as usize),
				);
			}

			if phdr.p_type == PT_LOAD {
				load_segs.push(LoadSegment {
					vaddr: phdr.p_vaddr,
					offset: phdr.p_offset,
					filesz: phdr.p_filesz,
					memsz: phdr.p_memsz,
					flags: phdr.p_flags,
				});
			}
		}
	}

	Ok(ElfImage {
		entry: e_header.e_entry,
		segments: load_segs,
	})
}

/// Parse ELF command for the kernel.
pub fn pelf(args: &[&str]) {
	if args.is_empty() {
		println!("pelf: missing file.");
		return;
	}

	let path = resolve_path(args[0]);

	let process = fs::with_fs(|fs| {
		match fs.read_file(path.as_str()) {
			Ok(bytes) => spawn_user_process(bytes, args, &[""]),
			Err(_) => {
				println!("pelf: file not found: {}", args[0]);
				return Err(NullexError::FileNotFound);
			}
		}
	});

	match process {
		Ok(proc) => {
			serial_println!("[INFO] Entering User Process..");

			unsafe {
				enter_user_process(&proc);
			}

			let code = crate::arch::x86_64::user::USER_EXIT_CODE
				.load(core::sync::atomic::Ordering::SeqCst);
			println!("Process exited with code {}", code);
		}
		Err(_) => println!("pelf: failed to spawn process"),
	}
}

/// Load a ELF binary segment into memory.
pub fn load_segment(
    address_space: &mut AddressSpace,
    elf_bytes: &[u8],
    seg: &LoadSegment,
) -> Result<(), NullexError> {
    if seg.memsz == 0 {
        return Ok(());
    }

    let file_start = seg.offset as usize;
    let file_end = seg
        .offset
        .checked_add(seg.filesz)
        .ok_or(NullexError::ElfMagicIncorrect)? as usize;

    if file_end > elf_bytes.len() {
        return Err(NullexError::ElfMagicIncorrect);
    }

    let seg_start = seg.vaddr as usize;
    let seg_end = seg_start
        .checked_add(seg.memsz as usize)
        .ok_or(NullexError::ElfMagicIncorrect)?;

    let start_page = Page::containing_address(VirtAddr::new(seg_start as u64));
    let end_page = Page::containing_address(VirtAddr::new((seg_end - 1) as u64));

    let mut fa_guard = ALLOCATOR_INFO.frame_allocator.lock();
    let fa = fa_guard
        .as_mut()
        .ok_or(NullexError::FrameAllocatorNotInitialized)?;

    let table_ptr = unsafe { phys_to_virt(address_space.page_table.start_address()) };
    let pml4 = unsafe { &mut *table_ptr.as_mut_ptr::<PageTable>() };
    let mut mapper = unsafe { OffsetPageTable::new(pml4, *crate::PHYS_MEM_OFFSET.lock()) };

    let mut flags = PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE;

    if seg.flags & PF_W != 0 {
        flags |= PageTableFlags::WRITABLE;
    }

    if seg.flags & PF_X == 0 {
        flags |= PageTableFlags::NO_EXECUTE;
    }

	serial_println!(
		"segment vaddr={:#x} memsz={:#x} filesz={:#x} pages={}",
		seg.vaddr,
		seg.memsz,
		seg.filesz,
		(seg.memsz + 0xFFF) / 0x1000
	);

    for page in Page::range_inclusive(start_page, end_page) {
        let frame = fa.allocate_frame().ok_or(NullexError::FrameAllocationFailed)?;

        match unsafe { mapper.map_to(page, frame, flags, *fa) } {
			Ok(flush) => flush.flush(),
			Err(e) => {
				serial_println!("map_to failed: {:?}", e);
				return Err(NullexError::FrameAllocationFailed);
			}
		}

        let frame_virt = unsafe { phys_to_virt(frame.start_address()).as_mut_ptr::<u8>() };

        unsafe {
            write_bytes(frame_virt, 0, 4096);
        }

        let page_start = page.start_address().as_u64() as usize;
        let page_end = page_start + 4096;

        let copy_start = core::cmp::max(seg_start, page_start);
        let copy_end = core::cmp::min(seg_end, page_end);

        if copy_start < copy_end {
            let src_off = file_start + (copy_start - seg_start);
            let dst_off = copy_start - page_start;
            let len = copy_end - copy_start;

            unsafe {
                copy_nonoverlapping(
                    elf_bytes.as_ptr().add(src_off),
                    frame_virt.add(dst_off),
                    len,
                );
            }
        }
    }

    Ok(())
}