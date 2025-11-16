// https://cgit.git.savannah.gnu.org/cgit/grub.git/tree/doc/multiboot2.h?h=multiboot2
// https://cgit.git.savannah.gnu.org/cgit/grub.git/tree/doc/kernel.c?h=multiboot2

use core::{ptr::read_unaligned, u64};

use bootloader::bootinfo::{FrameRange, MemoryMap};

use crate::println;

pub const MULTIBOOT_SEARCH: u32 = 32768;
pub const MULTIBOOT_HEADER_ALIGN: u32 = 8;

pub const MULTIBOOT2_HEADER_MAGIC: u32 = 0xe85250d6;
pub const MULTIBOOT2_BOOTLOADER_MAGIC: u32 = 0x36d76289; // not needed, boot.asm does the check
pub const MULTIBOOT_MOD_ALIGN: u32 = 0x00001000;
pub const MULTIBOOT_INFO_ALIGN: u32 = 0x00000008;

pub const MULTIBOOT_TAG_ALIGN: u32 = 8;
pub const MULTIBOOT_TAG_TYPE_END: u32 = 0;
pub const MULTIBOOT_TAG_TYPE_CMDLINE: u32 = 1;
pub const MULTIBOOT_TAG_TYPE_BOOT_LOADER_NAME: u32 = 2;
pub const MULTIBOOT_TAG_TYPE_MODULE: u32 = 3;
pub const MULTIBOOT_TAG_TYPE_BASIC_MEMINFO: u32 = 4;
pub const MULTIBOOT_TAG_TYPE_BOOTDEV: u32 = 5;
pub const MULTIBOOT_TAG_TYPE_MMAP: u32 = 6;
pub const MULTIBOOT_TAG_TYPE_VBE: u32 = 7;
pub const MULTIBOOT_TAG_TYPE_FRAMEBUFFER: u32 = 8;
pub const MULTIBOOT_TAG_TYPE_ELF_SECTIONS: u32 = 9;
pub const MULTIBOOT_TAG_TYPE_APM: u32 = 10;
pub const MULTIBOOT_TAG_TYPE_EFI32: u32 = 11;
pub const MULTIBOOT_TAG_TYPE_EFI64: u32 = 12;
pub const MULTIBOOT_TAG_TYPE_SMBIOS: u32 = 13;
pub const MULTIBOOT_TAG_TYPE_ACPI_OLD: u32 = 14;
pub const MULTIBOOT_TAG_TYPE_ACPI_NEW: u32 = 15;
pub const MULTIBOOT_TAG_TYPE_NETWORK: u32 = 16;
pub const MULTIBOOT_TAG_TYPE_EFI_MMAP: u32 = 17;
pub const MULTIBOOT_TAG_TYPE_EFI_BS: u32 = 18;
pub const MULTIBOOT_TAG_TYPE_EFI32_IH: u32 = 19;
pub const MULTIBOOT_TAG_TYPE_EFI64_IH: u32 = 20;
pub const MULTIBOOT_TAG_TYPE_LOAD_BASE_ADDR: u32 = 21;

pub const MULTIBOOT_HEADER_TAG_END: u32 = 0;
pub const MULTIBOOT_HEADER_TAG_INFOMATION_REQUEST: u32 = 1;
pub const MULTIBOOT_HEADER_TAG_ADDRESS: u32 = 2;
pub const MULTIBOOT_HEADER_TAG_ENTRY_ADDRESS: u32 = 3;
pub const MULTIBOOT_HEADER_TAG_CONSOLE_FLAGS: u32 = 4;
pub const MULTIBOOT_HEADER_TAG_FRAMEBUFFER: u32 = 5;
pub const MULTIBOOT_HEADER_TAG_MODULE_ALIGN: u32 = 6;
pub const MULTIBOOT_HEADER_TAG_EFI_BS: u32 = 7;
pub const MULTIBOOT_HEADER_TAG_ENTRY_ADDRESS_EFI32: u32 = 8;
pub const MULTIBOOT_HEADER_TAG_ENTRY_ADDRESS_EFI64: u32 = 9;
pub const MULTIBOOT_HEADER_TAG_RELOCATABLE: u32 = 10;

