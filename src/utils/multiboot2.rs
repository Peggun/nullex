//!
//! multiboot2.rs
//! 
//! Multiboot2 module for the kernel.
//! 

// https://cgit.git.savannah.gnu.org/cgit/grub.git/tree/doc/multiboot2.h?h=multiboot2
// https://cgit.git.savannah.gnu.org/cgit/grub.git/tree/doc/kernel.c?h=multiboot2

use core::{ptr::read_unaligned, u64};

use x86_64::PhysAddr;

use crate::{
	acpi::RSDT,
	arch::x86_64::bootinfo::{FrameRange, MemoryMap, MemoryRegion, MemoryRegionType},
	memory::phys_to_virt,
	println,
	serial_println
};

const MULTIBOOT_SEARCH: u32 = 32768;
const MULTIBOOT_HEADER_ALIGN: u32 = 8;

const MULTIBOOT2_HEADER_MAGIC: u32 = 0xe85250d6;
const MULTIBOOT2_BOOTLOADER_MAGIC: u32 = 0x36d76289; // not needed, boot.asm does the check
const MULTIBOOT_MOD_ALIGN: u32 = 0x00001000;
const MULTIBOOT_INFO_ALIGN: u32 = 0x00000008;

const MULTIBOOT_TAG_ALIGN: u32 = 8;
const MULTIBOOT_TAG_TYPE_END: u32 = 0;
const MULTIBOOT_TAG_TYPE_CMDLINE: u32 = 1;
const MULTIBOOT_TAG_TYPE_BOOT_LOADER_NAME: u32 = 2;
const MULTIBOOT_TAG_TYPE_MODULE: u32 = 3;
const MULTIBOOT_TAG_TYPE_BASIC_MEMINFO: u32 = 4;
const MULTIBOOT_TAG_TYPE_BOOTDEV: u32 = 5;
const MULTIBOOT_TAG_TYPE_MMAP: u32 = 6;
const MULTIBOOT_TAG_TYPE_VBE: u32 = 7;
const MULTIBOOT_TAG_TYPE_FRAMEBUFFER: u32 = 8;
const MULTIBOOT_TAG_TYPE_ELF_SECTIONS: u32 = 9;
const MULTIBOOT_TAG_TYPE_APM: u32 = 10;
const MULTIBOOT_TAG_TYPE_EFI32: u32 = 11;
const MULTIBOOT_TAG_TYPE_EFI64: u32 = 12;
const MULTIBOOT_TAG_TYPE_SMBIOS: u32 = 13;
const MULTIBOOT_TAG_TYPE_ACPI_OLD: u32 = 14;
const MULTIBOOT_TAG_TYPE_ACPI_NEW: u32 = 15;
const MULTIBOOT_TAG_TYPE_NETWORK: u32 = 16;
const MULTIBOOT_TAG_TYPE_EFI_MMAP: u32 = 17;
const MULTIBOOT_TAG_TYPE_EFI_BS: u32 = 18;
const MULTIBOOT_TAG_TYPE_EFI32_IH: u32 = 19;
const MULTIBOOT_TAG_TYPE_EFI64_IH: u32 = 20;
const MULTIBOOT_TAG_TYPE_LOAD_BASE_ADDR: u32 = 21;

const MULTIBOOT_HEADER_TAG_END: u32 = 0;
const MULTIBOOT_HEADER_TAG_INFOMATION_REQUEST: u32 = 1;
const MULTIBOOT_HEADER_TAG_ADDRESS: u32 = 2;
const MULTIBOOT_HEADER_TAG_ENTRY_ADDRESS: u32 = 3;
const MULTIBOOT_HEADER_TAG_CONSOLE_FLAGS: u32 = 4;
const MULTIBOOT_HEADER_TAG_FRAMEBUFFER: u32 = 5;
const MULTIBOOT_HEADER_TAG_MODULE_ALIGN: u32 = 6;
const MULTIBOOT_HEADER_TAG_EFI_BS: u32 = 7;
const MULTIBOOT_HEADER_TAG_ENTRY_ADDRESS_EFI32: u32 = 8;
const MULTIBOOT_HEADER_TAG_ENTRY_ADDRESS_EFI64: u32 = 9;
const MULTIBOOT_HEADER_TAG_RELOCATABLE: u32 = 10;

