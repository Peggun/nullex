//!
//! elf.rs
//!
//! ELF binary helpers for the kernel.

// https://codebrowser.dev/linux/include/elf.h.html
// to be split further later.

use alloc::vec::Vec;
use core::ptr::{copy_nonoverlapping, write_bytes};

use x86_64::{
	VirtAddr,
	structures::paging::{
		FrameAllocator,
		Mapper,
		OffsetPageTable,
		Page,
		PageTable,
		PageTableFlags
	}
};

use crate::{
	allocator::ALLOCATOR_INFO,
	error::NullexError,
	memory::phys_to_virt,
	println,
	serial_println,
	task::address_space::AddressSpace
};

const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];

const EI_NIDENT: usize = 16;

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
	e_shstrrndx: ElfHalf
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
	p_align: ElfXword
}
#[repr(C)]
/// A PT_LOAD Segment of a ELF binary.
pub struct LoadSegment {
	vaddr: u64,
	offset: u64,
	filesz: u64,
	memsz: u64,
	flags: u32
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
					phdr_size.min(e_header.e_phentsize as usize)
				);
			}

			if phdr.p_type == PT_LOAD {
				load_segs.push(LoadSegment {
					vaddr: phdr.p_vaddr,
					offset: phdr.p_offset,
					filesz: phdr.p_filesz,
					memsz: phdr.p_memsz,
					flags: phdr.p_flags
				});
			}
		}
	}

	Ok(ElfImage {
		entry: e_header.e_entry,
		segments: load_segs
	})
}

/// Parse ELF command for the kernel.
// pub fn pelf(args: &[&str]) {
// 	if args.is_empty() {
// 		println!("pelf: missing file.");
// 		return;
// 	}

// 	let path = resolve_path(args[0]);

// 	let process = fs::with_fs(|fs| match fs.read_file(path.as_str()) {
// 		Ok(bytes) => spawn_user_process(bytes, args, &[""]),
// 		Err(_) => {
// 			println!("pelf: file not found: {}", args[0]);
// 			return Err(NullexError::FileNotFound);
// 		}
// 	});

// 	match process {
// 		Ok(proc) => {
// 			serial_println!("[INFO] Entering User Process..");

// 			unsafe {
// 				enter_user_process(&proc);
// 			}

// 			let code = crate::arch::x86_64::user::USER_EXIT_CODE
// 				.load(core::sync::atomic::Ordering::SeqCst);
// 			println!("Process exited with code {}", code);
// 		}
// 		Err(_) => println!("pelf: failed to spawn process")
// 	}
// }

/// Load a ELF binary segment into memory.
// change errors to replace the ElfMagic ones with their own.
pub fn load_segment(
	address_space: &mut AddressSpace,
	elf_bytes: &[u8],
	seg: &LoadSegment
) -> Result<(), NullexError> {
	if seg.memsz == 0 {
		return Ok(());
	}

	if seg.filesz > seg.memsz {
		return Err(NullexError::ElfMagicIncorrect);
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

	let file_seg_end = seg_start
		.checked_add(seg.filesz as usize)
		.ok_or(NullexError::ElfMagicIncorrect)?;

	let start_page = Page::containing_address(VirtAddr::new(seg_start as u64));

	let end_page = Page::containing_address(VirtAddr::new((seg_end - 1) as u64));

	let mut fa_guard = ALLOCATOR_INFO.frame_allocator.lock();

	let fa_ref = fa_guard
		.as_mut()
		.ok_or(NullexError::FrameAllocatorNotInitialized)?;

	let fa = &mut **fa_ref;

	let table_ptr = unsafe { phys_to_virt(address_space.page_table.start_address()) };

	let pml4 = unsafe { &mut *table_ptr.as_mut_ptr::<PageTable>() };

	let mut mapper =
		unsafe { OffsetPageTable::new(pml4, VirtAddr::new(crate::memory::PHYS_MEM_OFFSET)) };

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
		let frame = fa
			.allocate_frame()
			.ok_or(NullexError::FrameAllocationFailed)?;

		match unsafe { mapper.map_to(page, frame, flags, fa) } {
			Ok(flush) => flush.flush(),

			Err(e) => {
				serial_println!("map_to failed: {:?}", e);

				return Err(NullexError::FrameAllocationFailed);
			}
		}

		address_space.owned_frames.push(frame);

		let frame_virt = unsafe { phys_to_virt(frame.start_address()).as_mut_ptr::<u8>() };

		// zeros bss to fix malloc errors in libnullex C
		unsafe {
			write_bytes(frame_virt, 0, 4096);
		}

		let page_start = page.start_address().as_u64() as usize;

		let page_end = page_start + 4096;

		let copy_start = core::cmp::max(seg_start, page_start);

		let copy_end = core::cmp::min(file_seg_end, page_end);

		if copy_start < copy_end {
			let src_off = file_start + (copy_start - seg_start);

			let dst_off = copy_start - page_start;

			let len = copy_end - copy_start;

			unsafe {
				copy_nonoverlapping(
					elf_bytes.as_ptr().add(src_off),
					frame_virt.add(dst_off),
					len
				);
			}
		}
	}

	Ok(())
}

