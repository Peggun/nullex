//!
//! src/arch/x86_64/user.rs
//!
//! x86_64 Usermode module for the kernel.

use alloc::{string::String, vec::Vec};
use core::{
	ptr::copy_nonoverlapping,
	sync::atomic::{AtomicBool, AtomicI32}
};

use x86_64::{
	VirtAddr,
	registers::control::Cr3,
	structures::{
		paging::{FrameAllocator, Mapper, OffsetPageTable, Page, PageTableFlags, PhysFrame},
		tss::TaskStateSegment
	}
};

use crate::{
	allocator::ALLOCATOR_INFO,
	arch::x86_64::bootinfo::{FrameRange, MemoryRegion, MemoryRegionType},
	error::NullexError,
	gdt::TSS,
	memory::{BootInfoFrameAllocator, PHYS_MEM_OFFSET, phys_to_virt},
	serial_println,
	task::{address_space::AddressSpace, context::UserContext, process::Process}
};

pub static USER_EXIT_REQUESTED: AtomicBool = AtomicBool::new(false);
pub static USER_EXIT_CODE: AtomicI32 = AtomicI32::new(0);

pub static mut KERNEL_RETURN_RSP: u64 = 0;
pub static mut KERNEL_RETURN_RBP: u64 = 0;
pub static mut KERNEL_RETURN_ADDR: u64 = 0;

pub const USER_STACK_TOP: u64 = 0x0000_7FFF_0000_0000;
const USER_STACK_PAGES: usize = 8;

const TRANSITION_STACK_SIZE: usize = 4096 * 4;

pub static mut KERNEL_CR3: u64 = 0;

#[repr(align(16))]
struct TransitionStack([u8; TRANSITION_STACK_SIZE]);

static mut TRANSITION_STACK: TransitionStack = TransitionStack([0; TRANSITION_STACK_SIZE]);

#[inline(always)]
unsafe fn transition_stack_top() -> u64 {
	let base = unsafe { core::ptr::addr_of!(TRANSITION_STACK.0) as *const u8 as u64 };
	base + TRANSITION_STACK_SIZE as u64
}

/// Write bytes to user stack using physical frame alias (NO CR3 SWITCHING)
unsafe fn push_bytes(stack_frames: &[(u64, PhysFrame)], sp: &mut u64, bytes: &[u8]) -> u64 {
	*sp -= bytes.len() as u64;

	let mut remaining = bytes;
	let mut cur = *sp;

	while !remaining.is_empty() {
		let page_base = cur & !0xFFF;
		let offset = (cur & 0xFFF) as usize;

		let (_, frame) = stack_frames
			.iter()
			.find(|(addr, _)| *addr == page_base)
			.expect("stack page not mapped");

		let frame_ptr = unsafe { phys_to_virt(frame.start_address()).as_mut_ptr::<u8>() };

		let to_copy = core::cmp::min(remaining.len(), 4096 - offset);

		unsafe {
			copy_nonoverlapping(remaining.as_ptr(), frame_ptr.add(offset), to_copy);
		}

		remaining = &remaining[to_copy..];
		cur += to_copy as u64;
	}

	*sp
}

/// Push a u64 using push_bytes
unsafe fn push_u64(stack_frames: &[(u64, PhysFrame)], sp: &mut u64, val: u64) {
	let bytes = val.to_le_bytes();
	unsafe { push_bytes(stack_frames, sp, &bytes) };
}

