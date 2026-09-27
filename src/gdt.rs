//! gdt.rs
//!
//! GDT (Global Descriptor Table) module for the kernel.

use x86_64::{
	VirtAddr,
	structures::{
		gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector},
		tss::TaskStateSegment
	}
};

use crate::lazy_static;

pub(crate) const DOUBLE_FAULT_IST_INDEX: u16 = 0;
pub(crate) const PAGE_FAULT_IST_INDEX: u16 = 1;

const KERNEL_STACK_SIZE: usize = 4096 * 5;
static mut KERNEL_STACK: [u8; KERNEL_STACK_SIZE] = [0; KERNEL_STACK_SIZE];

/// Size of the stack when an interrupt is fired.
pub const INTERRUPT_STACK_SIZE: usize = 4096 * 8;
#[repr(align(16))]
struct IStack([u8; INTERRUPT_STACK_SIZE]);
static mut INTERRUPT_STACK: IStack = IStack([0; INTERRUPT_STACK_SIZE]);

/// The top of the Interrupt Stack
pub fn interrupt_stack_top() -> u64 {
	unsafe {
		let base = core::ptr::addr_of!(INTERRUPT_STACK.0) as u64;
		base + INTERRUPT_STACK_SIZE as u64
	}
}

lazy_static! {
	pub static ref TSS: TaskStateSegment = {
		let mut tss = TaskStateSegment::new();

		// IST slot 0: dedicated double-fault stack
		tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] =
			VirtAddr::new(double_fault_stack_top());

		// rsp0: kernel stack for ring 3 -> ring 0 transitions (interrupts, syscalls)
		tss.privilege_stack_table[0] = VirtAddr::new(interrupt_stack_top());

		tss
	};
}

lazy_static! {
	static ref GDT: (GlobalDescriptorTable, Selectors) = {
		let mut gdt = GlobalDescriptorTable::new();
		let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
		let tss_selector = gdt.add_entry(Descriptor::tss_segment(&TSS));
		// user_data must come before user_code for sysret compatibility
		let user_data_selector = gdt.add_entry(Descriptor::user_data_segment());
		let user_code_selector = gdt.add_entry(Descriptor::user_code_segment());
		(gdt, Selectors {
			code_selector,
			tss_selector,
			user_code_selector,
			user_data_selector,
		})
	};
}

pub const DOUBLE_FAULT_STACK_SIZE: usize = 4096 * 5;

/// Returns the virtual address of the bottom of the dedicated double-fault
/// stack.
pub fn double_fault_stack_start() -> u64 {
	unsafe { core::ptr::addr_of!(KERNEL_STACK) as u64 }
}

/// Returns the virtual address of the top of the dedicated double-fault
/// stack.
pub fn double_fault_stack_top() -> u64 {
	double_fault_stack_start() + DOUBLE_FAULT_STACK_SIZE as u64
}

struct Selectors {
	code_selector: SegmentSelector,
	tss_selector: SegmentSelector,
	user_code_selector: SegmentSelector,
	user_data_selector: SegmentSelector
}

/// Returns the raw u16 selector value with RPL=3 bits set.
pub fn user_code_selector() -> u16 {
	GDT.1.user_code_selector.0 | 3
}

/// Returns the raw u16 selector value of the user data with RPL=3 bits set.
pub fn user_data_selector() -> u16 {
	GDT.1.user_data_selector.0 | 3
}

/// Sets the kernel stack to the value passed.
pub fn set_kernel_stack(stack_top: u64) {
	unsafe {
		let tss = &*TSS as *const TaskStateSegment as *mut TaskStateSegment;
		(*tss).privilege_stack_table[0] = VirtAddr::new(stack_top);
	}
}

/// Initialises the GDT (Global Descriptor Table)
pub fn init() {
	use x86_64::instructions::{
		segmentation::{CS, Segment},
		tables::load_tss
	};

	GDT.0.load();
	unsafe {
		CS::set_reg(GDT.1.code_selector);
		load_tss(GDT.1.tss_selector);
	}
}

#[cfg(feature = "test")]
pub mod tests {
	use crate::{
		gdt::{
			DOUBLE_FAULT_IST_INDEX,
			INTERRUPT_STACK_SIZE,
			KERNEL_STACK_SIZE,
			TSS,
			interrupt_stack_top,
			set_kernel_stack,
			user_code_selector,
			user_data_selector
		},
		tassert,
		tassert_eq,
		testing::ktest::TestError
	};

	pub fn test_double_fault_ist_index() -> Result<(), TestError> {
		tassert_eq!(DOUBLE_FAULT_IST_INDEX, 0);
		Ok(())
	}
	crate::create_test!(test_double_fault_ist_index);

	pub fn test_kernel_stack_size() -> Result<(), TestError> {
		tassert_eq!(KERNEL_STACK_SIZE, 4096 * 5);
		Ok(())
	}
	crate::create_test!(test_kernel_stack_size);

	pub fn test_interrupt_stack_size() -> Result<(), TestError> {
		tassert_eq!(INTERRUPT_STACK_SIZE, 4096 * 8);
		Ok(())
	}
	crate::create_test!(test_interrupt_stack_size);

	pub fn test_interrupt_stack_top_calculation() -> Result<(), TestError> {
		let top = interrupt_stack_top();
		tassert!(
			top > 0,
			"Interrupt stack top should be a valid non-zero address"
		);
		tassert_eq!(
			top % 16,
			0,
			"Interrupt stack top should be 16-byte aligned due to IStack repr(align(16))"
		);
		Ok(())
	}
	crate::create_test!(test_interrupt_stack_top_calculation);

	pub fn test_user_code_selector_rpl() -> Result<(), TestError> {
		let selector = user_code_selector();
		tassert_eq!(
			selector & 0b11,
			3,
			"User code selector should have RPL=3 (lowest 2 bits set)"
		);
		Ok(())
	}
	crate::create_test!(test_user_code_selector_rpl);

	pub fn test_user_data_selector_rpl() -> Result<(), TestError> {
		let selector = user_data_selector();
		tassert_eq!(
			selector & 0b11,
			3,
			"User data selector should have RPL=3 (lowest 2 bits set)"
		);
		Ok(())
	}
	crate::create_test!(test_user_data_selector_rpl);

	pub fn test_tss_ist_double_fault_stack_initialized() -> Result<(), TestError> {
		let ist_entry = TSS.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize];
		tassert!(
			ist_entry.as_u64() > 0,
			"Double fault IST entry should be initialized to a valid stack top"
		);
		Ok(())
	}
	crate::create_test!(test_tss_ist_double_fault_stack_initialized);

	pub fn test_set_kernel_stack_updates_tss() -> Result<(), TestError> {
		let original_rsp0 = TSS.privilege_stack_table[0].as_u64();
		let new_stack_top = 0xFFFFBEEF00000000u64;

		set_kernel_stack(new_stack_top);

		let current_rsp0 = TSS.privilege_stack_table[0].as_u64();
		tassert_eq!(
			current_rsp0,
			new_stack_top,
			"set_kernel_stack should update TSS privilege_stack_table[0]"
		);

		set_kernel_stack(original_rsp0);
		Ok(())
	}
	crate::create_test!(test_set_kernel_stack_updates_tss);
}
