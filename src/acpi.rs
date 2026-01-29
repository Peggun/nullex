use alloc::vec::Vec;
use core::ptr::{addr_of, read_unaligned};

use x86_64::VirtAddr;

use crate::{
	PHYS_MEM_OFFSET,
	common::ports::outb,
	gsi::GSI_TABLE,
	interrupts::allocate_and_register_vector,
	io::pci::{pci_find_index_from_gsi, try_bind_device},
	ioapic::IoApic,
	lazy_static,
	rtc::{PIC1_DATA, PIC2_DATA},
	serial_println,
	utils::mutex::SpinMutex
};

// https://wiki.osdev.org/RSDT (What can you find?)
pub const MADT_TABLE_SIGNATURE: &'static str = "APIC";
pub const BERT_TABLE_SIGNATURE: &'static str = "BERT";
pub const CPEP_TABLE_SIGNATURE: &'static str = "CPEP";
pub const DSDT_TABLE_SIGNATURE: &'static str = "DSDT";
pub const ECDT_TABLE_SIGNATURE: &'static str = "ECDT";
pub const EINJ_TABLE_SIGNATURE: &'static str = "EINJ";
pub const ERST_TABLE_SIGNATURE: &'static str = "ERST";
pub const FADT_TABLE_SIGNATURE: &'static str = "FACP";
pub const FACS_TABLE_SIGNATURE: &'static str = "FACS";
pub const HEST_TABLE_SIGNATURE: &'static str = "HEST";
pub const MSCT_TABLE_SIGNATURE: &'static str = "MSCT";
pub const MPST_TABLE_SIGNATURE: &'static str = "MPST";
// skip OEM tables as there are lots of OEM tables (add later)
pub const PMTT_TABLE_SIGNATURE: &'static str = "PMTT";
pub const PSDT_TABLE_SIGNATURE: &'static str = "PSDT";
pub const RASF_TABLE_SIGNATURE: &'static str = "RASF";
pub const RSDT_TABLE_SIGNATURE: &'static str = "RSDT";
pub const SBST_TABLE_SIGNATURE: &'static str = "SBST";
pub const SLIT_TABLE_SIGNATURE: &'static str = "SLIT";
pub const SRAT_TABLE_SIGNATURE: &'static str = "SRAT";
pub const SSDT_TABLE_SIGNATURE: &'static str = "SSDT";
pub const XSDT_TABLE_SIGNATURE: &'static str = "XSDT";

lazy_static! {
	pub static ref RSDT: SpinMutex<VirtAddr> = SpinMutex::new(VirtAddr::zero());
}

pub enum AcpiTableType {
	Madt,
	Bert,
	Cpep,
	Dsdt,
	Ecdt,
	Einj,
	Erst,
	Fadt,
	Facs,
	Hest,
	Msct,
	Mpst,
	Pmtt,
	Psdt,
	Rasf,
	Rsdt,
	Sbst,
	Slit,
	Srat,
	Ssdt,
	Xsdt
}

impl AcpiTableType {
	pub fn signature(&self) -> &'static str {
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
pub struct AcpiSdtHeader {
	pub signature: [u8; 4],
	pub length: u32,
	pub revision: u8,
	pub checksum: u8,
	pub oem_id: [u8; 6],
	pub oem_table_id: [u8; 8],
	pub oem_revision: u32,
	pub creator_id: u32,
	pub creator_revision: u32
}

#[repr(C, packed)]
pub struct Rsdt {
	pub header: AcpiSdtHeader,
	pub pointers_to_other_sdt: Vec<u32>
}

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
pub struct MadtTable {
	pub header: AcpiSdtHeader,
	pub lapic_addr: u32,
	pub flags: u32
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct MadtTableEntry {
	pub r#type: u8,
	pub length: u8
}

#[repr(C, packed)]
#[derive(Debug)]
pub struct InterruptSourceOverride {
	pub header: MadtTableEntry,
	pub bus: u8,
	pub source: u8,
	pub gsi: u32,
	pub flags: u16
}

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

pub fn program_gsi_vector(ioapic_base: u64, gsi: u8, vector: u8, dest_apic: u8, unmask: bool) {
	serial_println!(
		"[IOAPIC] Programming GSI {} -> vector {}, dest APIC {}, unmask={}",
		gsi,
		vector,
		dest_apic,
		unmask
	);

	let mut ioapic = unsafe { IoApic::new(ioapic_base) };
	let mut rte = unsafe { ioapic.table_entry(gsi) };

	let gsi_table = GSI_TABLE.lock();

	if gsi_table[gsi as usize].has_iso {
		let flags = gsi_table[gsi as usize].flags;
		serial_println!("[IOAPIC] GSI {} has ISO with flags {:#x}", gsi, flags);

		// Polarity: bits 0-1
		match flags & 0x03 {
			0x01 => {
				serial_println!("[IOAPIC] Setting active-high polarity for GSI {}", gsi);
				rte.set_polarity(false); // false = active high
			}
			0x03 => {
				serial_println!("[IOAPIC] Setting active-low polarity for GSI {}", gsi);
				rte.set_polarity(true); // true = active low
			}
			_ => {
				serial_println!("[IOAPIC] Using default polarity for GSI {}", gsi);
			}
		}

		// Trigger mode: bits 2-3
		match (flags >> 2) & 0x03 {
			0x01 => {
				serial_println!("[IOAPIC] Setting edge-triggered mode for GSI {}", gsi);
				rte.set_trigger_mode(false); // false = edge
			}
			0x03 => {
				serial_println!("[IOAPIC] Setting level-triggered mode for GSI {}", gsi);
				rte.set_trigger_mode(true); // true = level
			}
			_ => {
				serial_println!("[IOAPIC] Using default trigger mode for GSI {}", gsi);
			}
		}
	} else {
		serial_println!("[IOAPIC] GSI {} has no ISO, using defaults", gsi);
	}

	rte.set_vector(vector);
	rte.set_dest(dest_apic);
	rte.set_mask(!unmask);

	serial_println!("[IOAPIC] Writing RTE for GSI {}", gsi);
	unsafe {
		ioapic.set_table_entry(gsi, rte);
	}

	// Verify the write
	let verify = unsafe { ioapic.table_entry(gsi) };
	serial_println!(
		"[IOAPIC] Verified GSI {} -> vec={}, flags={:?}, dest={:#x}, mask={}",
		gsi,
		verify.vector(),
		verify.flags(),
		verify.dest(),
		verify.mask()
	);
}

pub fn unmask_all_programmed_gsis() {
	let ioapic_virt_base = PHYS_MEM_OFFSET.lock().as_u64() + 0xFEC0_0000u64;
	for gsi in 0..256 {
		if GSI_TABLE.lock()[gsi].vector.is_some() {
			let mut ioapic = unsafe { IoApic::new(ioapic_virt_base) };
			let mut rte = unsafe { ioapic.table_entry(gsi as u8) };
			rte.set_mask(false);
			unsafe {
				ioapic.set_table_entry(gsi as u8, rte);
			}
			serial_println!("[INIT] Unmasked GSI {}", gsi);
		}
	}
}
