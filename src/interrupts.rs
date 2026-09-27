//!
//! interrupts.rs
//!
//! Interrupt handling module for the kernel.

use core::{
	mem::MaybeUninit,
	sync::atomic::{AtomicBool, Ordering}
};

use ::x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use x86_64::{VirtAddr, structures::idt::PageFaultErrorCode};

use crate::{
	apic::{APIC_TICK_COUNT, send_eoi},
	arch::x86_64::user::{
		asm_copy_from_user_fault,
		asm_copy_from_user_src_end,
		asm_copy_from_user_src_start,
		asm_copy_to_user_dst_end,
		asm_copy_to_user_dst_start,
		asm_copy_to_user_fault
	},
	common::{
		bits::BitMap,
		ports::{inb, outb}
	},
	debug::backtrace::trace_stack_trace,
	drivers::keyboard::queue::add_scancode,
	error::NullexError,
	gdt,
	hlt_loop,
	kernel_panic,
	lazy_static,
	println,
	rtc::{CMOS_DATA, CMOS_INDEX, REG_C, RTC_TICKS, send_rtc_eoi},
	serial::add_byte,
	serial_println,
	sync::mutex::SpinMutex,
	syscall::syscall,
	task::cr3::KernelCr3Guard
};

pub(crate) const APIC_TIMER_VECTOR: u8 = 0x20;
const KEYBOARD_VECTOR: u8 = 0x21;
const SERIAL_VECTOR: u8 = 0x24;
const KASSERT_VECTOR: u8 = 0x25;
const RTC_VECTOR: u8 = 0x28;
const SYSCALL_VECTOR: u8 = 0x80;

// TODO: remove the maybeuninit, just move to a safe lazy_static!
static mut IDT_STORAGE: MaybeUninit<InterruptDescriptorTable> = MaybeUninit::uninit();
static IDT_INITED: AtomicBool = AtomicBool::new(false);

static IN_PAGE_FAULT: AtomicBool = AtomicBool::new(false);