pub const MULTIBOOT_ARCHITECTURE_I386: u32 = 0;
pub const MULTIBOOT_ARCHITECTURE_MIPS32: u32 = 4;
pub const MULTIBOOT_HEADER_TAG_OPIONAL: u32 = 1;

pub const MULTIBOOT_LOAD_PREFERENCE_NONE: u32 = 0;
pub const MULTIBOOT_LOAD_PREFERENCE_LOW: u32 = 1;
pub const MULTIBOOT_LOAD_PREFERENCE_HIGH: u32 = 4;

pub const MULTIBOOT_CONSOLE_FLAGS_CONSOLE_REQUIRED: u32 = 1;
pub const MULTIBOOT_CONSOLE_FLAGS_EGA_TEXT_SUPPORTED: u32 = 2;

pub const MULTIBOOT_MEMORY_AVAILABLE: u32 = 1;
pub const MULTIBOOT_MEMORY_RESERVED: u32 = 2;
pub const MULTIBOOT_MEMORY_ACPI_RECLAIMABLE: u32 = 3;
pub const MULTIBOOT_MEMORY_NVS: u32 = 4;
pub const MULTIBOOT_MEMORY_BADRAM: u32 = 5;

pub const MULTIBOOT_FRAMEBUFFER_TYPE_INDEXED: u8 = 0;
pub const MULTIBOOT_FRAMEBUFFER_TYPE_RGB: u8 = 1;
pub const MULTIBOOT_FRAMEBUFFER_TYPE_EGA_TEXT: u8 = 2;

#[repr(C)]
#[derive(Debug)]
pub struct MultibootHeader {
	pub magic: u32,
	pub architecture: u32,
	pub header_length: u32,
	pub checksum: u32
}

#[repr(C)]
#[derive(Debug)]
pub struct MultibootHeaderTag {
	pub r#type: u16,
	pub flags: u16,
	pub size: u32
}

#[repr(C)]
#[derive(Debug)]
pub struct MultibootHeaderTagInformationRequest {
	pub r#type: u16,
	pub flags: u16,
	pub size: u32,
	pub requests: [u32; 0]
}

#[repr(C)]
#[derive(Debug)]
pub struct MultibootHeaderTagAddress {
	pub r#type: u16,
	pub flags: u16,
	pub size: u32,
	pub header_addr: u32,
	pub load_addr: u32,
	pub load_end_addr: u32,
	pub bss_end_addr: u32
}

#[repr(C)]
#[derive(Debug)]
pub struct MultibootHeaderTagEntryAddress {
	pub r#type: u16,
	pub flags: u16,
	pub size: u32,
	pub entry_addr: u32
}

#[repr(C)]
#[derive(Debug)]
pub struct MultibootHeaderTagConsoleFlags {
	pub r#type: u16,
	pub flags: u16,
	pub size: u32,
	pub console_flags: u32
}

#[repr(C)]
#[derive(Debug)]
pub struct MultibootHeaderTagFramebuffer {
	pub r#type: u16,
	pub flags: u16,
	pub size: u32,
	pub width: u32,
	pub height: u32,
	pub depth: u32
}

#[repr(C)]
#[derive(Debug)]
pub struct MultibootHeaderTagModuleAlign {
	pub r#type: u16,
	pub flags: u16,
	pub size: u32
}

#[repr(C)]
#[derive(Debug)]
pub struct MultibootHeaderTagRelocatable {
	pub r#type: u16,
	pub flags: u16,
	pub size: u32,
	pub min_addr: u32,
	pub max_addr: u32,
	pub align: u32,
	pub preference: u32
}