const MULTIBOOT_ARCHITECTURE_I386: u32 = 0;
const MULTIBOOT_ARCHITECTURE_MIPS32: u32 = 4;
const MULTIBOOT_HEADER_TAG_OPIONAL: u32 = 1;

const MULTIBOOT_LOAD_PREFERENCE_NONE: u32 = 0;
const MULTIBOOT_LOAD_PREFERENCE_LOW: u32 = 1;
const MULTIBOOT_LOAD_PREFERENCE_HIGH: u32 = 4;

const MULTIBOOT_CONSOLE_FLAGS_CONSOLE_REQUIRED: u32 = 1;
const MULTIBOOT_CONSOLE_FLAGS_EGA_TEXT_SUPPORTED: u32 = 2;

const MULTIBOOT_MEMORY_AVAILABLE: u32 = 1;
const MULTIBOOT_MEMORY_RESERVED: u32 = 2;
const MULTIBOOT_MEMORY_ACPI_RECLAIMABLE: u32 = 3;
const MULTIBOOT_MEMORY_NVS: u32 = 4;
const MULTIBOOT_MEMORY_BADRAM: u32 = 5;

const MULTIBOOT_FRAMEBUFFER_TYPE_INDEXED: u8 = 0;
const MULTIBOOT_FRAMEBUFFER_TYPE_RGB: u8 = 1;
const MULTIBOOT_FRAMEBUFFER_TYPE_EGA_TEXT: u8 = 2;

#[repr(C)]
#[derive(Debug)]
struct MultibootHeader {
	magic: u32,
	architecture: u32,
	header_length: u32,
	checksum: u32
}

#[repr(C)]
#[derive(Debug)]
struct MultibootHeaderTag {
	r#type: u16,
	flags: u16,
	size: u32
}

#[repr(C)]
#[derive(Debug)]
struct MultibootHeaderTagInformationRequest {
	r#type: u16,
	flags: u16,
	size: u32,
	requests: [u32; 0]
}

#[repr(C)]
#[derive(Debug)]
struct MultibootHeaderTagAddress {
	r#type: u16,
	flags: u16,
	size: u32,
	header_addr: u32,
	load_addr: u32,
	load_end_addr: u32,
	bss_end_addr: u32
}

#[repr(C)]
#[derive(Debug)]
struct MultibootHeaderTagEntryAddress {
	r#type: u16,
	flags: u16,
	size: u32,
	entry_addr: u32
}

#[repr(C)]
#[derive(Debug)]
struct MultibootHeaderTagConsoleFlags {
	r#type: u16,
	flags: u16,
	size: u32,
	console_flags: u32
}

#[repr(C)]
#[derive(Debug)]
struct MultibootHeaderTagFramebuffer {
	r#type: u16,
	flags: u16,
	size: u32,
	width: u32,
	height: u32,
	depth: u32
}

#[repr(C)]
#[derive(Debug)]
struct MultibootHeaderTagModuleAlign {
	r#type: u16,
	flags: u16,
	size: u32
}

#[repr(C)]
#[derive(Debug)]
struct MultibootHeaderTagRelocatable {
	r#type: u16,
	flags: u16,
	size: u32,
	min_addr: u32,
	max_addr: u32,
	align: u32,
	preference: u32
}

#[repr(C)]
#[derive(Debug)]
struct MultibootColour {
	red: u8,
	green: u8,
	blue: u8
}

#[repr(C)]
#[derive(Debug)]
struct MultibootMmapEntry {
	addr: u64,
	len: u64,