#[cfg(feature = "test")]
pub mod tests {
	use alloc::{string::ToString, vec::Vec};

	use crate::{
		error::NullexError,
		process::elf::{Elf64Ehdr, Elf64Phdr, load_segment, parse_elf},
		tassert,
		tassert_eq,
		testing::ktest::TestError
	};

	#[repr(C)]
	struct MockEhdr {
		e_ident: [u8; 16],
		e_type: u16,
		e_machine: u16,
		e_version: u32,
		e_entry: u64,
		e_phoff: u64,
		e_shoff: u64,
		e_flags: u32,
		e_ehsize: u16,
		e_phentsize: u16,
		e_phnum: u16,
		e_shentsize: u16,
		e_shnum: u16,
		e_shstrndx: u16
	}

	#[repr(C)]
	struct MockPhdr {
		p_type: u32,
		p_flags: u32,
		p_offset: u64,
		p_vaddr: u64,
		p_paddr: u64,
		p_filesz: u64,
		p_memsz: u64,
		p_align: u64
	}

	fn build_elf_bytes(phdrs: &[(u32, u64, u64, u64, u64, u32)]) -> Vec<u8> {
		let ehdr_size = core::mem::size_of::<Elf64Ehdr>();
		let phdr_size = core::mem::size_of::<Elf64Phdr>();
		let mut bytes = vec![0u8; ehdr_size];

		let mut mock_ehdr = MockEhdr {
			e_ident: [0; 16],
			e_type: 2,
			e_machine: 0x3E,
			e_version: 1,
			e_entry: 0x400000,
			e_phoff: ehdr_size as u64,
			e_shoff: 0,
			e_flags: 0,
			e_ehsize: ehdr_size as u16,
			e_phentsize: phdr_size as u16,
			e_phnum: phdrs.len() as u16,
			e_shentsize: 0,
			e_shnum: 0,
			e_shstrndx: 0
		};
		mock_ehdr.e_ident[0..4].copy_from_slice(&[0x7f, b'E', b'L', b'F']);

		unsafe {
			core::ptr::copy_nonoverlapping(
				&mock_ehdr as *const MockEhdr as *const u8,
				bytes.as_mut_ptr(),
				ehdr_size
			);
		}

		for &(p_type, p_offset, p_vaddr, p_filesz, p_memsz, p_flags) in phdrs {
			let mock_phdr = MockPhdr {
				p_type,
				p_flags,
				p_offset,
				p_vaddr,
				p_paddr: 0,
				p_filesz,
				p_memsz,
				p_align: 0x1000
			};
			let phdr_bytes = unsafe {
				core::slice::from_raw_parts(&mock_phdr as *const MockPhdr as *const u8, phdr_size)
			};
			bytes.extend_from_slice(phdr_bytes);
		}

		bytes
	}

	pub fn test_parse_elf_buffer_too_small() -> Result<(), TestError> {
		let bytes = [0u8; 10];
		let res = parse_elf(&bytes);
		tassert!(matches!(res, Err(NullexError::ElfMagicIncorrect)));
		Ok(())
	}
	crate::create_test!(test_parse_elf_buffer_too_small);

	pub fn test_parse_elf_invalid_magic() -> Result<(), TestError> {
		let mut bytes = build_elf_bytes(&[]);
		bytes[0] = 0x00;
		let res = parse_elf(&bytes);
		tassert!(matches!(res, Err(NullexError::ElfMagicIncorrect)));
		Ok(())
	}
	crate::create_test!(test_parse_elf_invalid_magic);

	pub fn test_parse_elf_valid_no_segments() -> Result<(), TestError> {
		let bytes = build_elf_bytes(&[]);
		let res = parse_elf(&bytes).unwrap();
		tassert_eq!(res.entry, 0x400000);
		tassert!(res.segments.is_empty());
		Ok(())
	}
	crate::create_test!(test_parse_elf_valid_no_segments);

	pub fn test_parse_elf_valid_with_load_segment() -> Result<(), TestError> {
		let bytes = build_elf_bytes(&[(1, 0, 0x400000, 100, 100, 0x5)]);
		let res = parse_elf(&bytes).unwrap();
		tassert_eq!(res.segments.len(), 1);
		Ok(())
	}
	crate::create_test!(test_parse_elf_valid_with_load_segment);

	pub fn test_parse_elf_ignores_non_load_segments() -> Result<(), TestError> {
		let bytes = build_elf_bytes(&[
			(0, 0, 0, 0, 0, 0),
			(1, 0, 0x400000, 100, 100, 0x5),
			(2, 0, 0, 0, 0, 0)
		]);
		let res = parse_elf(&bytes).unwrap();
		tassert_eq!(res.segments.len(), 1);
		Ok(())
	}
	crate::create_test!(test_parse_elf_ignores_non_load_segments);