pub unsafe fn setup_user_stack(
	address_space: &mut AddressSpace,
	args: Vec<String>,
	envs: Vec<String>
) -> Result<u64, NullexError> {
	let mut fa_guard = ALLOCATOR_INFO.frame_allocator.lock();
	let fa_ref = fa_guard
		.as_mut()
		.ok_or(NullexError::FrameAllocatorNotInitialized)?;
	let fa: &mut BootInfoFrameAllocator = &mut **fa_ref;

	let table_ptr = unsafe { phys_to_virt(address_space.page_table.start_address()) };
	let mut mapper = unsafe {
		OffsetPageTable::new(&mut *table_ptr.as_mut_ptr(), VirtAddr::new(PHYS_MEM_OFFSET))
	};

	let stack_top = VirtAddr::new(USER_STACK_TOP);
	let stack_size = 4096 * USER_STACK_PAGES;
	let stack_bottom = stack_top - stack_size as u64;

	let start_page = Page::containing_address(stack_bottom);
	let end_page = Page::containing_address(VirtAddr::new(stack_top.as_u64() - 1));

	let flags = PageTableFlags::PRESENT
		| PageTableFlags::USER_ACCESSIBLE
		| PageTableFlags::WRITABLE
		| PageTableFlags::NO_EXECUTE;

	let mut stack_frames: Vec<(u64, PhysFrame)> = Vec::new();

	for page in Page::range_inclusive(start_page, end_page) {
		let frame = fa
			.allocate_frame()
			.ok_or(NullexError::FrameAllocationFailed)?;
		unsafe {
			mapper.map_to(page, frame, flags, fa)?.flush();
		}
		stack_frames.push((page.start_address().as_u64(), frame));
		address_space.owned_frames.push(frame);
	}

	let mut sp = USER_STACK_TOP;

	let mut arg_ptrs: Vec<u64> = Vec::with_capacity(args.len());
	let mut env_ptrs: Vec<u64> = Vec::with_capacity(envs.len());

	for s in envs.iter().rev() {
		let mut buf = s.as_bytes().to_vec();
		buf.push(0);
		let addr = unsafe { push_bytes(&stack_frames.as_slice(), &mut sp, &buf.as_slice()) };
		env_ptrs.push(addr);
	}

	for s in args.iter().rev() {
		let mut buf = s.as_bytes().to_vec();
		buf.push(0);
		let addr = unsafe { push_bytes(&stack_frames.as_slice(), &mut sp, &buf.as_slice()) };
		arg_ptrs.push(addr);
	}

	sp &= !0xF;

	let total_pushes = envs.len() + args.len() + 3;
	if total_pushes % 2 == 0 {
		unsafe { push_u64(&stack_frames.as_slice(), &mut sp, 0) };
	}

	unsafe { push_u64(&stack_frames.as_slice(), &mut sp, 0) };
	for &ptr in env_ptrs.iter() {
		unsafe { push_u64(&stack_frames.as_slice(), &mut sp, ptr) };
	}
	unsafe { push_u64(&stack_frames.as_slice(), &mut sp, 0) };
	for &ptr in arg_ptrs.iter() {
		unsafe { push_u64(&stack_frames.as_slice(), &mut sp, ptr) };
	}
	unsafe { push_u64(&stack_frames.as_slice(), &mut sp, args.len() as u64) };

	address_space.regions.push(MemoryRegion {
		range: FrameRange::new(stack_top.as_u64(), stack_bottom.as_u64()),
		region_type: MemoryRegionType::InUse
	});

	Ok(sp)
}

