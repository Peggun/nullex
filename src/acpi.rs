//!
//! acpi.rs
//! 
//! ACPI definitions for the kernel.
//! 

use alloc::vec::Vec;
use core::ptr::{addr_of, read_unaligned};

use x86_64::VirtAddr;

use crate::{
	PHYS_MEM_OFFSET, apic::{PIC1_DATA, PIC2_DATA}, common::ports::outb, gsi::{GSI_TABLE, program_gsi_vector}, interrupts::allocate_and_register_vector, io::pci::{pci_find_index_from_gsi, try_bind_device}, lazy_static, serial_println, utils::mutex::SpinMutex
};

// https://wiki.osdev.org/RSDT
const MADT_TABLE_SIGNATURE: &'static str = "APIC";
const BERT_TABLE_SIGNATURE: &'static str = "BERT";
const CPEP_TABLE_SIGNATURE: &'static str = "CPEP";
const DSDT_TABLE_SIGNATURE: &'static str = "DSDT";
const ECDT_TABLE_SIGNATURE: &'static str = "ECDT";
const EINJ_TABLE_SIGNATURE: &'static str = "EINJ";
const ERST_TABLE_SIGNATURE: &'static str = "ERST";
const FADT_TABLE_SIGNATURE: &'static str = "FACP";
const FACS_TABLE_SIGNATURE: &'static str = "FACS";
const HEST_TABLE_SIGNATURE: &'static str = "HEST";
const MSCT_TABLE_SIGNATURE: &'static str = "MSCT";
const MPST_TABLE_SIGNATURE: &'static str = "MPST";
// skip OEM tables as there are lots of OEM tables (add later)
const PMTT_TABLE_SIGNATURE: &'static str = "PMTT";
const PSDT_TABLE_SIGNATURE: &'static str = "PSDT";
const RASF_TABLE_SIGNATURE: &'static str = "RASF";
const RSDT_TABLE_SIGNATURE: &'static str = "RSDT";
const SBST_TABLE_SIGNATURE: &'static str = "SBST";
const SLIT_TABLE_SIGNATURE: &'static str = "SLIT";
const SRAT_TABLE_SIGNATURE: &'static str = "SRAT";
const SSDT_TABLE_SIGNATURE: &'static str = "SSDT";
const XSDT_TABLE_SIGNATURE: &'static str = "XSDT";

lazy_static! {
	/// Static reference to the Root System Descriptor Table (RSDT)
	pub static ref RSDT: SpinMutex<VirtAddr> = SpinMutex::new(VirtAddr::zero());
}

/// Enum representing all ACPI tables.
pub enum AcpiTableType {
	/// MADT Table
	Madt,
	/// BERT Table
	Bert,
	/// CPEP Table
	Cpep,
	/// DSDT Table
	Dsdt,
	/// ECDT Table
	Ecdt,
	/// EINJ Table
	Einj,
	/// ERST Table
	Erst,
	/// FADT Table
	Fadt,
	/// FACS Table
	Facs,
	/// HEST Table
	Hest,
	/// MSCT Table
	Msct,
	/// MPST Table
	Mpst,
	/// PMTT Table
	Pmtt,
	/// PSDT Table
	Psdt,
	/// RASF Table
	Rasf,
	/// RSDT Table
	Rsdt,
	/// SBST Table
	Sbst,
	/// SLIT Table
	Slit,
	/// SRAT Table
	Srat,
	/// SSDT Table
	Ssdt,
	/// XSDT Table
	Xsdt
}

impl AcpiTableType {
	fn signature(&self) -> &'static str {
		match self {
			AcpiTableType::Madt => MADT_TABLE_SIGNATURE,
			AcpiTableType::Bert => BERT_TABLE_SIGNATURE,
			AcpiTableType::Cpep => CPEP_TABLE_SIGNATURE,
			AcpiTableType::Dsdt => DSDT_TABLE_SIGNATURE,
			AcpiTableType::Ecdt => ECDT_TABLE_SIGNATURE,
			AcpiTableType::Einj => EINJ_TABLE_SIGNATURE,
			AcpiTableType::Erst => ERST_TABLE_SIGNATURE,
			AcpiTableType::Fadt => FADT_TABLE_SIGNATURE,
			AcpiTableType::Facs => FACS_TABLE_SIGNATURE,
			AcpiTableType::Hest => HEST_TABLE_SIGNATURE,
			AcpiTableType::Msct => MSCT_TABLE_SIGNATURE,
			AcpiTableType::Mpst => MPST_TABLE_SIGNATURE,
			AcpiTableType::Pmtt => PMTT_TABLE_SIGNATURE,
			AcpiTableType::Psdt => PSDT_TABLE_SIGNATURE,
			AcpiTableType::Rasf => RASF_TABLE_SIGNATURE,
			AcpiTableType::Rsdt => RSDT_TABLE_SIGNATURE,
			AcpiTableType::Sbst => SBST_TABLE_SIGNATURE,
			AcpiTableType::Slit => SLIT_TABLE_SIGNATURE,
			AcpiTableType::Srat => SRAT_TABLE_SIGNATURE,
			AcpiTableType::Ssdt => SSDT_TABLE_SIGNATURE,
			AcpiTableType::Xsdt => XSDT_TABLE_SIGNATURE
		}
	}
}

