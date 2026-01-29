// SPDX-License-Identifier: MIT OR Apache-2.0
// This file: <src/ioapic.rs>
// Portions copied from upstream:
//   https://github.com/kwzhao/x2apic-rs (commit aff8465)
//   Upstream original file(s): <src/ioapic/*>
// Copyright (c) 2019 Kevin Zhao
// Modifications: Added serial_println! for debugging, modifed for kernel halts.
// Expanded `RedirectionTableEntry` impl functions to support all of the
// possible RTE flags. Made `high` & `low` RTE members public. Added tests
// See THIRD_PARTY_LICENSES.md for full license texts and upstream details.

use core::{
	convert::{TryFrom, TryInto},
	fmt,
	ptr::{self, Unique},
	sync::atomic::{Ordering, compiler_fence}
};

use crate::{PHYS_MEM_OFFSET, bitflags, lazy_static, serial_println, utils::mutex::SpinMutex};

lazy_static! {
	pub static ref IOAPIC: SpinMutex<IoApic> =
		SpinMutex::new(unsafe { IoApic::new(PHYS_MEM_OFFSET.lock().as_u64() + 0xFEC0_0000) });
}

#[derive(Debug)]
pub struct IoApicRegisters {
	ioregsel: Unique<u32>,
	ioregwin: Unique<u32>
}

impl IoApicRegisters {
	pub unsafe fn new(base_addr: u64) -> Self {
		unsafe {
			let base = base_addr as *mut u32;

			IoApicRegisters {
				ioregsel: Unique::new_unchecked(base.offset(0)),
				ioregwin: Unique::new_unchecked(base.offset(4))
			}
		}
	}

	pub unsafe fn read(&mut self, selector: u32) -> u32 {
		unsafe {
			ptr::write_volatile(self.ioregsel.as_ptr(), selector);
			ptr::read_volatile(self.ioregwin.as_ptr())
		}
	}

	pub unsafe fn write(&mut self, selector: u32, value: u32) {
		unsafe {
			compiler_fence(Ordering::SeqCst);
			ptr::write_volatile(self.ioregsel.as_ptr(), selector);
			ptr::write_volatile(self.ioregwin.as_ptr(), value);
			compiler_fence(Ordering::SeqCst);
			let _ = ptr::read_volatile(self.ioregwin.as_ptr());
		}
	}

	pub unsafe fn set(&mut self, selector: u32, mask: u32) {
		unsafe {
			ptr::write_volatile(self.ioregsel.as_ptr(), selector);

			let val = ptr::read_volatile(self.ioregwin.as_ptr());
			ptr::write_volatile(self.ioregwin.as_ptr(), val | mask);
		}
	}

	pub unsafe fn clear(&mut self, selector: u32, mask: u32) {
		unsafe {
			ptr::write_volatile(self.ioregsel.as_ptr(), selector);

			let val = ptr::read_volatile(self.ioregwin.as_ptr());
			ptr::write_volatile(self.ioregwin.as_ptr(), val & !mask);
		}
	}
}

// Register selectors
pub const ID: u32 = 0x00;
pub const VERSION: u32 = 0x01;
pub const ARBITRATION: u32 = 0x02;
pub const TABLE_BASE: u32 = 0x10;
pub const IRQ_MODE_MASK: u32 = 0x0000_0700;

/// IOAPIC interrupt modes.
#[derive(Debug, PartialEq)]
#[repr(u8)]
pub enum IrqMode {
	/// Asserts the INTR signal on all allowed processors.
	Fixed = 0b000,
	/// Asserts the INTR signal on the lowest priority processor allowed.
	LowestPriority = 0b001,
	/// System management interrupt.
	/// Requires edge-triggering.
	SystemManagement = 0b010,
	/// Asserts the NMI signal on all allowed processors.
	/// Requires edge-triggering.
	NonMaskable = 0b100,
	/// Asserts the INIT signal on all allowed processors.
	/// Requires edge-triggering.
	Init = 0b101,
	/// Asserts the INTR signal as a signal that originated in an
	/// externally-connected interrupt controller.
	/// Requires edge-triggering.
	External = 0b111
}

impl IrqMode {
	pub(super) fn as_u32(self) -> u32 {
		(self as u32) << 8
	}
}

impl TryFrom<u32> for IrqMode {
	type Error = u32;

	fn try_from(value: u32) -> Result<Self, Self::Error> {
		match (value & IRQ_MODE_MASK) >> 8 {
			0b000 => Ok(IrqMode::Fixed),
			0b001 => Ok(IrqMode::LowestPriority),
			0b010 => Ok(IrqMode::SystemManagement),
			0b100 => Ok(IrqMode::NonMaskable),
			0b101 => Ok(IrqMode::Init),
			0b111 => Ok(IrqMode::External),
			other => Err(other)
		}
	}
}