	// defines are above
	r#type: u32,
	zero: u32
}
type MultibootMemoryMap = MultibootMmapEntry;

#[repr(C)]
#[derive(Debug)]
struct MultibootTag {
	r#type: u32,
	size: u32
}

#[repr(C)]
#[derive(Debug)]
struct MultibootTagString {
	r#type: u32,
	size: u32,
	string: [u8; 0]
}

#[repr(C)]
#[derive(Debug)]
struct MultibootTagModule {
	r#type: u32,
	size: u32,
	mod_start: u32,
	mod_end: u32,
	cmdline: [u8; 0]
}

#[repr(C)]
#[derive(Debug)]
struct MultibootTagBasicMemInfo {
	r#type: u32,
	size: u32,
	mem_lower: u32,
	mem_upper: u32
}

#[repr(C)]
#[derive(Debug)]
struct MultibootTagBootDev {
	r#type: u32,
	size: u32,
	biosdev: u32,
	slice: u32,
	part: u32
}

#[repr(C)]
#[derive(Debug)]
struct MultibootTagMmap {
	r#type: u32,
	size: u32,
	entry_size: u32,
	entry_version: u32,
	entries: [MultibootMmapEntry; 0]
}

#[repr(C)]
#[derive(Debug)]
struct MultibootVbeInfoBlock {
	external_specification: [u8; 512]
}

#[repr(C)]
#[derive(Debug)]
struct MultibootVbeModeInfoBlock {
	external_specification: [u8; 256]
}

#[repr(C)]
#[derive(Debug)]
struct MultibootTagVbe {
	r#type: u32,
	size: u32,

	vbe_mode: u16,
	vbe_interface_seg: u16,
	vbe_interface_off: u16,
	vbe_interface_len: u16,

	vbe_control_info: MultibootVbeInfoBlock,
	vbe_mode_info: MultibootVbeModeInfoBlock
}

#[repr(C)]
#[derive(Debug)]
struct MultibootTagFramebufferCommon {
	r#type: u32,
	size: u32,

	framebuffer_addr: u64,
	framebuffer_pitch: u32,
	framebuffer_width: u32,
	framebuffer_height: u32,
	framebuffer_bpp: u8,