	pub fn test_parse_elf_truncated_program_headers() -> Result<(), TestError> {
		let mut bytes = build_elf_bytes(&[(1, 0, 0x400000, 100, 100, 0x5)]);
		let ehdr = unsafe { &mut *(bytes.as_mut_ptr() as *mut MockEhdr) };
		ehdr.e_phnum = 2;
		let res = parse_elf(&bytes).unwrap();
		tassert_eq!(res.segments.len(), 1);
		Ok(())
	}
	crate::create_test!(test_parse_elf_truncated_program_headers);

	pub fn test_load_segment_zero_memsz() -> Result<(), TestError> {
		let bytes = build_elf_bytes(&[(1, 0, 0x400000, 0, 0, 0x5)]);
		let image = parse_elf(&bytes).unwrap();
		let seg = &image.segments[0];

		let mut dummy_addr_space =
			core::mem::MaybeUninit::<crate::task::address_space::AddressSpace>::uninit();
		let addr_space = unsafe { dummy_addr_space.assume_init_mut() };

		let res = load_segment(addr_space, &bytes, seg);
		tassert!(res.is_ok());
		Ok(())
	}
	crate::create_test!(test_load_segment_zero_memsz);

	pub fn test_load_segment_filesz_greater_than_memsz() -> Result<(), TestError> {
		let bytes = build_elf_bytes(&[(1, 0, 0x400000, 100, 50, 0x5)]);
		let image = parse_elf(&bytes).unwrap();
		let seg = &image.segments[0];

		let mut dummy_addr_space =
			core::mem::MaybeUninit::<crate::task::address_space::AddressSpace>::uninit();
		let addr_space = unsafe { dummy_addr_space.assume_init_mut() };

		let res = load_segment(addr_space, &bytes, seg);
		tassert!(matches!(res, Err(NullexError::ElfMagicIncorrect)));
		Ok(())
	}
	crate::create_test!(test_load_segment_filesz_greater_than_memsz);

	pub fn test_load_segment_file_end_out_of_bounds() -> Result<(), TestError> {
		let bytes = build_elf_bytes(&[(1, 1000, 0x400000, 100, 100, 0x5)]);
		let image = parse_elf(&bytes).unwrap();
		let seg = &image.segments[0];

		let mut dummy_addr_space =
			core::mem::MaybeUninit::<crate::task::address_space::AddressSpace>::uninit();
		let addr_space = unsafe { dummy_addr_space.assume_init_mut() };

		let res = load_segment(addr_space, &bytes, seg);
		tassert!(matches!(res, Err(NullexError::ElfMagicIncorrect)));
		Ok(())
	}
	crate::create_test!(test_load_segment_file_end_out_of_bounds);

	pub fn test_load_segment_vaddr_overflow() -> Result<(), TestError> {
		let bytes = build_elf_bytes(&[(1, 0, u64::MAX, 100, 100, 0x5)]);
		let image = parse_elf(&bytes).unwrap();
		let seg = &image.segments[0];

		let mut dummy_addr_space =
			core::mem::MaybeUninit::<crate::task::address_space::AddressSpace>::uninit();
		let addr_space = unsafe { dummy_addr_space.assume_init_mut() };

		let res = load_segment(addr_space, &bytes, seg);
		tassert!(matches!(res, Err(NullexError::ElfMagicIncorrect)));
		Ok(())
	}
	crate::create_test!(test_load_segment_vaddr_overflow);

	pub fn test_load_segment_filesz_overflow() -> Result<(), TestError> {
		let bytes = build_elf_bytes(&[(1, 0, 0x400000, u64::MAX, u64::MAX, 0x5)]);
		let image = parse_elf(&bytes).unwrap();
		let seg = &image.segments[0];

		let mut dummy_addr_space =
			core::mem::MaybeUninit::<crate::task::address_space::AddressSpace>::uninit();
		let addr_space = unsafe { dummy_addr_space.assume_init_mut() };

		let res = load_segment(addr_space, &bytes, seg);
		tassert!(matches!(res, Err(NullexError::ElfMagicIncorrect)));
		Ok(())
	}
	crate::create_test!(test_load_segment_filesz_overflow);

	pub fn test_load_segment_offset_plus_filesz_overflow() -> Result<(), TestError> {
		let bytes = build_elf_bytes(&[(1, u64::MAX, 0x400000, 100, 100, 0x5)]);
		let image = parse_elf(&bytes).unwrap();
		let seg = &image.segments[0];

		let mut dummy_addr_space =
			core::mem::MaybeUninit::<crate::task::address_space::AddressSpace>::uninit();
		let addr_space = unsafe { dummy_addr_space.assume_init_mut() };

		let res = load_segment(addr_space, &bytes, seg);
		tassert!(matches!(res, Err(NullexError::ElfMagicIncorrect)));
		Ok(())
	}
	crate::create_test!(test_load_segment_offset_plus_filesz_overflow);
}