bitflags! {
	/// Redirection table entry flags.
	#[derive(Debug, Clone, Copy)]
	pub struct IrqFlags: u32 {
		/// Logical destination mode (vs physical)
		const LOGICAL_DEST = 1 << 11;
		/// Delivery status: send pending (vs idle, readonly)
		const SEND_PENDING = 1 << 12;
		/// Low-polarity interrupt signal (vs high-polarity)
		const LOW_ACTIVE = 1 << 13;
		/// Remote IRR (readonly)
		const REMOTE_IRR = 1 << 14;
		/// Level-triggered interrupt (vs edge-triggered)
		const LEVEL_TRIGGERED = 1 << 15;
		/// Masked interrupt (vs unmasked)
		const MASKED = 1 << 16;
	}
}

/// Redirection table entry.
#[derive(Default)]
pub struct RedirectionTableEntry {
	pub low: u32,
	pub high: u32
}

impl RedirectionTableEntry {
	pub(crate) fn from_raw(low: u32, high: u32) -> Self {
		Self {
			low,
			high
		}
	}

	pub(crate) fn into_raw(self) -> (u32, u32) {
		(self.low, self.high)
	}

	/// Returns the interrupt vector.
	pub fn vector(&self) -> u8 {
		(self.low & 0xff) as u8
	}

	/// Sets the interrupt vector to `vector`.
	pub fn set_vector(&mut self, vector: u8) {
		self.low = self.low & !0xff | vector as u32
	}

	/// Returns the interrupt delivery mode.
	pub fn mode(&self) -> IrqMode {
		self.low.try_into().unwrap()
	}

	/// Sets the interrupt delivery mode to `mode`.
	pub fn set_mode(&mut self, mode: IrqMode) {
		self.low = self.low & !IRQ_MODE_MASK | mode.as_u32()
	}

	/// Returns the redirection table entry flags.
	pub fn flags(&self) -> IrqFlags {
		IrqFlags::from_bits_truncate(self.low)
	}

	/// Sets the redirection table entry flags to `flags`.
	pub fn set_flags(&mut self, flags: IrqFlags) {
		let ro_flags = IrqFlags::SEND_PENDING | IrqFlags::REMOTE_IRR;
		self.low = self.low & !(IrqFlags::all() - ro_flags).bits() | (flags - ro_flags).bits()
	}

	/// Returns the destination field.
	pub fn dest(&self) -> u8 {
		(self.high >> 24) as u8
	}

	/// Sets the destination field to `dest`.
	pub fn set_dest(&mut self, dest: u8) {
		self.high = (dest as u32) << 24;
	}

	pub fn dest_mode(&self) -> bool {
		((self.low >> 11) & 0x1) != 0
	}

	pub fn delivery_status(&self) -> bool {
		((self.low >> 12) & 0x1) != 0
	}

	pub fn set_delivery_status(&mut self, val: bool) {
		self.low = (self.low & !(0x1 << 12)) | ((val as u32) << 12);
	}

	pub fn polarity(&self) -> bool {
		((self.low >> 13) & 0x1) != 0
	}

	pub fn set_polarity(&mut self, val: bool) {
		self.low = (self.low & !(0x1 << 13)) | ((val as u32) << 13);
	}

	pub fn remote_irr(&self) -> bool {
		((self.low >> 14) & 0x1) != 0
	}

	pub fn set_remote_irr(&mut self, val: bool) {
		self.low = (self.low & !(0x1 << 14)) | ((val as u32) << 14);
	}

	pub fn trigger_mode(&self) -> bool {
		((self.low >> 15) & 0x1) != 0
	}

	pub fn set_trigger_mode(&mut self, val: bool) {
		self.low = (self.low & !(0x1 << 15)) | ((val as u32) << 15);
	}

	pub fn mask(&self) -> bool {
		((self.low >> 16) & 0x1) != 0
	}

	pub fn set_mask(&mut self, val: bool) {
		self.low = (self.low & !(0x1 << 16)) | ((val as u32) << 16);
	}

	pub fn destination(&self) -> u8 {
		(self.high & 0xff) as u8
	}

	pub fn set_destination(&mut self, val: u8) {
		self.high = (self.high & !0xff) | (val as u32 & 0xff);
	}
}

// Gets the lower segment selector for `irq`
pub fn lo(irq: u8) -> u32 {
	TABLE_BASE + (2 * u32::from(irq))
}

// Gets the upper segment selector for `irq`
pub fn hi(irq: u8) -> u32 {
	lo(irq) + 1
}

impl fmt::Debug for RedirectionTableEntry {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.debug_struct("RedirectionTableEntry")
			.field("vector", &self.vector())
			.field("mode", &self.mode())
			.field("flags", &self.flags())
			.field("dest", &self.dest())
			.finish()
	}
}