	// defines are above
	framebuffer_type: u8,
	reserved: u16
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct FramebufferPalette {
	framebuffer_palette_num_colors: u16,
	framebuffer_palette: *const MultibootColour
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct FramebufferRgbFields {
	framebuffer_red_field_position: u8,
	framebuffer_red_mask_size: u8,
	framebuffer_green_field_position: u8,
	framebuffer_green_mask_size: u8,
	framebuffer_blue_field_position: u8,
	framebuffer_blue_mask_size: u8
}

#[repr(C)]
union FramebufferDetails {
	palette: FramebufferPalette,
	rgb_fields: FramebufferRgbFields
}

#[repr(C)]
struct MultibootTagFramebuffer {
	common: MultibootTagFramebufferCommon,
	details: FramebufferDetails
}

#[repr(C)]
#[derive(Debug)]
struct MultibootTagElfSections {
	r#type: u32,
	size: u32,
	num: u32,
	entsize: u32,
	shndx: u32,
	sections: [u8; 0]
}

#[repr(C)]
#[derive(Debug)]
struct MultibootTagApm {
	r#type: u32,
	size: u32,
	version: u16,
	cseg: u16,
	offset: u32,
	cseg_16: u16,
	dseg: u16,
	flags: u16,
	cseg_len: u16,
	cseg_16_len: u16,
	dseg_len: u16
}

#[repr(C)]
#[derive(Debug)]
struct MultibootTagEfi32 {
	r#type: u32,
	size: u32,
	pointer: u32
}

#[repr(C)]
#[derive(Debug)]
struct MultibootTagEfi64 {
	r#type: u32,
	size: u32,
	pointer: u64
}

#[repr(C)]
#[derive(Debug)]
struct MultibootTagSmbios {
	r#type: u32,
	size: u32,
	major: u8,
	minor: u8,
	reserved: [u8; 6],
	tables: [u8; 0]
}

#[repr(C)]
#[derive(Debug)]
struct MultibootTagOldAcpi {
	r#type: u32,
	size: u32,
	signature: [u8; 8],
	checksum: u8,
	oem_id: [u8; 6],
	rev: u8,
	rsdt_address: u32 // phys_addr
}

#[repr(C)]
#[derive(Debug)]
struct MultibootTagNewAcpi {
	r#type: u32,
	size: u32,
	rsdp: [u8; 0]
}

#[repr(C)]
#[derive(Debug)]
struct MultibootTagEfiMmap {
	r#type: u32,
	size: u32,
	descr_size: u32,
	descr_vers: u32,
	efi_mmap: [u8; 0]
}

#[repr(C)]
#[derive(Debug)]
struct MultibootTagEfi32IH {
	r#type: u32,
	size: u32,
	pointer: u32
}

#[repr(C)]
#[derive(Debug)]
struct MultibootTagEfi64IH {
	r#type: u32,
	size: u32,
	pointer: u64
}

#[repr(C)]
#[derive(Debug)]
struct MultibootTagLoadBaseAddr {
	r#type: u32,
	size: u32,
	load_base_addr: u32
}

#[repr(C)]
#[derive(Debug)]
struct MultibootInfoHeader {
	total_size: u32,
	reserved: u32
}

/// Structure representing the boot-time information 
/// provided to us by Multiboot2
pub struct BootInformation {
	/// The kernel's physical memory offset.
	pub physical_memory_offset: usize,
	/// The kernel's memory map
	pub memory_map: MemoryMap,

	/// The Root System Description Pointer
	pub rsdp: usize
}

impl BootInformation {
	fn new() -> Self {
		Self {
			physical_memory_offset: 0,
			memory_map: MemoryMap::new(),
			rsdp: 0
		}
	}
}

// linker symbols
unsafe extern "C" {
	/// .text address in linker.ld for specified architecture
	pub unsafe static __text_addr: u8;
	/// The physical address based on which this kernel is linked on
	pub unsafe static __link_phys_base: u8;
	/// The end of the kernel's `.bin` or `.iso` file.
	pub unsafe static _end: u8;
}

/// Parses multiboot2 information and returns a `BootInformation`
/// # Safety
/// - Requires the `mbi_addr` to point to proper, mapped memory.
pub unsafe fn parse_multiboot2(mbi_addr: usize) -> BootInformation {
	unsafe {
		if (mbi_addr & 7) == 1 {
			panic!("Unaligned mbi: 0x{:X}", mbi_addr)
		}

		let size = *(mbi_addr as *const u32);
		println!("MBI Size: 0x{:x}", size);

		let mut tag = (mbi_addr as *const u8).add(8) as *const MultibootTag;

		let mut bi = BootInformation::new(); // empty

		while (*tag).r#type != MULTIBOOT_TAG_TYPE_END {
			println!("Tag: 0x{:X}, Size: {:X}", (*tag).r#type, (*tag).size);
			match (*tag).r#type {
				MULTIBOOT_TAG_TYPE_CMDLINE => {
					let str = tag as *const MultibootTagString;
					println!("Command line = {:?}", (*str).string)
				}
				MULTIBOOT_TAG_TYPE_BOOT_LOADER_NAME => {
					let str = tag as *const MultibootTagString;
					println!("Boot loader Name = {:?}", (*str).string)
				}
				MULTIBOOT_TAG_TYPE_MODULE => {
					let module = tag as *const MultibootTagModule;
					println!(
						"Module at 0x{:X}-0x{:X}. Command line {:?}",
						(*module).mod_start,
						(*module).mod_end,
						(*module).cmdline
					);
				}
				MULTIBOOT_TAG_TYPE_BASIC_MEMINFO => {
					let mem = tag as *const MultibootTagBasicMemInfo;
					println!(
						"mem_lower = {:?}, mem_upper = {:?}",
						(*mem).mem_lower,
						(*mem).mem_upper
					);
				}
				MULTIBOOT_TAG_TYPE_MMAP => {
					let tag_mmap = &*(tag as *const MultibootTagMmap);
					println!("mmap");
					let mut bl_map = MemoryMap::new();

					let mut entry_ptr = tag_mmap.entries.as_ptr();
					let end = (tag as *const u8).wrapping_add(tag_mmap.size as usize);

					if tag_mmap.entry_size == 0 {
						// avoid infinite loop
						break;
					}

					while (entry_ptr as *const u8) < end {
						let mmap_entry = &*entry_ptr;

						let base = mmap_entry.addr;
						let len = mmap_entry.len;

						let region_kind = match mmap_entry.r#type {
							MULTIBOOT_MEMORY_AVAILABLE => MemoryRegionType::Usable,
							MULTIBOOT_MEMORY_RESERVED => MemoryRegionType::Reserved,
							MULTIBOOT_MEMORY_ACPI_RECLAIMABLE => MemoryRegionType::AcpiReclaimable,
							MULTIBOOT_MEMORY_NVS => MemoryRegionType::AcpiNvs,
							MULTIBOOT_MEMORY_BADRAM => MemoryRegionType::BadMemory,
							_ => MemoryRegionType::Reserved
						};

						bl_map.add_region(MemoryRegion {
							range: FrameRange::new(base, len),
							region_type: region_kind
						});

						entry_ptr = ((entry_ptr as *const u8)
							.wrapping_add(tag_mmap.entry_size as usize))
							as *const MultibootMemoryMap;
					}

					bi.memory_map = bl_map;
				}
				MULTIBOOT_TAG_TYPE_FRAMEBUFFER => {
					let tagfb = tag as *const MultibootTagFramebuffer;
					let fb = (*tagfb).common.framebuffer_addr as *const u8;
					let mut colour: u32 = 0;
					let mut i: u32 = 0;

					match (*tagfb).common.framebuffer_type {
						MULTIBOOT_FRAMEBUFFER_TYPE_INDEXED => {
							let palette_info = (*tagfb).details.palette;
							let num = palette_info.framebuffer_palette_num_colors as usize;
							let pal_ptr = palette_info.framebuffer_palette;

							if !pal_ptr.is_null() && num > 0 {
								let mut best_distance: u32 = u32::MAX;
								let mut best_index: usize = 0;

								for idx in 0..num {
									let col: &MultibootColour = &*pal_ptr.add(idx);
									println!(
										"palette[{}] = r={} g={} b={}",
										idx, col.red, col.green, col.blue
									);

									// distance = (0xff - palette[i].blue) * (0xff -
									// palette[i].blue)
									//          + palette[i].red * palette[i].red
									//          + palette[i].green * palette[i].green;
									let b_inv = 0xffu32 - (col.blue as u32);
									let r = col.red as u32;
									let g = col.green as u32;
									let distance = b_inv
										.wrapping_mul(b_inv)
										.wrapping_add(r.wrapping_mul(r))
										.wrapping_add(g.wrapping_mul(g));

									if distance < best_distance {
										best_distance = distance;
										best_index = idx;
									}
								}

								colour = best_index as u32;
							}
						}
						MULTIBOOT_FRAMEBUFFER_TYPE_RGB => {
							colour =
								((1 << (*tagfb).details.rgb_fields.framebuffer_blue_mask_size) - 1)
									<< (*tagfb).details.rgb_fields.framebuffer_blue_field_position;
						}
						MULTIBOOT_FRAMEBUFFER_TYPE_EGA_TEXT => {
							colour = '\\' as u32 | 0x0100;
						}
						_ => {
							colour = 0xffffffff;
						}
					}

					// loops through width and height, could use two for loops, the kernel.c code
					// says otherwise.
					while i < (*tagfb).common.framebuffer_width
						&& i < (*tagfb).common.framebuffer_height
					{
						match (*tagfb).common.framebuffer_bpp {
							8 => {
								let pixel = fb.wrapping_add(
									((*tagfb).common.framebuffer_pitch * i + i)
										.try_into()
										.unwrap()
								) as *mut u8;
								*pixel = colour as u8;
							}
							16 => {
								let pixel = fb.wrapping_add(
									((*tagfb).common.framebuffer_pitch * i + 2 * i)
										.try_into()
										.unwrap()
								) as *mut u16;
								*pixel = colour as u16;
							}
							24 => {
								let pixel = fb.wrapping_add(
									((*tagfb).common.framebuffer_pitch * i + 3 * i)
										.try_into()
										.unwrap()
								) as *mut u32;
								*pixel = (colour & 0xffffff) | (*pixel & 0xff000000)
							}
							32 => {
								let pixel = fb.wrapping_add(
									((*tagfb).common.framebuffer_pitch * i + 4 * i)
										.try_into()
										.unwrap()
								) as *mut u32;
								*pixel = colour;
							}
							_ => {}
						}
						i += 1;
					}
				}

				MULTIBOOT_TAG_TYPE_ACPI_OLD => {
					let oa: *const MultibootTagOldAcpi = tag as *const MultibootTagOldAcpi;

					let signature_arr = (*oa).signature;

					// convert the signature WITHOUT using the heap as its not initialized yet
					let signature: &str = match str::from_utf8(&signature_arr) {
						Ok(v) => v,
						Err(e) => panic!("invalid UTF-8 sequence: {}", e)
					};

					if signature != "RSD PTR " {
						serial_println!(
							"[MULTIBOOT2] [ERROR] acpi invalid signature: {}",
							signature
						)
					}

					serial_println!("[MULTIBOOT2] oa rsdp: {}", (*oa).rsdt_address);
					let mut lock = RSDT.lock();
					let virt = phys_to_virt(PhysAddr::new((*oa).rsdt_address.into()));
					*lock = virt;
				}

				MULTIBOOT_TAG_TYPE_LOAD_BASE_ADDR => {
					let lb: *const MultibootTagLoadBaseAddr =
						tag as *const MultibootTagLoadBaseAddr;
					let loaded_base = read_unaligned((*lb).load_base_addr as *const u32) as usize;

					let link_base = {
						println!("link phys base: {}", __link_phys_base);
						let sym = &__link_phys_base as *const u8 as usize;
						println!("sym: {}", sym);
						if sym != 0 { sym } else { 0x0010_0000usize } // 1mb fallback
					};

					println!("link_base: {}", link_base);
					println!("phys mem offset: {}", loaded_base.wrapping_sub(link_base));
					bi.physical_memory_offset = loaded_base.wrapping_sub(link_base);
				}
				_ => println!("Unknown multiboot tag.")
			}

			tag = (tag as *const u8).add((((*tag).size + 7) & !7).try_into().unwrap())
				as *const MultibootTag;
			let total = (tag as *const u8 as usize).wrapping_sub(mbi_addr);
			println!("Total mbi size: 0x{:X}", total);
		}
		println!("parsed mb2");
		bi
	}
}

/// Computes the physical memory map offset with symbols from the linker script.
/// # Safety
/// The symbols need to be there, and need to be valid in order for the computation
/// to be accurate.
pub unsafe fn compute_phys_map_offset() -> u64 {
	unsafe {
		let phys_base = &__link_phys_base as *const u8 as u64;
		let text_vaddr = &__text_addr as *const u8 as u64;
		text_vaddr.wrapping_sub(phys_base)
	}
}
