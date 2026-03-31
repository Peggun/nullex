//!
//! user.rs
//! 
//! x86_64 Usermode module for the kernel.
//! 

use core::{ptr::copy_nonoverlapping, sync::atomic::{AtomicBool, AtomicI32}};

use alloc::vec::Vec;
use x86_64::{
    VirtAddr,
    structures::paging::{FrameAllocator, Mapper, OffsetPageTable, Page, PageTableFlags, PhysFrame},
};

use crate::{
    PHYS_MEM_OFFSET, allocator::ALLOCATOR_INFO, arch::x86_64::bootinfo::{FrameRange, MemoryRegion, MemoryRegionType}, memory::{BootInfoFrameAllocator, phys_to_virt}, serial_println, task::{AddressSpace, Process}
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
unsafe fn push_bytes(
    stack_frames: &[(u64, PhysFrame)],
    sp: &mut u64,
    bytes: &[u8],
) -> u64 {
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
            copy_nonoverlapping(
                remaining.as_ptr(),
                frame_ptr.add(offset),
                to_copy,
            );
        }

        remaining = &remaining[to_copy..];
        cur += to_copy as u64;
    }

    *sp
}

/// Push a u64 using push_bytes
unsafe fn push_u64(
    stack_frames: &[(u64, PhysFrame)],
    sp: &mut u64,
    val: u64,
) {
    let bytes = val.to_le_bytes();
    unsafe { push_bytes(stack_frames, sp, &bytes) };
}


pub unsafe fn setup_user_stack(
    address_space: &mut AddressSpace,
    args: &[&str],
    envs: &[&str],
) -> u64 {
    let mut fa_guard = ALLOCATOR_INFO.frame_allocator.lock();
    let fa_ref = fa_guard.as_mut().unwrap();
    let fa: &mut BootInfoFrameAllocator = &mut **fa_ref;

    let table_ptr = unsafe { phys_to_virt(address_space.page_table.start_address()) };
    let mut mapper = unsafe { OffsetPageTable::new(
        &mut *table_ptr.as_mut_ptr(),
        *PHYS_MEM_OFFSET.lock(),
    ) };

    let stack_top = VirtAddr::new(USER_STACK_TOP);
    let stack_size = 4096 * USER_STACK_PAGES;
    let stack_bottom = stack_top - stack_size as u64;

    let start_page = Page::containing_address(stack_bottom);
    let end_page = Page::containing_address(VirtAddr::new(stack_top.as_u64() - 1));

    let flags = PageTableFlags::PRESENT
        | PageTableFlags::USER_ACCESSIBLE
        | PageTableFlags::WRITABLE
        | PageTableFlags::NO_EXECUTE;

    // Track mapped pages → frames
    let mut stack_frames: Vec<(u64, PhysFrame)> = Vec::new();

    for page in Page::range_inclusive(start_page, end_page) {
        let frame = fa.allocate_frame().expect("out of frames for user stack");

        unsafe {
            mapper
                .map_to(page, frame, flags, fa)
                .expect("failed to map user stack")
                .flush();
        }

        stack_frames.push((page.start_address().as_u64(), frame));
    }

    let mut sp = USER_STACK_TOP;

    let mut arg_ptrs: Vec<u64> = Vec::with_capacity(args.len());
    let mut env_ptrs: Vec<u64> = Vec::with_capacity(envs.len());

    // Write env strings
    for s in envs.iter().rev() {
        let mut buf = s.as_bytes().to_vec();
        buf.push(0);
        let addr = unsafe { push_bytes(&stack_frames.as_slice(), &mut sp, &buf.as_slice()) };
        env_ptrs.push(addr);
    }

    // Write arg strings
    for s in args.iter().rev() {
        let mut buf = s.as_bytes().to_vec();
        buf.push(0);
        let addr = unsafe { push_bytes(&stack_frames.as_slice(), &mut sp, &buf.as_slice()) };
        arg_ptrs.push(addr);
    }

    // Align stack
    sp &= !0xF;

    // envp NULL
    unsafe { push_u64(&stack_frames.as_slice(), &mut sp, 0) };

    // envp pointers
    for &ptr in env_ptrs.iter().rev() {
        unsafe { push_u64(&stack_frames.as_slice(), &mut sp, ptr) };
    }

    // argv NULL
    unsafe { push_u64(&stack_frames.as_slice(), &mut sp, 0) };

    // argv pointers
    for &ptr in arg_ptrs.iter().rev() {
        unsafe { push_u64(&stack_frames.as_slice(), &mut sp, ptr) };
    }

    // argc
    unsafe { push_u64(&stack_frames.as_slice(), &mut sp, args.len() as u64) };

    address_space.regions.push(MemoryRegion {
        range: FrameRange::new(stack_top.as_u64(), stack_bottom.as_u64()),
        region_type: MemoryRegionType::InUse,
    });

    sp
}


pub unsafe fn enter_user_process(process: &Process) {
    let address_space = process
        .address_space
        .as_ref()
        .expect("attempted to enter_user_process on a kernel process");

    let trampoline_sp = unsafe { transition_stack_top() };

    unsafe {
        KERNEL_CR3 = x86_64::registers::control::Cr3::read()
            .0.start_address().as_u64();
    }

    serial_println!("[INFO] About to iretq: rip={:#x} rsp={:#x} cs={:#x} ss={:#x}",
        process.context.rip,
        process.context.rsp,
        process.context.cs,
        process.context.ss,
    );
    unsafe {
        core::arch::asm!(
            "cli",

            "lea {ret_addr}, [rip + 2f]",
            "mov [{krsp}], rsp",
            "mov [{krbp}], rbp",
            "mov [{kret}], {ret_addr}",

            "mov rsp, {tramp_sp}",
            "push {ss}",
            "push {user_rsp}",
            "push {rflags}",
            "push {cs}",
            "push {rip}",
            "mov cr3, {cr3}",
            "iretq",

            "2:",
            ret_addr = out(reg) _,
            krsp = in(reg) core::ptr::addr_of_mut!(KERNEL_RETURN_RSP),
            krbp = in(reg) core::ptr::addr_of_mut!(KERNEL_RETURN_RBP),
            kret = in(reg) core::ptr::addr_of_mut!(KERNEL_RETURN_ADDR),
            tramp_sp = in(reg) trampoline_sp,
            cr3 = in(reg) address_space.page_table.start_address().as_u64(),
            user_rsp = in(reg) process.context.rsp,
            ss = in(reg) crate::gdt::user_data_selector() as u64,
            rflags = in(reg) process.context.rflags,
            cs = in(reg) crate::gdt::user_code_selector() as u64,
            rip = in(reg) process.context.rip,
        );
    }
}