pub unsafe fn enter_user_process(process: &Process) {
	serial_println!("process: {}", process.state.id.get());

	let address_space = process
		.address_space
		.as_ref()
		.expect("attempted to enter_user_process on a kernel process");

	unsafe {
		let tss = &*TSS as *const TaskStateSegment as *mut TaskStateSegment;
		(*tss).privilege_stack_table[0] = VirtAddr::new(process.kernel_stack_top);
	}

	let trampoline_sp = unsafe { transition_stack_top() };

	unsafe {
		KERNEL_CR3 = x86_64::registers::control::Cr3::read()
			.0
			.start_address()
			.as_u64();
	}

	let cur = Cr3::read().0.start_address().as_u64();
	unsafe {
		let k_ptr = core::ptr::addr_of_mut!(KERNEL_CR3);
		let val = core::ptr::addr_of!(KERNEL_CR3).read();
		if val == 0 {
			k_ptr.write(cur);
		} else if val != cur {
			serial_println!("[FATAL] KERNEL_CR3 drift: {:#x} != {:#x}", val, cur);
			loop {}
		}
	}

	serial_println!(
		"[INFO] About to iretq: rip={:#x} rsp={:#x} cs={:#x} ss={:#x}",
		process.context.rip,
		process.context.rsp,
		process.context.cs,
		process.context.ss,
	);

	serial_println!(
		"[DEBUG] rflags={:#x}, IF bit={}",
		process.context.rflags,
		(process.context.rflags >> 9) & 1
	);

	serial_println!("=== IRETQ FRAME DEBUG ===");
	serial_println!("RIP:    {:#x}", process.context.rip);
	serial_println!(
		"CS:     {:#x} (Must end in 3, e.g. 0x1B/0x23)",
		crate::gdt::user_code_selector() as u64
	);
	serial_println!(
		"RFLAGS: {:#x}",
		process.context.rflags | (1u64 << 1) | (1u64 << 9)
	);
	serial_println!("RSP:    {:#x}", process.context.rsp);
	serial_println!(
		"SS:     {:#x} (Must end in 3, e.g. 0x1B/0x23)",
		crate::gdt::user_data_selector() as u64
	);
	serial_println!("=========================");

	unsafe {
		core::arch::asm!(
			"cli",

			// 1. Save kernel return state
			"lea {ret_addr}, [rip + 2f]",
			"mov [{krsp}], rsp",
			"mov [{krbp}], rbp",
			"mov [{kret}], {ret_addr}",

			// 2. Switch to trampoline stack
			"mov rsp, {tramp_sp}",

			// 3. PUSH THE IRETQ FRAME FIRST!
			// This places it at the bottom of our stack layout (higher addresses).
			"push {ss}",
			"push {user_rsp}",
			"push {rflags}",
			"push {cs}",
			"push {rip}",

			// 4. Push all user registers from the context struct.
			// These sit on top of the iretq frame (lower addresses).
			// CRITICAL: This MUST happen BEFORE switching CR3, because {ctx} lives on the
			// kernel heap, which is NOT mapped in the user page tables!
			"push qword ptr [{ctx} + {off_r15}]",
			"push qword ptr [{ctx} + {off_r14}]",
			"push qword ptr [{ctx} + {off_r13}]",
			"push qword ptr [{ctx} + {off_r12}]",
			"push qword ptr [{ctx} + {off_r11}]",
			"push qword ptr [{ctx} + {off_r10}]",
			"push qword ptr [{ctx} + {off_r9}]",
			"push qword ptr [{ctx} + {off_r8}]",
			"push qword ptr [{ctx} + {off_rbp}]",
			"push qword ptr [{ctx} + {off_rdi}]",
			"push qword ptr [{ctx} + {off_rsi}]",
			"push qword ptr [{ctx} + {off_rdx}]",
			"push qword ptr [{ctx} + {off_rcx}]",
			"push qword ptr [{ctx} + {off_rbx}]",
			"push qword ptr [{ctx} + {off_rax}]",

			// 5. Switch CR3 to the user page tables
			"mov cr3, {cr3}",

			// 6. Pop all user registers into their actual CPU registers
			// This consumes the general purpose registers, leaving RSP pointing
			// exactly at the iretq frame we pushed in step 3.
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

			// 7. Jump to user space!
			// The CPU pops the iretq frame correctly and enters Ring 3.
			"iretq",

			"2:",
			ret_addr = out(reg) _,
			krsp = in(reg) core::ptr::addr_of_mut!(KERNEL_RETURN_RSP),
			krbp = in(reg) core::ptr::addr_of_mut!(KERNEL_RETURN_RBP),
			kret = in(reg) core::ptr::addr_of_mut!(KERNEL_RETURN_ADDR),
			tramp_sp = in(reg) trampoline_sp,
			cr3 = in(reg) address_space.page_table.start_address().as_u64(),
			user_rsp = in(reg) process.context.rsp,
			ss = in(reg) (crate::gdt::user_data_selector() | 3) as u64, // Ensure RPL=3
			rflags = in(reg) process.context.rflags | (1u64 << 1) | (1u64 << 9),
			cs = in(reg) (crate::gdt::user_code_selector() | 3) as u64,  // Ensure RPL=3
			rip = in(reg) process.context.rip,
			ctx = in(reg) &process.context as *const UserContext,
			off_rax = const core::mem::offset_of!(UserContext, rax),
			off_rbx = const core::mem::offset_of!(UserContext, rbx),
			off_rcx = const core::mem::offset_of!(UserContext, rcx),
			off_rdx = const core::mem::offset_of!(UserContext, rdx),
			off_rsi = const core::mem::offset_of!(UserContext, rsi),
			off_rdi = const core::mem::offset_of!(UserContext, rdi),
			off_r8 = const core::mem::offset_of!(UserContext, r8),
			off_r9 = const core::mem::offset_of!(UserContext, r9),
			off_r10 = const core::mem::offset_of!(UserContext, r10),
			off_r11 = const core::mem::offset_of!(UserContext, r11),
			off_r12 = const core::mem::offset_of!(UserContext, r12),
			off_r13 = const core::mem::offset_of!(UserContext, r13),
			off_r14 = const core::mem::offset_of!(UserContext, r14),
			off_r15 = const core::mem::offset_of!(UserContext, r15),
			off_rbp = const core::mem::offset_of!(UserContext, rbp)
		);
	}
}