#[repr(C)]
#[derive(Debug)]
pub struct MultibootColour {
	pub red: u8,
	pub green: u8,
	pub blue: u8
}

#[repr(C)]
#[derive(Debug)]
pub struct MultibootMmapEntry {
	pub addr: u64,
	pub len: u64,

	// defines are above
	pub r#type: u32,
	pub zero: u32
}
pub type MultibootMemoryMap = MultibootMmapEntry;

#[repr(C)]
#[derive(Debug)]
pub struct MultibootTag {
	pub r#type: u32,
	pub size: u32
}

#[repr(C)]
#[derive(Debug)]
pub struct MultibootTagString {
	pub r#type: u32,
	pub size: u32,
	pub string: [u8; 0]
}

#[repr(C)]
#[derive(Debug)]
pub struct MultibootTagModule {
	pub r#type: u32,
	pub size: u32,
	pub mod_start: u32,
	pub mod_end: u32,
	pub cmdline: [u8; 0]
}

#[repr(C)]
#[derive(Debug)]
pub struct MultibootTagBasicMemInfo {
	pub r#type: u32,
	pub size: u32,
	pub mem_lower: u32,
	pub mem_upper: u32
}

#[repr(C)]
#[derive(Debug)]
pub struct MultibootTagBootDev {
	pub r#type: u32,
	pub size: u32,
	pub biosdev: u32,
	pub slice: u32,
	pub part: u32
}

#[repr(C)]
#[derive(Debug)]
pub struct MultibootTagMmap {
	pub r#type: u32,
	pub size: u32,
	pub entry_size: u32,
	pub entry_version: u32,
	pub entries: [MultibootMmapEntry; 0]
}

#[repr(C)]
#[derive(Debug)]
pub struct MultibootVbeInfoBlock {
	pub external_specification: [u8; 512]
}

#[repr(C)]
#[derive(Debug)]
pub struct MultibootVbeModeInfoBlock {
	pub external_specification: [u8; 256]
}

#[repr(C)]
#[derive(Debug)]
pub struct MultibootTagVbe {
	pub r#type: u32,
	pub size: u32,

	pub vbe_mode: u16,
	pub vbe_interface_seg: u16,
	pub vbe_interface_off: u16,
	pub vbe_interface_len: u16,

	pub vbe_control_info: MultibootVbeInfoBlock,
	pub vbe_mode_info: MultibootVbeModeInfoBlock
}

#[repr(C)]
#[derive(Debug)]
pub struct MultibootTagFramebufferCommon {
	pub r#type: u32,
	pub size: u32,

	pub framebuffer_addr: u64,
	pub framebuffer_pitch: u32,
	pub framebuffer_width: u32,
	pub framebuffer_height: u32,
	pub framebuffer_bpp: u8,

