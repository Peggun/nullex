use core::sync::atomic::AtomicU64;

use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};
use crate::{apic::apic::send_eoi, println, gdt, hlt_loop};
use lazy_static::lazy_static;
use spin;
use pic8259::ChainedPics;

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

// We'll keep the PIC for devices such as the keyboard.
pub static PICS: spin::Mutex<ChainedPics> =
    spin::Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

static TICK_COUNT: AtomicU64 = AtomicU64::new(0);

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();

        // Standard exception handlers.
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
            
            // For the timer, switch from the PIC timer handler to the APIC timer handler.
            idt[InterruptIndex::Timer.as_usize()].set_handler_fn(apic_timer_handler);
            
            // Leave the keyboard handler using PIC (for example).
            idt[InterruptIndex::Keyboard.as_usize()].set_handler_fn(keyboard_interrupt_handler);
        }
        idt
    };
}

/// Loads the Interrupt Descriptor Table.
pub fn init_idt() {
    IDT.load();
}

/// Breakpoint exception handler.
extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

/// Double fault handler.
extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64
) -> ! {
    println!("\n\nDOUBLE FAULT");
    panic!("System halted");
}

/// Keyboard interrupt handler (still using the PIC).
extern "x86-interrupt" fn keyboard_interrupt_handler(
    _stack_frame: InterruptStackFrame
) {
    use x86_64::instructions::port::Port;
    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };
    crate::task::keyboard::scancode::add_scancode(scancode);

    unsafe {
        PICS.lock().notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}

/// Page fault handler.
extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;

    println!("EXCEPTION: PAGE FAULT");
    println!("Accessed Address: {:?}", Cr2::read());
    println!("Error Code: {:?}", error_code);
    println!("{:#?}", stack_frame);
    hlt_loop();
}

/// APIC Timer Interrupt Handler.
///
/// This handler is invoked when the APIC timer fires. It can be expanded to
/// include tick counting, scheduling, or other periodic tasks. Notice that we
/// no longer use the PIC to acknowledge the interrupt; instead, we signal the
/// end-of-interrupt directly to the APIC using `send_eoi()`.
extern "x86-interrupt" fn apic_timer_handler(_stack_frame: InterruptStackFrame) {
    // Example: You might increment a global tick counter or call a scheduler here.
    println!("APIC Timer Interrupt");

    // Signal to the Local APIC that the interrupt has been handled.
    unsafe {
        send_eoi();
    }
}

/// Defines the interrupt vectors used in the IDT.
///
/// Although the PIC is no longer used for the timer, we can reuse the same
/// vector (32 or 0x20) for our APIC timer interrupt.
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,    // Vector 32 (0x20)
    Keyboard,                // Vector 33 (0x21)
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }
    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}
