// SPDX-License-Identifier: MIT OR Apache-2.0
// This file: <src/ioapic.rs>
// Portions copied from upstream:
//   https://github.com/kwzhao/x2apic-rs (commit aff8465)
//   Upstream original file(s): <src/ioapic/*>
// Copyright (c) 2019 Kevin Zhao
// Modifications: None
// See THIRD_PARTY_LICENSES.md for full license texts and upstream details.

use core::{
	convert::{TryFrom, TryInto},
	fmt,
	ptr::{self, Unique}
};

use crate::{bitflags, serial_println};

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
			ptr::write_volatile(self.ioregsel.as_ptr(), selector);
			ptr::write_volatile(self.ioregwin.as_ptr(), value);
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
#[derive(Debug)]
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
	low: u32,
	high: u32
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

			let version_reg = self.regs.read(VERSION);
			let max_redir = ((version_reg >> 16) & 0xFF) as u8;
			serial_println!(
				"[IOAPIC] Version {:#X}, max redir entries = {}",
				version_reg,
				max_redir
			);

			for irq in 0..=max_redir {
				let vector = offset + irq;
				let mut entry = RedirectionTableEntry::default();

				entry.set_vector(vector);
				entry.set_mode(IrqMode::Fixed);
				entry.set_dest(dest_apic);

				entry.set_flags(IrqFlags::MASKED);

				let (low, high) = entry.into_raw();
				self.regs.write(lo(irq), low);
				self.regs.write(lo(irq), high);
			}

			// Unmask the keyboard (IRQ 1)
			let mut entry = RedirectionTableEntry::default();
			entry.set_vector(offset + 1); // IRQ 1 → vector (offset + 1)
			entry.set_mode(IrqMode::Fixed);
			entry.set_flags(IrqFlags::empty()); // edge/high, unmasked
			entry.set_dest(dest_apic);

			let (low, high) = entry.into_raw();
			self.regs.write(lo(1), low);
			self.regs.write(hi(1), high);

			serial_println!(
				"[IOAPIC] IRQ1 → vector {}, dest_apic {}",
				offset + 1,
				dest_apic
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