/// The IOAPIC structure.
#[derive(Debug)]
pub struct IoApic {
	regs: IoApicRegisters
}

impl IoApic {
	pub unsafe fn new(base_addr: u64) -> Self {
		unsafe {
			IoApic {
				regs: IoApicRegisters::new(base_addr)
			}
		}
	}

	pub unsafe fn init(&mut self, offset: u8, dest_apic: u8) {
		unsafe {
			serial_println!(
				"[IOAPIC] Starting init, offset={}, dest_apic={}",
				offset,
				dest_apic
			);

			serial_println!(
				"[IOAPIC] IOREGSEL ptr = {:p}, IOWIN ptr = {:p}",
				self.regs.ioregsel.as_ptr(),
				self.regs.ioregwin.as_ptr()
			);

			let version_reg = self.regs.read(VERSION);
			let max_redir = ((version_reg >> 16) & 0xFF) as u8;
			serial_println!(
				"[IOAPIC] Version {:#X}, max redir entries = {}",
				version_reg,
				max_redir
			);

			let safe_cap: u8 = core::cmp::min(max_redir, 15);

			serial_println!("[IOAPIC] using safe_cap = {}", safe_cap);

			for irq in 0..=safe_cap {
				serial_println!("[IOAPIC] programming irq {}", irq);

				let vector = offset + irq;
				let mut entry = RedirectionTableEntry::default();

				entry.set_vector(vector);
				entry.set_mode(IrqMode::Fixed);
				entry.set_dest(dest_apic);
				entry.set_flags(IrqFlags::MASKED);

				let (low, high) = entry.into_raw();

				// write low then high; read them back and log if mismatch
				self.regs.write(lo(irq), low);
				self.regs.write(hi(irq), high);

				let verify_lo = self.regs.read(lo(irq));
				let verify_hi = self.regs.read(hi(irq));
				if verify_lo != low || verify_hi != high {
					serial_println!(
						"[IOAPIC] Warning: RTE write verification mismatch irq={} wrote=(0x{:08X},0x{:08X}) read=(0x{:08X},0x{:08X})",
						irq,
						low,
						high,
						verify_lo,
						verify_hi
					);
				} else {
					serial_println!("[IOAPIC] RTE {} OK", irq);
				}
			}

			// Now explicitly configure/unmask keyboard (IRQ 1) if it's within safe_cap.
			if 1 <= safe_cap {
				let mut entry = RedirectionTableEntry::default();
				entry.set_vector(offset + 1);
				entry.set_mode(IrqMode::Fixed);
				entry.set_flags(IrqFlags::empty()); // unmasked
				entry.set_dest(dest_apic);

				let (low, high) = entry.into_raw();
				self.regs.write(lo(1), low);
				self.regs.write(hi(1), high);

				// Verify keyboard RTE specifically
				let verify_lo = self.regs.read(lo(1));
				let verify_hi = self.regs.read(hi(1));
				serial_println!(
					"[IOAPIC] Keyboard RTE read back = (0x{:08X},0x{:08X})",
					verify_lo,
					verify_hi
				);
			} else {
				serial_println!("[IOAPIC] WARNING: safe_cap < 1; keyboard not configured");
			}

			dump_gsi(11);

			serial_println!(
				"[IOAPIC] init finished (safe initialization). advertised_max = {}, safe_cap = {}",
				max_redir,
				safe_cap
			);
		}
	}

	/// Returns the IOAPIC ID.
	pub unsafe fn id(&mut self) -> u8 {
		unsafe { ((self.regs.read(ID) >> 24) & 0xf) as u8 }
	}

	/// Sets the IOAPIC ID to `id`.
	pub unsafe fn set_id(&mut self, id: u8) {
		unsafe {
			self.regs.write(ID, u32::from(id) << 24);
		}
	}

	/// Returns the IOAPIC version.
	pub unsafe fn version(&mut self) -> u8 {
		unsafe { (self.regs.read(VERSION) & 0xff) as u8 }
	}

	/// Returns the entry number (starting at zero) of the highest entry in the
	/// redirection table.
	pub unsafe fn max_table_entry(&mut self) -> u8 {
		unsafe { ((self.regs.read(VERSION) >> 16) & 0xff) as u8 }
	}

	/// Returns the IOAPIC arbitration ID.
	pub unsafe fn arbitration_id(&mut self) -> u8 {
		unsafe { ((self.regs.read(ARBITRATION) >> 24) & 0xf) as u8 }
	}

	/// Sets the IOAPIC arbitration ID to `id`.
	pub unsafe fn set_arbitration_id(&mut self, id: u8) {
		unsafe {
			self.regs.write(ARBITRATION, u32::from(id) << 24);
		}
	}

