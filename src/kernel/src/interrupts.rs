// interrupts.rs

/*
Interrupt handling module for the kernel.
*/

use core::{
	arch::asm,
	sync::atomic::Ordering
};

use lazy_static::lazy_static;
use pic8259::ChainedPics;
use spin;
use x86_64::{
	registers::segmentation::Segment,
	structures::idt::{
		InterruptDescriptorTable,
		InterruptStackFrame,
		PageFaultErrorCode
	}
};

use crate::{
	apic::{TICK_COUNT, apic::send_eoi},
	gdt,
	hlt_loop,
	println,
	serial_println,
	task::executor::{
		CURRENT_PID,
		PROCESS_QUEUE,
		UserProcessState,
	}
};

// Syscall IDs
pub const SYS_PRINT: u32 = 1;
pub const SYS_EXIT: u32 = 2;
pub const SYS_FORK: u32 = 3;
pub const SYS_WAIT: u32 = 4;
pub const SYS_OPEN: u32 = 5;
pub const SYS_CLOSE: u32 = 6;
pub const SYS_READ: u32 = 7;
pub const SYS_WRITE: u32 = 8;
pub const SYS_EXEC: u32 = 9;
pub const SYS_KILL: u32 = 10;
pub const SYS_SLEEP: u32 = 11;

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
			idt[InterruptIndex::Timer.as_usize()]
				.set_handler_fn(apic_timer_handler)
				.set_stack_index(gdt::TIMER_IST_INDEX);
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
	serial_println!("EXCEPTION: GENERAL PROTECTION FAULT");
	serial_println!("Error Code: {:#x}", error_code);
	serial_println!("Stack Frame: {:#?}", stack_frame);

	// Get the current RSP
	let rsp: u64;
	unsafe {
		asm!("mov {}, rsp", out(reg) rsp);
	}

	// Access the stack values that iretq was about to pop
	let stack = rsp as *const u64;
	let rip_to_return = unsafe { *stack.add(4) }; // rsp + 32
	let cs_to_return = unsafe { *stack.add(5) }; // rsp + 40
	let rflags_to_return = unsafe { *stack.add(6) }; // rsp + 48
	let rsp_to_return = unsafe { *stack.add(7) }; // rsp + 56
	let ss_to_return = unsafe { *stack.add(8) }; // rsp + 64

	serial_println!("Values to be popped by iretq:");
	serial_println!("RIP: {:#x}", rip_to_return);
	serial_println!("CS: {:#x}", cs_to_return);
	serial_println!("RFLAGS: {:#x}", rflags_to_return);
	serial_println!("RSP: {:#x}", rsp_to_return);
	serial_println!("SS: {:#x}", ss_to_return);

	// Print current segment registers
	let ds = x86_64::registers::segmentation::DS::get_reg();
	let es = x86_64::registers::segmentation::ES::get_reg();
	let fs = x86_64::registers::segmentation::FS::get_reg();
	let gs = x86_64::registers::segmentation::GS::get_reg();
	let ss = x86_64::registers::segmentation::SS::get_reg();
	let cs = x86_64::registers::segmentation::CS::get_reg();

	serial_println!("Current Segment Registers:");
	serial_println!(
		"DS: {:#x}, ES: {:#x}, FS: {:#x}, GS: {:#x}, SS: {:#x}, CS: {:#x}",
		ds.0,
		es.0,
		fs.0,
		gs.0,
		ss.0,
		cs.0
	);

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
extern "x86-interrupt" fn page_fault_handler(
	stack_frame: InterruptStackFrame,
	error_code: PageFaultErrorCode
) {
	use x86_64::registers::control::Cr2;
	serial_println!("EXCEPTION: PAGE FAULT");
	serial_println!("Accessed Address: {:?}", Cr2::read()); // Faulting address
	serial_println!("Error Code: {:?}", error_code);
	serial_println!("Stack Frame: {:#?}", stack_frame);
	unsafe {
		let rax: u64;
		asm!("mov {}, rax", out(reg) rax);
		serial_println!("RAX: {:#x}", rax); // Print RAX
	}
	crate::hlt_loop();
}

/// APIC Timer Interrupt Handler.
///
/// This handler is invoked when the APIC timer fires. It can be expanded to
/// include tick counting, scheduling, or other periodic tasks. Notice that we
/// no longer use the PIC to acknowledge the interrupt; instead, we signal the
/// end-of-interrupt directly to the APIC using `send_eoi()`.
extern "x86-interrupt" fn apic_timer_handler(_stack_frame: InterruptStackFrame) {
	serial_println!(
		"[Debug] APIC timer interrupt, tick count: {}",
		TICK_COUNT.load(Ordering::Relaxed)
	);
	TICK_COUNT.fetch_add(1, Ordering::Relaxed);
	unsafe {
		send_eoi();
	}
}

