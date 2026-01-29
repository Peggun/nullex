// interrupts.rs

/*
Interrupt handling module for the kernel.
*/

use core::{
	arch::asm,
	mem::MaybeUninit,
	sync::atomic::{AtomicBool, Ordering}
};

use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

use crate::{
	apic::{TICK_COUNT, send_eoi},
	common::ports::{inb, outb},
	drivers::{
		keyboard::queue::add_scancode,
		virtio::net::{VIRTIO_NET_IDT_VECTOR, virtio_net_interrupt_handler}
	},
	gdt,
	lazy_static,
	println,
	rtc::{
		CMOS_DATA,
		CMOS_INDEX,
		NMI_BIT,
		PIC_EOI,
		PIC1_CMD,
		PIC2_CMD,
		REG_C,
		RTC_TICKS,
		send_rtc_eoi
	},
	serial::add_byte,
	serial_println,
	syscall::syscall,
	task::executor::CURRENT_PROCESS,
	utils::{bits::BitMap, mutex::SpinMutex},
	hlt_loop,
};

pub const APIC_TIMER_VECTOR: u8 = 32;
pub const KEYBOARD_VECTOR: u8 = 33;
pub const SERIAL_VECTOR: u8 = 36;
pub const RTC_VECTOR: u8 = 0x70; // irq 8 - 15 is mapped from 0x70 to 0x77;
pub const SYSCALL_VECTOR: u8 = 0x80;

// TODO: remove the maybeuninit, just move to a safe lazy_static!
pub static mut IDT_STORAGE: MaybeUninit<InterruptDescriptorTable> = MaybeUninit::uninit();
pub static IDT_INITED: AtomicBool = AtomicBool::new(false);

lazy_static! {
	// bitmap
	pub static ref VECTOR_TABLE: SpinMutex<BitMap> = {
		let mut bmp = BitMap::new(256);
		bmp.set_idxs((0..31).into(), true);
		bmp.set_idx(255, true);
		SpinMutex::new(bmp)
	};
}

pub fn init_idt() {
	unsafe {
		x86_64::instructions::interrupts::disable();

		let mut local_idt = InterruptDescriptorTable::new();

		// Exception handlers
		local_idt.breakpoint.set_handler_fn(breakpoint_handler);
		local_idt.page_fault.set_handler_fn(page_fault_handler);
		local_idt
			.double_fault
			.set_handler_fn(double_fault_handler)
			.set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);

		// driver handlers
		local_idt[APIC_TIMER_VECTOR as usize].set_handler_fn(apic_timer_handler);
		local_idt[KEYBOARD_VECTOR as usize].set_handler_fn(keyboard_interrupt_handler);
		local_idt[SERIAL_VECTOR as usize].set_handler_fn(serial_input_interrupt_handler);
		local_idt[RTC_VECTOR as usize].set_handler_fn(rtc_timer_handler);

		// syscall handler
		local_idt[SYSCALL_VECTOR as usize].set_handler_fn(syscall_handler);

		// virtio handlers
		local_idt[VIRTIO_NET_IDT_VECTOR as usize].set_handler_fn(virtio_net_interrupt_handler);

		// Spurious interrupt handler
		local_idt[0xFF].set_handler_fn(spurious_interrupt_handler);

		let storage_ptr: *mut MaybeUninit<InterruptDescriptorTable> =
			core::ptr::addr_of_mut!(IDT_STORAGE);
		let idt_ptr = storage_ptr as *mut InterruptDescriptorTable;
		core::ptr::write(idt_ptr, local_idt);
		let idt_ref: &InterruptDescriptorTable = &*idt_ptr;
		idt_ref.load();

		IDT_INITED.store(true, Ordering::SeqCst);
		// CRITICAL: Keep interrupts DISABLED here. Enable only after
		// APIC/IOAPIC init in kernel_main. Enabling too early causes
		// interrupt handlers to read APIC registers at uninitialized address 0.
	}
}

pub unsafe fn add_idt_entry(
	vector: usize,
	handler: extern "x86-interrupt" fn(InterruptStackFrame)
) { unsafe {
	x86_64::instructions::interrupts::without_interrupts(|| {
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

/// Double fault handler.
extern "x86-interrupt" fn double_fault_handler(
	stack_frame: InterruptStackFrame,
	error_code: u64
) -> ! {
	println!("\n\nDOUBLE FAULT");
	println!("Error Code: {}", error_code);
	println!("StackFrame: {:#?}", stack_frame);
	panic!("System halted");
}

/// Keyboard interrupt handler.
extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
	use x86_64::instructions::port::Port;

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

	// Send EOI via APIC instead of PIC
	unsafe {
		send_eoi();
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

	// Send EOI via APIC
	unsafe {
		send_eoi();
	}
}

/// Spurious interrupt handler (vector 0xFF).
/// Called when a spurious interrupt is received.
extern "x86-interrupt" fn spurious_interrupt_handler(_stack_frame: InterruptStackFrame) {
	serial_println!("[WARNING] Spurious interrupt received (vector 0xFF)");
	// Per x86_64 spec: do NOT send EOI for spurious interrupts
}

/// Page fault handler.
extern "x86-interrupt" fn page_fault_handler(
	stack_frame: InterruptStackFrame,
	error_code: PageFaultErrorCode
) {
	#[cfg(not(feature = "test"))]
	{
		use x86_64::registers::control::Cr2;

		println!("EXCEPTION: PAGE FAULT");
		println!("Accessed Address: {:?}", Cr2::read());
		println!("Error Code: {:?}", error_code);
		println!("{:#?}", stack_frame);

		hlt_loop();
	}
	#[cfg(feature = "test")]
	{
		use x86_64::registers::control::Cr2;

		use crate::qemu_exit;

		serial_println!("EXCEPTION: PAGE FAULT");
		serial_println!("Accessed Address: {:?}", Cr2::read());
		serial_println!("Error Code: {:?}", error_code);
		serial_println!("{:#?}", stack_frame);
		qemu_exit(1)
	}
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

/// Defines the interrupt vectors used in the IDT.
// uses APIC now.
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptVector {
	Timer = APIC_TIMER_VECTOR,
	Keyboard = KEYBOARD_VECTOR,
	Serial = SERIAL_VECTOR,
	Syscall = SYSCALL_VECTOR
}

impl InterruptVector {
	pub fn as_u8(self) -> u8 {
		self as u8
	}

	pub fn as_usize(self) -> usize {
		self.as_u8() as usize
	}
}

// allocate and register return the vector
pub fn allocate_and_register_vector(
	handler: extern "x86-interrupt" fn(InterruptStackFrame)
) -> Result<usize, &'static str> {
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

	Err("vector table full") // table full
}