// https://wiki.osdev.org/RSDT
#[repr(C)]
#[derive(Debug, Copy, Clone)]
/// Structure representing a ACPI SDT (System Descriptor Table) header. All tables have this.
pub struct AcpiSdtHeader {
	signature: [u8; 4],
	length: u32,
	revision: u8,
	checksum: u8,
	oem_id: [u8; 6],
	oem_table_id: [u8; 8],
	oem_revision: u32,
	creator_id: u32,
	creator_revision: u32
}

#[repr(C, packed)]
struct Rsdt {
	header: AcpiSdtHeader,
	pointers_to_other_sdt: Vec<u32>
}

#[allow(unused)]
impl Rsdt {
	// incase. not used currently, *const T is in use.
	pub fn new(header: AcpiSdtHeader) -> Result<Self, &'static str> {
		if str::from_utf8(&header.signature).unwrap() != RSDT_TABLE_SIGNATURE {
			return Err("Incorrect RSDT Signature.\nAre you sure you are trying to parse RSDT?")
		}

		let ptos = (header.length as usize - size_of::<AcpiSdtHeader>()) / 4;

		Ok(Self {
			header,
			pointers_to_other_sdt: Vec::with_capacity(ptos)
		})
	}
}

#[repr(C, packed)]
#[derive(Debug)]
struct MadtTable {
	header: AcpiSdtHeader,
	lapic_addr: u32,
	flags: u32
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
struct MadtTableEntry {
	r#type: u8,
	length: u8
}

#[repr(C, packed)]
#[derive(Debug)]
struct InterruptSourceOverride {
	header: MadtTableEntry,
	bus: u8,
	source: u8,
	gsi: u32,
	flags: u16
}

/// Finds and returns the specified ACPI table.
pub unsafe fn find_acpi_table(
	root_sdt: VirtAddr,
	table_type: AcpiTableType
) -> Option<*const AcpiSdtHeader> {
	unsafe {
		let rsdt: *const Rsdt = root_sdt.as_u64() as *const Rsdt;
		let entries = ((*rsdt).header.length as usize - size_of::<AcpiSdtHeader>()) / 4;

		for entry in 0..entries {
			let ptr = addr_of!((*rsdt).pointers_to_other_sdt);
			let h = (ptr as *const u32).add(entry).read_unaligned() as *const AcpiSdtHeader;
			if str::from_utf8(&(*h).signature).unwrap() != table_type.signature() {
				continue;
			}

			return Some(h);
		}

		None
	}
}

/// Finds and links all Interrupt Source Overrides (ISO) 
pub unsafe fn link_isos() {
	serial_println!("[ACPI] Starting ISO (Interrupt Source Override) linking...");

	unsafe {
		let madt_table = find_acpi_table(*RSDT.lock(), AcpiTableType::Madt)
			.expect("no madt table found") as *const MadtTable;

		serial_println!("[ACPI] MADT table found at {:#x}", madt_table as usize);

		if ((*madt_table).flags & (1 << 0)) != 0 {
			serial_println!("[ACPI] Disabling legacy PICs");
			outb(PIC1_DATA, 0xFF);
			outb(PIC2_DATA, 0xFF);
		}

		let ioapic_virt_base = (*PHYS_MEM_OFFSET.lock()).as_u64() + 0xFEC0_0000u64;
		serial_println!("[ACPI] IOAPIC virtual base: {:#x}", ioapic_virt_base);

		let local_apic_id = (crate::apic::read_register(crate::apic::APIC_ID) >> 24) as u8;
		serial_println!("[ACPI] Local APIC ID: {}", local_apic_id);

		let base_u8 = madt_table as *const u8;
		let start = base_u8.add(size_of::<MadtTable>()) as *const u8;
		let end = base_u8.add((*madt_table).header.length as usize) as *const u8;

		let mut entry_ptr = start;
		let mut iso_count = 0;

		// First pass: Record all ISOs in the GSI table
		serial_println!("[ACPI] First pass: Recording ISOs...");
		while (entry_ptr as usize) < (end as usize) {
			let entry_hdr = entry_ptr as *const MadtTableEntry;
			let entry = read_unaligned(entry_hdr);

			match entry.r#type {
				2 => {
					let iso_ptr = entry_ptr as *const InterruptSourceOverride;
					let iso = read_unaligned(iso_ptr);
					let gsi = iso.gsi as usize;
					let bus = iso.bus;
					let source = iso.source;
					let flags = iso.flags;

					serial_println!(
						"[ACPI] ISO found: bus={}, source={}, GSI={}, flags={:#x}",
						bus,
						source,
						gsi,
						flags
					);

					if gsi >= 256 {
						serial_println!("[ACPI] WARNING: GSI {} out of range, skipping", gsi);
						entry_ptr = entry_ptr.add(entry.length as usize);
						continue;
					}

					{
						let mut gt = GSI_TABLE.lock();
						gt[gsi].flags = flags;
						gt[gsi].has_iso = true;
					}

					iso_count += 1;
				}
				_ => {}
			}

			let len = entry.length as usize;
			if len == 0 {
				serial_println!("[ACPI] ERROR: MADT entry length is 0 — aborting");
				break;
			}
			entry_ptr = entry_ptr.add(len);
		}

		serial_println!("[ACPI] Found {} ISOs in first pass", iso_count);

		// Second pass: For each ISO with a handler, allocate vector and program IOAPIC
		serial_println!("[ACPI] Second pass: Programming IOAPICs...");
		let mut programmed_count = 0;

		for gsi in 0..256 {
			let (has_iso, has_handler, _existing_vector) = {
				let gt = GSI_TABLE.lock();
				(gt[gsi].has_iso, gt[gsi].handler.is_some(), gt[gsi].vector)
			};

			if !has_iso {
				continue;
			}

			serial_println!("[ACPI] Processing GSI {}: has_handler={}", gsi, has_handler);

			// If no handler yet, try to bind a device driver
			if !has_handler {
				serial_println!(
					"[ACPI] No handler for GSI {}, attempting device binding...",
					gsi
				);
				if let Some(idx) = pci_find_index_from_gsi(gsi) {
					serial_println!("[ACPI] Found PCI device index {} for GSI {}", idx, gsi);
					try_bind_device(idx);
				} else {
					serial_println!("[ACPI] No PCI device found for GSI {}", gsi);
				}
			}

			// Re-check if we now have a handler
			let (maybe_handler, existing_vector) = {
				let gt = GSI_TABLE.lock();
				(gt[gsi].handler, gt[gsi].vector)
			};

			if let Some(handler_fn) = maybe_handler {
				// CRITICAL FIX: Check if vector already exists (from driver probe)
				let vector = if let Some(existing) = existing_vector {
					serial_println!(
						"[ACPI] GSI {} already has vector {}, reusing it",
						gsi,
						existing
					);
					existing as usize
				} else {
					// Only allocate a new vector if one doesn't exist
					serial_println!("[ACPI] Allocating new vector for GSI {}...", gsi);
					match allocate_and_register_vector(handler_fn) {
						Ok(v) => {
							serial_println!("[ACPI] Allocated vector {} for GSI {}", v, gsi);
							{
								let mut gt = GSI_TABLE.lock();
								gt[gsi].vector = Some(v as u8);
							}
							v
						}
						Err(e) => {
							serial_println!(
								"[ACPI] ERROR: No vectors available for GSI {}: {}",
								gsi,
								e
							);
							continue;
						}
					}
				};

				serial_println!(
					"[ACPI] Programming IOAPIC for GSI {} with vector {}...",
					gsi,
					vector
				);
				program_gsi_vector(
					ioapic_virt_base,
					gsi as u8,
					vector as u8,
					local_apic_id,
					true
				);
				programmed_count += 1;
			} else {
				serial_println!(
					"[ACPI] No handler found for GSI {} after binding attempt",
					gsi
				);
			}
		}

		serial_println!(
			"[ACPI] ISO linking complete: programmed {} interrupts",
			programmed_count
		);
	}
}