	/// Returns the redirection table entry of `irq`.
	pub unsafe fn table_entry(&mut self, irq: u8) -> RedirectionTableEntry {
		unsafe {
			let lo = lo(irq);
			let hi = hi(irq);
			RedirectionTableEntry::from_raw(self.regs.read(lo), self.regs.read(hi))
		}
	}

	/// Configures the redirection table entry of `irq` to `entry`.
	pub unsafe fn set_table_entry(&mut self, irq: u8, entry: RedirectionTableEntry) {
		unsafe {
			let lo = lo(irq);
			let hi = hi(irq);
			let (lo_value, hi_value) = entry.into_raw();
			self.regs.write(lo, lo_value);
			self.regs.write(hi, hi_value);
		}
	}

	/// Enable interrupt number `irq`.
	pub unsafe fn enable_irq(&mut self, irq: u8) {
		unsafe {
			self.regs.clear(lo(irq), IrqFlags::MASKED.bits());
		}
	}

	/// Disable interrupt number `irq`.
	pub unsafe fn disable_irq(&mut self, irq: u8) {
		unsafe {
			self.regs.set(lo(irq), IrqFlags::MASKED.bits());
		}
	}
}

pub mod prelude {
	pub use crate::ioapic::*;
}

#[cfg(feature = "test")]
pub mod tests {
	use crate::{ioapic::prelude::*, utils::ktest::TestError};

	pub fn test_lo_hi_computation() -> Result<(), TestError> {
		let l = lo(5);
		let h = hi(5);
		assert_eq!(h, l + 1);
		assert_eq!(l, TABLE_BASE + (2 * 5u32));
		Ok(())
	}
	crate::create_test!(test_lo_hi_computation);

	pub fn test_rte_vector_set_get() -> Result<(), TestError> {
		let mut e = RedirectionTableEntry::default();
		e.set_vector(0xAB);
		assert_eq!(e.vector(), 0xAB);
		Ok(())
	}
	crate::create_test!(test_rte_vector_set_get);

	pub fn test_rte_mode_roundtrip() -> Result<(), TestError> {
		let mut e = RedirectionTableEntry::default();
		assert_eq!(e.mode(), IrqMode::Fixed);
		e.set_mode(IrqMode::NonMaskable);
		assert_eq!(e.mode(), IrqMode::NonMaskable);
		Ok(())
	}
	crate::create_test!(test_rte_mode_roundtrip);

	pub fn test_flags_and_dest() -> Result<(), TestError> {
		let mut e = RedirectionTableEntry::default();
		e.set_flags(IrqFlags::MASKED | IrqFlags::LEVEL_TRIGGERED);
		let flags = e.flags();
		assert!(flags.contains(IrqFlags::MASKED));
		assert!(flags.contains(IrqFlags::LEVEL_TRIGGERED));
		e.set_dest(0xEE);
		assert_eq!(e.dest(), 0xEE);
		Ok(())
	}
	crate::create_test!(test_flags_and_dest);

	pub fn test_into_from_raw_roundtrip() -> Result<(), TestError> {
		let mut a = RedirectionTableEntry::default();
		a.set_vector(0x12);
		a.set_mode(IrqMode::External);
		a.set_flags(IrqFlags::MASKED);
		a.set_dest(0x42);

		let (lo_raw, hi_raw) = a.into_raw();
		let b = RedirectionTableEntry::from_raw(lo_raw, hi_raw);

		assert_eq!(b.vector(), 0x12);
		assert_eq!(b.mode(), IrqMode::External);
		assert!(b.flags().contains(IrqFlags::MASKED));
		assert_eq!(b.dest(), 0x42);
		Ok(())
	}
	crate::create_test!(test_into_from_raw_roundtrip);

	pub fn test_invalid_irqmode_tryfrom() -> Result<(), TestError> {
		let raw = (0b011_u32) << 8;
		match IrqMode::try_from(raw) {
			Ok(_) => panic!("expected Err for invalid mode 0b011"),
			Err(e) => assert_eq!(e, 0b011)
		}
		Ok(())
	}
	crate::create_test!(test_invalid_irqmode_tryfrom);
}

pub fn dump_gsi(gsi: u8) {
	unsafe {
		let ioapic_virt_base = (*PHYS_MEM_OFFSET.lock()).as_u64() + 0xFEC0_0000u64;
		let mut ioapic = IoApicRegisters::new(ioapic_virt_base);
		let lov = ioapic.read(lo(gsi));
		let hiv = ioapic.read(hi(gsi));
		serial_println!("GSI {} RTE: lo={:#010x} hi={:#010x}", gsi, lov, hiv);
		let rte = RedirectionTableEntry::from_raw(lov, hiv);
		serial_println!(
			"[RTE] vector={} mode={:?} flags={:?} dest={:#x}",
			rte.vector(),
			rte.mode(),
			rte.flags(),
			rte.dest()
		);
	}
}
