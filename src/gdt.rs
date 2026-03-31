//! gdt.rs
//!
//! GDT (Global Descriptor Table) module for the kernel.
//!

use x86_64::{
    VirtAddr,
    structures::{
        gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector},
        tss::TaskStateSegment
    }
};
use crate::lazy_static;

pub(crate) const DOUBLE_FAULT_IST_INDEX: u16 = 0;

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
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            let stack_start = VirtAddr::from_ptr(core::ptr::addr_of!(KERNEL_STACK));
            stack_start + KERNEL_STACK_SIZE
        };

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

struct Selectors {
    code_selector: SegmentSelector,
    tss_selector: SegmentSelector,
    user_code_selector: SegmentSelector,
    user_data_selector: SegmentSelector,
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