/// Wraps `extern "x86-interrupt"` handlers with automatic CR3 save/restore.
/// You will still have to do the following checks to actually enable this:
/// - The interrupt must be enabled in the IDT through the `IDT_TABLE`
/// - The interrupt must be enabled in the PIC
/// - The interrupt must be a valid interrupt
#[macro_export]
macro_rules! kernel_interrupt {
    // ---------------------------------------------------------------------
    // No error code
    // ---------------------------------------------------------------------
    (
        $(#[$meta:meta])*
        fn $name:ident($frame:ident: InterruptStackFrame) $body:block
    ) => {
        $(#[$meta])*
        extern "x86-interrupt" fn $name(
            $frame: ::x86_64::structures::idt::InterruptStackFrame
        ) {
            let _cr3_guard =
                unsafe { $crate::task::cr3::KernelCr3Guard::enter_interrupt() };
            $body
        }
    };

    // No error code (mut)
    (
        $(#[$meta:meta])*
        fn $name:ident(mut $frame:ident: InterruptStackFrame) $body:block
    ) => {
        $(#[$meta])*
        extern "x86-interrupt" fn $name(
            mut $frame: ::x86_64::structures::idt::InterruptStackFrame
        ) {
            let _cr3_guard =
                unsafe { $crate::task::cr3::KernelCr3Guard::enter_interrupt() };
            $body
        }
    };

    // ---------------------------------------------------------------------
    // u64 error code
    // ---------------------------------------------------------------------
    (
        $(#[$meta:meta])*
        fn $name:ident(
            $frame:ident: InterruptStackFrame,
            $code:ident: u64
        ) $body:block
    ) => {
        $(#[$meta])*
        extern "x86-interrupt" fn $name(
            $frame: ::x86_64::structures::idt::InterruptStackFrame,
            $code: u64,
        ) {
            let _cr3_guard =
                unsafe { $crate::task::cr3::KernelCr3Guard::enter_interrupt() };
            $body
        }
    };

    // u64 error code (mut)
    (
        $(#[$meta:meta])*
        fn $name:ident(
            mut $frame:ident: InterruptStackFrame,
            $code:ident: u64
        ) $body:block
    ) => {
        $(#[$meta])*
        extern "x86-interrupt" fn $name(
            mut $frame: ::x86_64::structures::idt::InterruptStackFrame,
            $code: u64,
        ) {
            let _cr3_guard =
                unsafe { $crate::task::cr3::KernelCr3Guard::enter_interrupt() };
            $body
        }
    };

    // ---------------------------------------------------------------------
    // u64 error code -> !
    // ---------------------------------------------------------------------
    (
        $(#[$meta:meta])*
        fn $name:ident(
            $frame:ident: InterruptStackFrame,
            $code:ident: u64
        ) -> ! $body:block
    ) => {
        $(#[$meta])*
        extern "x86-interrupt" fn $name(
            $frame: ::x86_64::structures::idt::InterruptStackFrame,
            $code: u64,
        ) -> ! {
            let _cr3_guard =
                unsafe { $crate::task::cr3::KernelCr3Guard::enter_interrupt() };
            $body
        }
    };

    // ---------------------------------------------------------------------
    // PageFaultErrorCode
    // ---------------------------------------------------------------------
    (
        $(#[$meta:meta])*
        fn $name:ident(
            $frame:ident: InterruptStackFrame,
            $code:ident: PageFaultErrorCode
        ) $body:block
    ) => {
        $(#[$meta])*
        extern "x86-interrupt" fn $name(
            $frame: ::x86_64::structures::idt::InterruptStackFrame,
            $code: ::x86_64::structures::idt::PageFaultErrorCode,
        ) {
            let _cr3_guard =
                unsafe { $crate::task::cr3::KernelCr3Guard::enter_interrupt() };
            $body
        }
    };

    // PageFaultErrorCode (mut)
    (
        $(#[$meta:meta])*
        fn $name:ident(
            mut $frame:ident: InterruptStackFrame,
            $code:ident: PageFaultErrorCode
        ) $body:block
    ) => {
        $(#[$meta])*
        extern "x86-interrupt" fn $name(
            mut $frame: ::x86_64::structures::idt::InterruptStackFrame,
            $code: ::x86_64::structures::idt::PageFaultErrorCode,
        ) {
            let _cr3_guard =
                unsafe { $crate::task::cr3::KernelCr3Guard::enter_interrupt() };
            $body
        }
    };
}

lazy_static! {
	/// Static reference to all used vectors for ISO's (Interrupt Source Override)
	pub static ref VECTOR_TABLE: SpinMutex<BitMap> = {
		let mut bmp = BitMap::new(256);
		bmp.set_idxs((0..31).into(), true);
		bmp.set_idx(255, true);
		SpinMutex::new(bmp)
	};
}

/// Initializes the IDT (Interrupt Descriptor Table)
pub unsafe fn init_idt() {
	unsafe {
		::x86_64::instructions::interrupts::disable();

		let mut local_idt = InterruptDescriptorTable::new();

		// Exception handlers
		local_idt.breakpoint.set_handler_fn(breakpoint_handler);
		local_idt
			.page_fault
			.set_handler_fn(page_fault_handler)
			.set_stack_index(gdt::PAGE_FAULT_IST_INDEX);
		local_idt
			.double_fault
			.set_handler_fn(double_fault_handler)
			.set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
		local_idt
			.general_protection_fault
			.set_handler_fn(general_protection_fault_handler);

		// driver handlers
		local_idt[APIC_TIMER_VECTOR as usize].set_handler_fn(apic_timer_handler);
		local_idt[KEYBOARD_VECTOR as usize].set_handler_fn(keyboard_interrupt_handler);
		local_idt[SERIAL_VECTOR as usize].set_handler_fn(serial_input_interrupt_handler);
		local_idt[RTC_VECTOR as usize].set_handler_fn(rtc_timer_handler);

		// syscall handler
		local_idt[SYSCALL_VECTOR as usize]
			.set_handler_fn(syscall_handler)
			.set_privilege_level(::x86_64::PrivilegeLevel::Ring3)
			.set_present(true)
			.disable_interrupts(false);

		// custom handlers
		// kernel panics
		local_idt[KASSERT_VECTOR as usize].set_handler_fn(kernel_assert_interrupt_handler);

		// Spurious interrupt handler
		local_idt[0xFF].set_handler_fn(spurious_interrupt_handler);

		let storage_ptr: *mut MaybeUninit<InterruptDescriptorTable> =
			core::ptr::addr_of_mut!(IDT_STORAGE);
		let idt_ptr = storage_ptr as *mut InterruptDescriptorTable;
		core::ptr::write(idt_ptr, local_idt);
		let idt_ref: &InterruptDescriptorTable = &*idt_ptr;
		idt_ref.load();

		IDT_INITED.store(true, Ordering::SeqCst);
	}
}

/// Adds an IDT entry and sets a handler function.
pub unsafe fn add_idt_entry(
	vector: usize,
	handler: extern "x86-interrupt" fn(InterruptStackFrame)
) {
	unsafe {
		::x86_64::instructions::interrupts::without_interrupts(|| {
			let storage_ptr: *mut MaybeUninit<InterruptDescriptorTable> =
				core::ptr::addr_of_mut!(IDT_STORAGE);
			let idt_ptr = storage_ptr as *mut InterruptDescriptorTable;
			let idt_ref: &mut InterruptDescriptorTable = &mut *idt_ptr;

			idt_ref[vector].set_handler_fn(handler);
			idt_ref.load();
		});
	}
}

kernel_interrupt! {
	/// Breakpoint exception handler.
	fn breakpoint_handler(stack_frame: InterruptStackFrame) {
		println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
	}
}

kernel_interrupt! {
	fn page_fault_handler(
		mut stack_frame: InterruptStackFrame,
		error_code: PageFaultErrorCode
	) {
		use x86_64::registers::control::Cr2;

		let faulting_rip = stack_frame.instruction_pointer.as_u64() as usize;

		unsafe {
			let read_start = core::ptr::addr_of!(asm_copy_from_user_src_start) as usize;
			let read_end = core::ptr::addr_of!(asm_copy_from_user_src_end) as usize;

			if faulting_rip >= read_start && faulting_rip < read_end {
				let read_recovery = core::ptr::addr_of!(asm_copy_from_user_fault) as usize;

				stack_frame.as_mut().update(|frame| {
					frame.instruction_pointer = VirtAddr::new(read_recovery as u64);
				});
				return;
			}

			let write_start = core::ptr::addr_of!(asm_copy_to_user_dst_start) as usize;
			let write_end = core::ptr::addr_of!(asm_copy_to_user_dst_end) as usize;

			if faulting_rip >= write_start && faulting_rip < write_end {
				let write_recovery = core::ptr::addr_of!(asm_copy_to_user_fault) as usize;

				stack_frame.as_mut().update(|frame| {
					frame.instruction_pointer = VirtAddr::new(write_recovery as u64);
				});
				return;
			}

			let addr = Cr2::read();
			serial_println!("EXCEPTION: PAGE FAULT");
			serial_println!("Accessed Address: {:?}", addr);
			serial_println!("Error Code: {:?}", error_code);
			serial_println!("{:#?}", stack_frame);

			if IN_PAGE_FAULT.swap(true, Ordering::SeqCst) {
				serial_println!("[PF] re-entrant fault, skipping backtrace");
				hlt_loop();
			}

			trace_stack_trace(15);
		}
	}
}

kernel_interrupt! {
	fn general_protection_fault_handler(
		stack_frame: InterruptStackFrame,
		error_code: u64
	) {
		serial_println!("\n\nGENERAL PROTECTION FAULT");
		serial_println!("Error Code: {}", error_code);
		serial_println!("StackFrame: {:#?}", stack_frame);

		trace_stack_trace(10);

		hlt_loop();
	}
}

kernel_interrupt! {
	fn double_fault_handler(
		stack_frame: InterruptStackFrame,
		error_code: u64
	) -> ! {
		serial_println!("\n\nDOUBLE FAULT");
		serial_println!("Error Code: {}", error_code);
		serial_println!("StackFrame: {:#?}", stack_frame);

		trace_stack_trace(10);

		panic!("System halted");
	}
}

kernel_interrupt! {
	/// Keyboard interrupt handler.
	fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
		use ::x86_64::instructions::port::Port;

		let mut port = Port::new(0x60);
		let scancode: u8 = unsafe { port.read() };

		add_scancode(scancode);

		unsafe {
			send_eoi();
		}
	}
}

kernel_interrupt! {
	fn serial_input_interrupt_handler(_stack_frame: InterruptStackFrame) {
		use ::x86_64::instructions::port::Port;

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
			send_eoi();
		}
	}
}

/// Spurious interrupt handler (vector 0xFF).
extern "x86-interrupt" fn spurious_interrupt_handler(_stack_frame: InterruptStackFrame) {
	serial_println!("[WARNING] Spurious interrupt received (vector 0xFF)");
	// Per x86_64 spec: do NOT send EOI for spurious interrupts
}

/// APIC Timer Interrupt Handler.
///
/// This handler is invoked when the APIC timer fires.
#[unsafe(naked)]
extern "x86-interrupt" fn apic_timer_handler(_stack_frame: InterruptStackFrame) {
	core::arch::naked_asm!(
		"push r15",
		"push r14",
		"push r13",
		"push r12",
		"push r11",
		"push r10",
		"push r9",
		"push r8",
		"push rbp",
		"push rdi",
		"push rsi",
		"push rdx",
		"push rcx",
		"push rbx",
		"push rax",

		"mov rdi, rsp",
		// keep the stack aligned for the SysV ABI before entering the C shim.
		"sub rsp, 8",
		"call {scheduler_inner}",
		"add rsp, 8",

		"mov rsp, rax",

		"pop rax",
		"pop rbx",
		"pop rcx",
		"pop rdx",
		"pop rsi",
		"pop rdi",
		"pop rbp",
		"pop r8",
		"pop r9",
		"pop r10",
		"pop r11",
		"pop r12",
		"pop r13",
		"pop r14",
		"pop r15",

		"iretq",
		scheduler_inner = sym apic_timer_handler_inner,
	)
}

extern "C" fn apic_timer_handler_inner(current_rsp: u64) -> u64 {
	let _cr3_guard = unsafe { KernelCr3Guard::enter_interrupt() };

	APIC_TICK_COUNT.fetch_add(1, Ordering::SeqCst);
	unsafe {
		send_eoi();
	}
	current_rsp
}

kernel_interrupt! {
	fn rtc_timer_handler(_stack_frame: InterruptStackFrame) {
		unsafe {
			outb(CMOS_INDEX, REG_C);
			let _ = inb(CMOS_DATA);
		}

		RTC_TICKS.fetch_add(1, Ordering::SeqCst);

		unsafe {
			send_rtc_eoi();
		}
	}
}

// 64-BIT! currently.
#[unsafe(naked)]
extern "x86-interrupt" fn syscall_handler(_stack_frame: InterruptStackFrame) {
	core::arch::naked_asm!(
		// save argument registers
		"pushq %rdi",
		"pushq %rsi",
		"pushq %rdx",
		"pushq %r10",
		"pushq %r9",
		"pushq %r8",

		// shuffle args for the SysV C call
		"movq %r8, %r9",
		"movq %r10, %r8",
		"movq %rdx, %rcx",
		"movq %rsi, %rdx",
		"movq %rdi, %rsi",
		"movl %eax, %edi",

		//align to 16 bytes and call inner handler
		"subq $8, %rsp",
		"call {inner}",
		"addq $8, %rsp",

		// save user state
		"movq {current_guard}, %r11",          // r11 = *CURRENT_PROCESS_GUARD
		"testq %r11, %r11",
		"jz 3f",

		// CPU-pushed RIP is at [rsp + 0x30]
		"movq 0x30(%rsp), %r10",
		"movq %r10, {off_ctx_rip}(%r11)",

		// RFLAGS at [rsp + 0x40]
		"movq 0x40(%rsp), %r10",
		"movq %r10, {off_ctx_rflags}(%r11)",

		// user RSP at [rsp + 0x48]
		"movq 0x48(%rsp), %r10",
		"movq %r10, {off_ctx_rsp}(%r11)",

		// syscall return value into context.rax
		"movq %rax, {off_ctx_rax}(%r11)",

		"3:",
		// yield check
		"cmpb $0, {force_reschedule}",
		"je 2f",

		// clear flag
		"movb $0, {force_reschedule}",

		// switch back to kernel pages and then executor
		"movq {kernel_cr3}, %r11",
		"movq %r11, %cr3",
		"movq {kernel_rsp}, %rsp",
		"movq {kernel_rbp}, %rbp",
		"jmp *{kernel_ret}",

		// return to ring 3
		"2:",
		"popq %r9",
		"popq %r8",
		"popq %r10",
		"popq %rdx",
		"popq %rsi",
		"popq %rdi",
		"iretq",

		inner = sym syscall_handler_inner,
		current_guard = sym crate::task::executor::CURRENT_PROCESS_GUARD,
		kernel_cr3 = sym crate::arch::x86_64::user::KERNEL_CR3,
		kernel_rsp = sym crate::arch::x86_64::user::KERNEL_RETURN_RSP,
		kernel_rbp = sym crate::arch::x86_64::user::KERNEL_RETURN_RBP,
		kernel_ret = sym crate::arch::x86_64::user::KERNEL_RETURN_ADDR,
		force_reschedule = sym crate::task::FORCE_RESCHEDULE,
		off_ctx_rip = const {
			core::mem::offset_of!(crate::task::process::Process, context)
				+ core::mem::offset_of!(crate::task::context::UserContext, rip)
		},
		off_ctx_rsp = const {
			core::mem::offset_of!(crate::task::process::Process, context)
				+ core::mem::offset_of!(crate::task::context::UserContext, rsp)
		},
		off_ctx_rflags = const {
			core::mem::offset_of!(crate::task::process::Process, context)
				+ core::mem::offset_of!(crate::task::context::UserContext, rflags)
		},
		off_ctx_rax = const {
			core::mem::offset_of!(crate::task::process::Process, context)
				+ core::mem::offset_of!(crate::task::context::UserContext, rax)
		},
		options(att_syntax)
	)
}

extern "C" fn syscall_handler_inner(
	num: u32,
	a1: u64,
	a2: u64,
	a3: u64,
	a4: u64,
	a5: u64,
	a6: u64
) -> i32 {
	unsafe { syscall(num, a1, a2, a3, a4, a5, a6) }
}

kernel_interrupt! {
	fn kernel_assert_interrupt_handler(stack_frame: InterruptStackFrame) {
		serial_println!("[ASSERT FAILED] Kernel Halted at {:#x}", stack_frame.instruction_pointer);
		trace_stack_trace(10);
		kernel_panic!();
	}
}

// extern "x86-interrupt" fn gsi_interrupt_dispatcher(_stack_frame:
// InterruptStackFrame) { 	let mut handled = false;
// 	{
// 		let gt = GSI_TABLE.lock();
// 		for gsi in 0..16 {
// 			if let Some(handler) = gt[gsi].handler {
// 				// Call the registered handler through unsafe asm
// 				// since x86-interrupt ABI functions cannot be called directly
// 				unsafe {
// 					core::arch::asm!(
// 						"call {0}",
// 						in(reg) handler as *const (),
// 						in("rdi") &_stack_frame,
// 						options(nostack),
// 					);
// 				}
// 				handled = true;
// 				break; // Assume only one interrupt at a time
// 			}
// 		}
// 	}

// 	if !handled {
// 		serial_println!("[GSI] Unhandled interrupt!");
// 	}

// 	unsafe { send_eoi(); }
// }

/// Allocates and registers a vector to the IOAPIC
pub fn allocate_and_register_vector(
	handler: extern "x86-interrupt" fn(InterruptStackFrame)
) -> Result<usize, NullexError> {
	let mut idx = 48;
	let mut vec_table = VECTOR_TABLE.lock();

	while idx < 256 {
		if vec_table.get_idx(idx) {
			idx += 1;
			continue;
		} else {
			// add the idt entry here
			vec_table.set_idx(idx, true);
			drop(vec_table);
			if !IDT_INITED.load(Ordering::SeqCst) {
				panic!("Attempted to add IDT entry before IDT initialization");
			}
			unsafe { add_idt_entry(idx, handler) };
			return Ok(idx)
		}
	}

	Err(NullexError::VectorTableFull) // table full
}

#[cfg(feature = "test")]
pub mod tests {
	use core::sync::atomic::Ordering;

	use super::*;
	use crate::{tassert, tassert_eq, tassert_ne, testing::ktest::TestError};

	pub fn test_interrupt_vector_constants() -> Result<(), TestError> {
		tassert_eq!(APIC_TIMER_VECTOR, 0x20);
		tassert_eq!(KEYBOARD_VECTOR, 0x21);
		tassert_eq!(SERIAL_VECTOR, 0x24);
		tassert_eq!(KASSERT_VECTOR, 0x25);
		tassert_eq!(RTC_VECTOR, 0x28);
		tassert_eq!(SYSCALL_VECTOR, 0x80);
		Ok(())
	}
	crate::create_test!(test_interrupt_vector_constants);

	pub fn test_vector_table_initial_state() -> Result<(), TestError> {
		let table = VECTOR_TABLE.lock();

		for i in 0..31 {
			tassert!(table.get_idx(i), "Vector {} should be reserved", i);
		}

		tassert!(
			!table.get_idx(31),
			"Vector 31 should not be set by init logic"
		);

		tassert!(table.get_idx(255), "Vector 255 should be reserved");

		tassert!(!table.get_idx(48), "Vector 48 should be available");
		tassert!(!table.get_idx(100), "Vector 100 should be available");
		tassert!(!table.get_idx(254), "Vector 254 should be available");

		Ok(())
	}
	crate::create_test!(test_vector_table_initial_state);

	pub fn test_in_page_fault_initial_state() -> Result<(), TestError> {
		tassert!(
			!IN_PAGE_FAULT.load(Ordering::SeqCst),
			"IN_PAGE_FAULT should be false initially"
		);
		Ok(())
	}
	crate::create_test!(test_in_page_fault_initial_state);
}
