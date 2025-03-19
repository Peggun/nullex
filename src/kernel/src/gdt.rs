// gdt.rs
/*
GDT (Global Descriptor Table) module for the kernel.
*/

use lazy_static::lazy_static;
use x86_64::{
	VirtAddr,
	structures::{
		gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector},
		tss::TaskStateSegment
	}
};

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;
pub const PAGE_FAULT_IST_INDEX: u16 = 1; // New IST index

lazy_static! {
	static ref GDT: (GlobalDescriptorTable, Selectors) = {
		let mut gdt = GlobalDescriptorTable::new();
		let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
		gdt.add_entry(Descriptor::user_code_segment());
		gdt.add_entry(Descriptor::user_data_segment());
		let tss_selector = gdt.add_entry(Descriptor::tss_segment(&TSS));
		(gdt, Selectors {
			code_selector,
			tss_selector
		})
	};
}

struct Selectors {
	code_selector: SegmentSelector,
	tss_selector: SegmentSelector
}

lazy_static! {
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();
        // Set the ring 0 privilege stack
        tss.privilege_stack_table[0] = {
            const STACK_SIZE: usize = 4096 * 5; // 20 KiB stack
            static mut STACK0: [u8; STACK_SIZE] = [0; STACK_SIZE];
            let stack_start = VirtAddr::from_ptr(&raw const STACK0);
            let stack_end = stack_start + STACK_SIZE; // Stack grows downward from this address
            stack_end
        };
        // Set IST for double fault
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            const STACK_SIZE: usize = 4096 * 5;
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];
            let stack_start = VirtAddr::from_ptr(&raw const STACK);
            let stack_end = stack_start + STACK_SIZE;
            stack_end
        };
        // Set IST for page fault
        tss.interrupt_stack_table[PAGE_FAULT_IST_INDEX as usize] = {
            const STACK_SIZE: usize = 4096 * 5;
            static mut STACK2: [u8; STACK_SIZE] = [0; STACK_SIZE];
            let stack_start = VirtAddr::from_ptr(&raw const STACK2);
            let stack_end = stack_start + STACK_SIZE;
            stack_end
        };
        tss
    };
}

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
