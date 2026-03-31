//!
//! interrupts.rs
//!
//! Interrupt handling module for the kernel.
//!

use core::{
	mem::MaybeUninit,
	sync::atomic::{AtomicBool, Ordering}
};

use ::x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

use crate::{
	apic::{APIC_TICK_COUNT, PIC_EOI, PIC1_CMD, PIC2_CMD, send_eoi}, common::ports::{inb, outb}, drivers::keyboard::queue::add_scancode, error::NullexError, gdt, hlt_loop, lazy_static, println, rtc::{
		CMOS_DATA,
		CMOS_INDEX,
		NMI_BIT,
		REG_C,
		RTC_TICKS,
		send_rtc_eoi
	}, serial::add_byte, serial_println, syscall::syscall, task::executor::CURRENT_PROCESS, utils::{bits::BitMap, mutex::SpinMutex}
};

pub(crate) const APIC_TIMER_VECTOR: u8 = 32;
const KEYBOARD_VECTOR: u8 = 33;
const SERIAL_VECTOR: u8 = 36;
const RTC_VECTOR: u8 = 0x70; // irq 8 - 15 is mapped from 0x70 to 0x77;
const SYSCALL_VECTOR: u8 = 0x80;

// TODO: remove the maybeuninit, just move to a safe lazy_static!
static mut IDT_STORAGE: MaybeUninit<InterruptDescriptorTable> = MaybeUninit::uninit();
static IDT_INITED: AtomicBool = AtomicBool::new(false);

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
		local_idt.page_fault.set_handler_fn(page_fault_handler);
		local_idt
			.double_fault
			.set_handler_fn(double_fault_handler)
			.set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
		local_idt.general_protection_fault.set_handler_fn(general_protection_fault_handler);


		// driver handlers
		local_idt[APIC_TIMER_VECTOR as usize].set_handler_fn(apic_timer_handler);
		local_idt[KEYBOARD_VECTOR as usize].set_handler_fn(keyboard_interrupt_handler);
		local_idt[SERIAL_VECTOR as usize].set_handler_fn(serial_input_interrupt_handler);
		local_idt[RTC_VECTOR as usize].set_handler_fn(rtc_timer_handler);

		// syscall handler
		local_idt[SYSCALL_VECTOR as usize].set_handler_fn(syscall_handler)
    		.set_privilege_level(::x86_64::PrivilegeLevel::Ring3);

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
) { unsafe {
	::x86_64::instructions::interrupts::without_interrupts(|| {
		let storage_ptr: *mut MaybeUninit<InterruptDescriptorTable> =
			core::ptr::addr_of_mut!(IDT_STORAGE);
		let idt_ptr = storage_ptr as *mut InterruptDescriptorTable;
		let idt_ref: &mut InterruptDescriptorTable = &mut *idt_ptr;

		idt_ref[vector].set_handler_fn(handler);
		idt_ref.load();
	});
}}

/// Breakpoint exception handler.
extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
	println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode
) {
    use ::x86_64::registers::control::Cr2;

    let addr = Cr2::read();
    serial_println!("EXCEPTION: PAGE FAULT");
    serial_println!("Accessed Address: {:?}", addr);
    serial_println!("Error Code: {:?}", error_code);
    serial_println!("{:#?}", stack_frame);

    println!("EXCEPTION: PAGE FAULT");
    println!("Accessed Address: {:?}", addr);
    println!("Error Code: {:?}", error_code);
    println!("{:#?}", stack_frame);

    hlt_loop();
}

extern "x86-interrupt" fn general_protection_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    serial_println!("\n\nGENERAL PROTECTION FAULT");
    serial_println!("Error Code: {}", error_code);
    serial_println!("StackFrame: {:#?}", stack_frame);

    println!("\n\nGENERAL PROTECTION FAULT");
    println!("Error Code: {}", error_code);
    println!("StackFrame: {:#?}", stack_frame);

    panic!("System halted");
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64
) -> ! {
    serial_println!("\n\nDOUBLE FAULT");
    serial_println!("Error Code: {}", error_code);
    serial_println!("StackFrame: {:#?}", stack_frame);

    println!("\n\nDOUBLE FAULT");
    println!("Error Code: {}", error_code);
    println!("StackFrame: {:#?}", stack_frame);

    panic!("System halted");
}

/// Keyboard interrupt handler.
extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
	use ::x86_64::instructions::port::Port;

	let mut port = Port::new(0x60);
	let scancode: u8 = unsafe { port.read() };

	{
		let mut lock = CURRENT_PROCESS.lock();
		if let Some(proc) = lock.as_mut() {
			if let Ok(queue) = proc.scancode_queue.try_get()
				&& queue.push(scancode).is_ok()
			{
				proc.waker.wake();
			}
		} else {
			add_scancode(scancode);
		}
	}

	unsafe {
		send_eoi();
	}
}

extern "x86-interrupt" fn serial_input_interrupt_handler(_stack_frame: InterruptStackFrame) {
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

/// Spurious interrupt handler (vector 0xFF).
extern "x86-interrupt" fn spurious_interrupt_handler(_stack_frame: InterruptStackFrame) {
	serial_println!("[WARNING] Spurious interrupt received (vector 0xFF)");
	// Per x86_64 spec: do NOT send EOI for spurious interrupts
}


/// APIC Timer Interrupt Handler.
///
/// This handler is invoked when the APIC timer fires.
extern "x86-interrupt" fn apic_timer_handler(_stack_frame: InterruptStackFrame) {
	APIC_TICK_COUNT.fetch_add(1, Ordering::Relaxed);
	unsafe {
		send_eoi();
	}
}

extern "x86-interrupt" fn rtc_timer_handler(_stack_frame: InterruptStackFrame) {
	// ack
	unsafe {
		outb(CMOS_INDEX, REG_C | NMI_BIT);
		let _ = inb(CMOS_DATA);
	}

	RTC_TICKS.fetch_add(1, Ordering::Relaxed);

	unsafe {
		outb(PIC2_CMD, PIC_EOI);
		outb(PIC1_CMD, PIC_EOI);
		send_rtc_eoi();
	}
}

// 64-BIT! currently.
#[unsafe(naked)]
extern "x86-interrupt" fn syscall_handler(_stack_frame: InterruptStackFrame) {
    core::arch::naked_asm!(
        // save what we are about to clobber during the arg shuffle
        "push rdi",
        "push rsi",
        "push rdx",
        // shuffle (rax, rdi, rsi, rdx) -> (rdi, rsi, rdx, rcx) for SysV inner call
        "mov rcx, rdx",
        "mov rdx, rsi",
        "mov rsi, rdi",
        "mov edi, eax",
        // Stack accounting:
        // CPU pushed 5 qwords (40), we pushed 3 (24), total 64. 64%16=0.
        // 'call' will push 8 more -> misaligned, so sub 8 first.
        "sub rsp, 8",
        "call {inner}",
        "add rsp, 8",
        // restore user registers (in reverse)
        "pop rdx",
        "pop rsi",
        "pop rdi",
        "iretq",
        inner = sym syscall_handler_inner,
    )
}

extern "C" fn syscall_handler_inner(num: u32, a1: u64, a2: u64, a3: u64) -> i32 {
    unsafe { syscall(num, a1, a2, a3, 0, 0) }
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