// test function
fn copy_from_user(
	kernel_buffer: *mut u8,
	user_ptr: *const u8,
	len: usize
) -> Result<(), &'static str> {
	// **IMPORTANT: This is a highly simplified and potentially insecure example.**
	// A real `copy_from_user` needs much more robust validation and error handling.

	// In a very basic kernel (for demonstration purposes only):
	unsafe {
		// In a real kernel, you would need to do much more validation and error
		// handling here!
		core::ptr::copy_nonoverlapping(user_ptr, kernel_buffer, len);
	}
	Ok(()) // Indicate success (in this simplified example)
}

/// Syscall handler via int 0x80.
extern "x86-interrupt" fn syscall_handler(_stack_frame: InterruptStackFrame) {
	let syscall_id: u32;
	let arg1: u64;
	let arg2: u64;

	// Read registers separately to avoid potential conflicts
	unsafe {
		asm!("mov {0:e}, eax", out(reg) syscall_id);
		asm!("mov {0:e}, ebx", out(reg) arg1);
		asm!("mov {0:e}, ecx", out(reg) arg2);
	}

	match syscall_id {
		SYS_PRINT => {
			let ptr = arg1 as *const u8;
			let len = arg2 as usize;

			let mut kernel_buffer: [u8; 4096] = [0; 4096];
			serial_println!(
				"[Debug] SYS_PRINT: User string length = {}, Kernel buffer size = {}",
				len,
				kernel_buffer.len()
			);

			if len > kernel_buffer.len() {
				crate::println!("Error: String too long for kernel buffer!");
			} else {
				let kernel_ptr = kernel_buffer.as_mut_ptr();
				match copy_from_user(kernel_ptr, ptr, len) {
					Ok(_) => {
						let slice = &kernel_buffer[..len];
						if let Ok(s) = core::str::from_utf8(slice) {
							crate::println!("{}", s);
						} else {
							crate::println!("Error: Invalid UTF-8 from user space!");
						}
					}
					Err(e) => {
						crate::println!("Error copying from user space: {}", e);
					}
				}
			}
		}
		SYS_EXIT => {
			let exit_code = arg1 as i32;
			serial_println!("Process exiting with code {}", exit_code);
			if let Some(pid) = CURRENT_PID.lock().as_ref() {
				if let Some(queue) = PROCESS_QUEUE.lock().as_mut() {
					if let Some(proc) = queue.iter_mut().find(|p| p.id == *pid) {
						serial_println!("Process {} found, setting state to Terminated", pid.get());
						proc.state = UserProcessState::Terminated;
					}
				}
			}
			unsafe {
				asm!(
				"mov rsp, {0}",
				"jmp {1}",
				in(reg) crate::task::executor::kernel_stack_top(),
				sym crate::task::executor::run_combined_executor,
				options(noreturn)
				);
			}
		}
		SYS_KILL => {
			let pid_to_kill = arg1 as u64;
			crate::serial_println!("Killing process {}", pid_to_kill);
			if let Some(queue) = PROCESS_QUEUE.lock().as_mut() {
				if let Some(proc) = queue.iter_mut().find(|p| p.id.get() == pid_to_kill) {
					proc.state = UserProcessState::Terminated;
				}
			}
			unsafe {
				asm!(
				"mov rsp, {0}",
				"jmp {1}",
				in(reg) crate::task::executor::kernel_stack_top(),
				sym crate::task::executor::run_combined_executor,
				options(noreturn)
				);
			}
		}
		_ => {
			crate::println!("Unknown syscall ID: {}", syscall_id);
		}
	}

	// Debug stack before return
	let rsp_before: u64;
	unsafe {
		asm!("mov {}, rsp", out(reg) rsp_before);
	}
	serial_println!("[Debug] Before iretq RSP: {:#x}", rsp_before);
	for i in 0..5 {
		serial_println!("[Debug] RSP + {}*8: {:#x}", i, unsafe {
			*(rsp_before as *const u64).add(i)
		});
	}
}

/// Defines the interrupt vectors used in the IDT.
///
/// Although the PIC is no longer used for the timer, we can reuse the same
/// vector (32 or 0x20) for our APIC timer interrupt.
// interrupts.rs (partial update)
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
	Timer = 0x30,    // Vector 48 (0x30)
	Keyboard = 0x21  // Vector 33 (0x21)
}

impl InterruptIndex {
	fn as_u8(self) -> u8 {
		self as u8
	}

	fn as_usize(self) -> usize {
		usize::from(self.as_u8())
	}
}