	// defines are above
	pub framebuffer_type: u8,
	pub reserved: u16
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FramebufferPalette {
	pub framebuffer_palette_num_colors: u16,
	pub framebuffer_palette: *const MultibootColour
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FramebufferRgbFields {
	pub framebuffer_red_field_position: u8,
	pub framebuffer_red_mask_size: u8,
	pub framebuffer_green_field_position: u8,
	pub framebuffer_green_mask_size: u8,
	pub framebuffer_blue_field_position: u8,
	pub framebuffer_blue_mask_size: u8
}

#[repr(C)]
pub union FramebufferDetails {
	pub palette: FramebufferPalette,
	pub rgb_fields: FramebufferRgbFields
}

#[repr(C)]
pub struct MultibootTagFramebuffer {
	pub common: MultibootTagFramebufferCommon,
	pub details: FramebufferDetails
}

#[repr(C)]
#[derive(Debug)]
pub struct MultibootTagElfSections {
	pub r#type: u32,
	pub size: u32,
	pub num: u32,
	pub entsize: u32,
	pub shndx: u32,
	pub sections: [u8; 0]
}

#[repr(C)]
#[derive(Debug)]
pub struct MultibootTagApm {
	pub r#type: u32,
	pub size: u32,
	pub version: u16,
	pub cseg: u16,
	pub offset: u32,
	pub cseg_16: u16,
	pub dseg: u16,
	pub flags: u16,
	pub cseg_len: u16,
	pub cseg_16_len: u16,
	pub dseg_len: u16
}

#[repr(C)]
#[derive(Debug)]
pub struct MultibootTagEfi32 {
	pub r#type: u32,
	pub size: u32,
	pub pointer: u32
}

#[repr(C)]
#[derive(Debug)]
pub struct MultibootTagEfi64 {
	pub r#type: u32,
	pub size: u32,
	pub pointer: u64
}

#[repr(C)]
#[derive(Debug)]
pub struct MultibootTagSmbios {
	pub r#type: u32,
	pub size: u32,
	pub major: u8,
	pub minor: u8,
	pub reserved: [u8; 6],
	pub tables: [u8; 0]
}

#[repr(C)]
#[derive(Debug)]
pub struct MultibootTagOldAcpi {
	pub r#type: u32,
	pub size: u32,
	pub rsdp: [u8; 0]
}

#[repr(C)]
#[derive(Debug)]
pub struct MultibootTagNewAcpi {
	pub r#type: u32,
	pub size: u32,
	pub rsdp: [u8; 0]
}

#[repr(C)]
#[derive(Debug)]
pub struct MultibootTagEfiMmap {
	pub r#type: u32,
	pub size: u32,
	pub descr_size: u32,
	pub descr_vers: u32,
	pub efi_mmap: [u8; 0]
}

#[repr(C)]
#[derive(Debug)]
pub struct MultibootTagEfi32IH {
	pub r#type: u32,
	pub size: u32,
	pub pointer: u32
}

#[repr(C)]
#[derive(Debug)]
pub struct MultibootTagEfi64IH {
	pub r#type: u32,
	pub size: u32,
	pub pointer: u64
}

#[repr(C)]
#[derive(Debug)]
pub struct MultibootTagLoadBaseAddr {
	pub r#type: u32,
	pub size: u32,
	pub load_base_addr: u32
}

#[repr(C)]
#[derive(Debug)]
pub struct MultibootInfoHeader {
	pub total_size: u32,
	pub reserved: u32
}

pub struct BootInformation {
	pub physical_memory_offset: usize,
	pub memory_map: MemoryMap
}

impl BootInformation {
	fn new() -> Self {
		Self {
			physical_memory_offset: 0,
			memory_map: MemoryMap::new()
		}
	}
}

// linker symbols
unsafe extern "C" {
	pub unsafe static __text_addr: u8; // .text addr in linker.ld
	pub unsafe static __link_phys_base: u8;
	pub unsafe static _end: u8;
}

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
							MULTIBOOT_MEMORY_AVAILABLE => {
								bootloader::bootinfo::MemoryRegionType::Usable
							}
							MULTIBOOT_MEMORY_RESERVED => {
								bootloader::bootinfo::MemoryRegionType::Reserved
							}
							MULTIBOOT_MEMORY_ACPI_RECLAIMABLE => {
								bootloader::bootinfo::MemoryRegionType::AcpiReclaimable
							}
							MULTIBOOT_MEMORY_NVS => bootloader::bootinfo::MemoryRegionType::AcpiNvs,
							MULTIBOOT_MEMORY_BADRAM => {
								bootloader::bootinfo::MemoryRegionType::BadMemory
							}
							_ => bootloader::bootinfo::MemoryRegionType::Reserved
						};

						bl_map.add_region(bootloader::bootinfo::MemoryRegion {
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

pub unsafe fn compute_phys_map_offset() -> u64 {
	unsafe {
		let phys_base = &__link_phys_base as *const u8 as u64;
		let text_vaddr = &__text_addr as *const u8 as u64;
		text_vaddr.wrapping_sub(phys_base)
	}
}