// some asm to check for valid user writes
// i to be honest dont really understand this, i wish there was a better safer
// way but apparantly there isnt and google ai told me this :) dont worry,
// everything else is manually coded by me
// it just creates synbols for the page fault handler to read.
// i swear everything else is written by me
core::arch::global_asm!(
	// --- EXPORTS ---
	".global asm_copy_from_user",
	".global asm_copy_from_user_src_start",
	".global asm_copy_from_user_src_end",
	".global asm_copy_from_user_fault",
	".global asm_copy_to_user",
	".global asm_copy_to_user_dst_start",
	".global asm_copy_to_user_dst_end",
	".global asm_copy_to_user_fault",
	// =========================================================================
	// 1. COPY FROM USER (READ)
	// Parameters: rdi = destination (kernel), rsi = source (user), rdx = length
	// Returns:    rax = bytes successfully transferred
	// =========================================================================
	".align 16",
	"asm_copy_from_user:",
	"    xor %rax, %rax",  // Clear byte counter
	"    test %rdx, %rdx", // Exit immediately if size is zero
	"    jz .L_from_user_done",
	"    stac", // Enable user-space access (SMAP safety bypass)
	".L_from_user_loop:",
	"asm_copy_from_user_src_start:",
	"    movb (%rsi), %r8b", // CRITICAL CRASH ZONE: Read from user memory
	"asm_copy_from_user_src_end:",
	"    movb %r8b, (%rdi)", // Store byte safely into kernel space
	"    inc %rsi",
	"    inc %rdi",
	"    inc %rax", // Log successful byte read
	"    dec %rdx",
	"    jnz .L_from_user_loop",
	"    clac", // Lock user-space access back down
	".L_from_user_done:",
	"    ret",
	"asm_copy_from_user_fault:",
	"    clac", // Always reset SMAP tracking on exception
	"    ret",  // Returns rax (shorter than expected size)
	// =========================================================================
	// 2. COPY TO USER (WRITE & MUTABLE SLICES)
	// Parameters: rdi = destination (user), rsi = source (kernel), rdx = length
	// Returns:    rax = bytes successfully transferred
	// =========================================================================
	".align 16",
	"asm_copy_to_user:",
	"    xor %rax, %rax",  // Clear byte counter
	"    test %rdx, %rdx", // Exit immediately if size is zero
	"    jz .L_to_user_done",
	"    stac", // Enable user-space access (SMAP safety bypass)
	".L_to_user_loop:",
	"    movb (%rsi), %r8b", // Read byte safely from kernel space
	"asm_copy_to_user_dst_start:",
	"    movb %r8b, (%rdi)", // CRITICAL CRASH ZONE: Write to user memory
	"asm_copy_to_user_dst_end:",
	"    inc %rsi",
	"    inc %rdi",
	"    inc %rax", // Log successful byte write
	"    dec %rdx",
	"    jnz .L_to_user_loop",
	"    clac", // Lock user-space access back down
	".L_to_user_done:",
	"    ret",
	"asm_copy_to_user_fault:",
	"    clac", // Always reset SMAP tracking on exception
	"    ret",  // Returns rax (shorter than expected size)
	options(att_syntax)
);

unsafe extern "C" {
	pub unsafe fn asm_copy_from_user(dst: *mut u8, src: *const u8, len: usize) -> usize;
	pub unsafe static asm_copy_from_user_src_start: u8;
	pub unsafe static asm_copy_from_user_src_end: u8;
	pub unsafe static asm_copy_from_user_fault: u8;

	pub unsafe fn asm_copy_to_user(dst: *mut u8, src: *const u8, len: usize) -> usize;
	pub unsafe static asm_copy_to_user_dst_start: u8;
	pub unsafe static asm_copy_to_user_dst_end: u8;
	pub unsafe static asm_copy_to_user_fault: u8;
}
