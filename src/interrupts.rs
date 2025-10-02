// interrupts.rs

/*
Interrupt handling module for the kernel.
*/

use core::{arch::asm, sync::atomic::Ordering};

use lazy_static::lazy_static;
use pic8259::ChainedPics;
use spin;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

use crate::{
	apic::{TICK_COUNT, send_eoi},
	gdt,
	hlt_loop,
	println,
	serial::add_byte,
	serial_println,
	syscall::syscall,
	task::executor::CURRENT_PROCESS
};

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

// We'll keep the PIC for devices such as the keyboard.
pub static PICS: spin::Mutex<ChainedPics> =
	spin::Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

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

			idt[InterruptIndex::Serial.as_usize()].set_handler_fn(serial_input_interrupt_handler);

			idt[0x80].set_handler_fn(syscall_handler);
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
	_stack_frame: InterruptStackFrame,
	_error_code: u64
) -> ! {
	println!("\n\nDOUBLE FAULT");
	panic!("System halted");
}

/// Keyboard interrupt handler (still using the pic).
extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
	use x86_64::instructions::port::Port;
	let mut port = Port::new(0x60);
	let scancode: u8 = unsafe { port.read() };

	{
		let mut lock = CURRENT_PROCESS.lock();

		let curr_proc = match lock.as_mut() {
			Some(proc) => proc,
			// keyboard process assumed
			// as that will always be running.
			None => {
				crate::task::keyboard::scancode::add_scancode(scancode);
				unsafe {
					PICS.lock()
						.notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
				}
				return;
			}
		};
		let curr_proc_queue = curr_proc.scancode_queue.try_get();
		let curr_proc_waker = &curr_proc.waker;

		if let Ok(queue) = curr_proc_queue {
			if queue.push(scancode).is_err() {
				// skip, the keypress gets dropped.
				// its not needed because only processes that dont use the
				// keyboard will fill up the scanqueue
			} else {
				curr_proc_waker.wake();
			}
		}
		// same here, its not needed because all processes that need the
		// keyboard will have the scanqueue setup
	}

	unsafe {
		PICS.lock()
			.notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
	}
}

extern "x86-interrupt" fn serial_input_interrupt_handler(_stack_frame: InterruptStackFrame) {
	use x86_64::instructions::port::Port;
	loop {
		let mut lsb = Port::<u8>::new(0x3FD);
		let lsb_data = unsafe { lsb.read() };
		if (lsb_data & 0x01) == 0 {
			break;
		}

		let mut rbr = Port::<u8>::new(0x3F8);
		let byte = unsafe { rbr.read() };
		add_byte(byte);
	}

	unsafe {
		PICS.lock()
			.notify_end_of_interrupt(InterruptIndex::Serial.as_u8());
	}
}

/// Page fault handler.
extern "x86-interrupt" fn page_fault_handler(
	stack_frame: InterruptStackFrame,
	error_code: PageFaultErrorCode
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
/// This handler is invoked when the APIC timer fires.
extern "x86-interrupt" fn apic_timer_handler(_stack_frame: InterruptStackFrame) {
	TICK_COUNT.fetch_add(1, Ordering::Relaxed);
	unsafe {
		send_eoi();
	}
}

extern "x86-interrupt" fn syscall_handler(_stack_frame: InterruptStackFrame) {
	let rax: u32; // syscall number
	let arg1: u64;
	let arg2: u64;
	let arg3: u64;

	// get syscall number and args
	unsafe {
		asm!(
			"mov {rax_out:r}, rax",
			"mov {rdi_out:r}, rdi",
			"mov {rsi_out:r}, rsi",
			"mov {rdx_out:r}, rdx",
			rax_out = out(reg) rax,
			rdi_out = out(reg) arg1,
			rsi_out = out(reg) arg2,
			rdx_out = out(reg) arg3,
			options(nostack, nomem),
		);
	}

	serial_println!(
		"rax: {}, arg1: {}, arg2: {}, arg3: {}",
		rax,
		arg1,
		arg2,
		arg3
	);

	let ret = unsafe { syscall(rax, arg1, arg2, arg3, 0, 0) };

	unsafe {
		core::arch::asm!(
			"mov rax, {0}",
			in(reg) ret as u64,
			options(nostack, nomem),
		);
	}
}

/// Defines the interrupt vectors used in the IDT.
///
/// Although the PIC is no longer used for the timer, we can reuse the same
/// vector (32 or 0x20) for our APIC timer interrupt.
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
	Timer = PIC_1_OFFSET,      // Vector 32 (0x20)
	Keyboard,                  // Vector 33 (0x21)
	Serial = PIC_1_OFFSET + 4  // Vector 36 (0x24) (Serial Input)
}

impl InterruptIndex {
	fn as_u8(self) -> u8 {
		self as u8
	}

	fn as_usize(self) -> usize {
		usize::from(self.as_u8())
	}
}
