// gdt.rs
use lazy_static::lazy_static;
use x86_64::{
    VirtAddr,
    structures::{
        gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector},
        tss::TaskStateSegment
    }
};
use core::ops::Deref;

pub static mut KERNEL_STACK_TOP: usize = 0;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;
pub const PAGE_FAULT_IST_INDEX: u16 = 1;
pub const TIMER_IST_INDEX: u16 = 2;

pub const STACK_SIZE: usize = 4096 * 5;
pub static mut KERNEL_STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];
pub static mut DOUBLE_FAULT_STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];
pub static mut PAGE_FAULT_STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];
// Define a struct with 16-byte alignment
#[repr(align(16))]
pub struct AlignedStack([u8; STACK_SIZE]);

impl Deref for AlignedStack {
    type Target = [u8; STACK_SIZE];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub static mut TIMER_IST_STACK: AlignedStack = AlignedStack([0; STACK_SIZE]);
pub static mut TSS: TaskStateSegment = TaskStateSegment::new();

lazy_static! {
    pub static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        
        let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
        let data_selector = gdt.add_entry(Descriptor::kernel_data_segment());
        gdt.add_entry(Descriptor::user_code_segment());
        gdt.add_entry(Descriptor::user_data_segment());
        let tss_selector = gdt.add_entry(Descriptor::tss_segment(
            unsafe { &*core::ptr::addr_of!(TSS) }
        ));
        
        let selectors = Selectors {
            code_selector,
            data_selector,
            tss_selector,
        };
        
        (gdt, selectors)
    };
}

pub struct Selectors {
    code_selector: SegmentSelector,
    data_selector: SegmentSelector,
    tss_selector: SegmentSelector,
}

pub fn init() {
    use x86_64::instructions::{
        segmentation::{CS, DS, ES, FS, GS, SS, Segment},
        tables::load_tss,
    };
    
    // Initialize stacks
    // Initialize stacks
    let kernel_stack_start = VirtAddr::from_ptr(unsafe { &raw const KERNEL_STACK as *const _ });
    let kernel_stack_top = kernel_stack_start + STACK_SIZE;
    unsafe { 
        TSS.privilege_stack_table[0] = kernel_stack_top;
        KERNEL_STACK_TOP = kernel_stack_top.as_u64() as usize; // Store the stack top
    }
    
    let double_fault_stack_start = VirtAddr::from_ptr(unsafe { &raw const DOUBLE_FAULT_STACK as *const _ });
    unsafe { TSS.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = double_fault_stack_start + STACK_SIZE; }
    
    let page_fault_stack_start = VirtAddr::from_ptr(unsafe { &raw const PAGE_FAULT_STACK as *const _ });
    unsafe { TSS.interrupt_stack_table[PAGE_FAULT_IST_INDEX as usize] = page_fault_stack_start + STACK_SIZE; }
    
    let timer_ist_stack_start = VirtAddr::from_ptr(unsafe { &raw const TIMER_IST_STACK as *const _ });
    unsafe { TSS.interrupt_stack_table[TIMER_IST_INDEX as usize] = timer_ist_stack_start + STACK_SIZE; }
    
    // Load GDT and set selectors
    let (gdt, selectors) = &*GDT;
    unsafe {
        gdt.load();
        CS::set_reg(selectors.code_selector);
        DS::set_reg(selectors.data_selector);
        ES::set_reg(selectors.data_selector);
        FS::set_reg(selectors.data_selector);
        GS::set_reg(selectors.data_selector);
        SS::set_reg(selectors.data_selector);
        load_tss(selectors.tss_selector);
    }
}