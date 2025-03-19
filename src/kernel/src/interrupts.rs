// interrupts.rs

/*
Interrupt handling module for the kernel.
*/

use core::{arch::asm, sync::atomic::Ordering};

use lazy_static::lazy_static;
use pic8259::ChainedPics;
use spin;
use x86_64::{structures::idt::{EntryOptions, InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode}, PrivilegeLevel};

use crate::{
	apic::{apic::send_eoi, TICK_COUNT},
	gdt,
	hlt_loop,
	println, serial_println
};

// Syscall IDs
pub const SYS_PRINT: u32 = 1;
pub const SYS_EXIT: u32 = 2;

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

// We'll keep the PIC for devices such as the keyboard.
pub static PICS: spin::Mutex<ChainedPics> =
	spin::Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });



lazy_static! {
	static ref IDT: InterruptDescriptorTable = {
		let mut idt = InterruptDescriptorTable::new();
		idt.breakpoint
			.set_handler_fn(breakpoint_handler)
			.set_privilege_level(x86_64::PrivilegeLevel::Ring3); // Set Ring 3 privilege level
		unsafe {
			idt.double_fault
				.set_handler_fn(double_fault_handler)
				.set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
			idt.page_fault
				.set_handler_fn(page_fault_handler)
				.set_stack_index(gdt::PAGE_FAULT_IST_INDEX); // Use IST
			idt[InterruptIndex::Timer.as_usize()].set_handler_fn(apic_timer_handler);
			idt[InterruptIndex::Keyboard.as_usize()].set_handler_fn(keyboard_interrupt_handler);
			idt[0x80]
				.set_handler_fn(syscall_handler)
				.set_privilege_level(x86_64::PrivilegeLevel::Ring3);
			idt.general_protection_fault.set_handler_fn(gp_fault_handler);
			idt[6].set_handler_fn(invalid_opcode_handler);
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
	serial_println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

/// Double fault handler.
extern "x86-interrupt" fn double_fault_handler(
	_stack_frame: InterruptStackFrame,
	_error_code: u64
) -> ! {
	println!("\n\nDOUBLE FAULT");
	println!("Error Code: {:?}", _error_code);
    println!("{:#?}", _stack_frame);
	serial_println!("\n\nDOUBLE FAULT");
	serial_println!("Error Code: {:?}", _error_code);
    serial_println!("{:#?}", _stack_frame);
	hlt_loop();
	//panic!("System halted");
}

extern "x86-interrupt" fn gp_fault_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    println!("EXCEPTION: GENERAL PROTECTION FAULT");
    println!("Error Code: {}", error_code);
    println!("{:#?}", stack_frame);
	serial_println!("EXCEPTION: GENERAL PROTECTION FAULT");
    serial_println!("Error Code: {}", error_code);
    serial_println!("{:#?}", stack_frame);
    hlt_loop();
}

extern "x86-interrupt" fn invalid_opcode_handler(stack_frame: InterruptStackFrame) {
	println!("EXCEPTION: INVALID OPCODE");
	println!("{:#?}", stack_frame);
	serial_println!("EXCEPTION: INVALID OPCODE");
	serial_println!("{:#?}", stack_frame);
	hlt_loop();
}

/// Keyboard interrupt handler (still using the PIC).
extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
	use x86_64::instructions::port::Port;
	let mut port = Port::new(0x60);
	let scancode: u8 = unsafe { port.read() };
	let _ = crate::task::keyboard::scancode::add_scancode(scancode);

	unsafe {
		PICS.lock()
			.notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
	}
}

/// Page fault handler.
extern "x86-interrupt" fn page_fault_handler(stack_frame: InterruptStackFrame, error_code: PageFaultErrorCode) {
    use x86_64::registers::control::Cr2;
    println!("EXCEPTION: PAGE FAULT");
    println!("Accessed Address: {:?}", Cr2::read());
    println!("Error Code: {:?}", error_code);
    println!("{:#?}", stack_frame);
	serial_println!("EXCEPTION: PAGE FAULT");
    serial_println!("Accessed Address: {:?}", Cr2::read());
    serial_println!("Error Code: {:?}", error_code);
    serial_println!("{:#?}", stack_frame);
    crate::hlt_loop(); // Avoid further faults
}

/// APIC Timer Interrupt Handler.
///
/// This handler is invoked when the APIC timer fires. It can be expanded to
/// include tick counting, scheduling, or other periodic tasks. Notice that we
/// no longer use the PIC to acknowledge the interrupt; instead, we signal the
/// end-of-interrupt directly to the APIC using `send_eoi()`.
extern "x86-interrupt" fn apic_timer_handler(_stack_frame: InterruptStackFrame) {
	TICK_COUNT.fetch_add(1, Ordering::Relaxed);
	unsafe {
		send_eoi();
	}
}

/// Syscall handler via int 0x80.
extern "x86-interrupt" fn syscall_handler(_stack_frame: InterruptStackFrame) {
	let syscall_id: u32;
	let arg1: u64;
	let arg2: u64;
	unsafe {
		asm!(
			"mov {0:r}, rax",  // Syscall ID
			"mov {1}, rdi",  // First argument
			"mov {2}, rsi",  // Second argument
			out(reg) syscall_id,
			out(reg) arg1,
			out(reg) arg2,
		);
	}

	match syscall_id {
		SYS_PRINT => {
			let ptr = arg1 as *const u8;
			let len = arg2 as usize;
			let slice = unsafe { core::slice::from_raw_parts(ptr, len) };
			if let Ok(s) = core::str::from_utf8(slice) {
				crate::println!("{}", s);
			}
		}
		SYS_EXIT => {
			let exit_code = arg1 as i32;
			crate::println!("Process exiting with code {}", exit_code);
			crate::hlt_loop();
		}
		_ => {
			crate::println!("Unknown syscall ID: {}", syscall_id);
		}
	}
}

/// Defines the interrupt vectors used in the IDT.
///
/// Although the PIC is no longer used for the timer, we can reuse the same
/// vector (32 or 0x20) for our APIC timer interrupt.
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
	Timer = PIC_1_OFFSET, // Vector 32 (0x20)
	Keyboard              // Vector 33 (0x21)
}

impl InterruptIndex {
	fn as_u8(self) -> u8 {
		self as u8
	}

	fn as_usize(self) -> usize {
		usize::from(self.as_u8())
	}
}
