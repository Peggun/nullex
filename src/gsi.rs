//! gsi.rs
//! 
//! Global System Interrupt module for the kernel.

use alloc::vec::Vec;

use x86_64::structures::idt::InterruptStackFrame;

use crate::{ioapic::IoApic, lazy_static, serial_println, utils::mutex::SpinMutex};

#[derive(Debug, Default, Clone)]
/// Global System Interrupt (GSI) information structure.
///
/// Represents configuration and state for a GSI, which is a numbered interrupt
/// in the system's interrupt controller (typically used in x86 systems via ACPI).
/// GSIs are used to route hardware interrupts from devices to CPU interrupt vectors.
pub struct GsiInfo {
	/// GSI configuration flags.
	pub flags: u16,
	/// Indicates whether this GSI uses level-triggered (ISO) interrupt mode.
	pub has_iso: bool,
	/// The CPU interrupt vector number assigned to this GSI, if allocated.
	pub vector: Option<u8>,
	/// Pointer to the device associated with this GSI, if applicable.
	pub device_ptr: Option<usize>,
	/// The interrupt handler function for this GSI, if registered.
	pub handler: Option<extern "x86-interrupt" fn(InterruptStackFrame)>,

	/// Current pending/active state of this GSI.
	pub pending: bool
}

lazy_static! {
	/// Static reference to the Global System Interrupt Table.
	pub static ref GSI_TABLE: SpinMutex<Vec<GsiInfo>> =
		SpinMutex::new(vec![GsiInfo::default(); 256]);
}

/// Programs a global system interrupt to a vector.
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