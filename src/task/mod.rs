//!
//! src/task/mod.rs
//! 
//! Module definition for the task handling for the kernel.
//! 

pub mod executor;
pub mod keyboard;

use alloc::{boxed::Box, string::String, sync::Arc, vec::Vec};
use x86_64::{VirtAddr, structures::paging::{FrameAllocator, Mapper, OffsetPageTable, Page, PageTable, PageTableFlags, PhysFrame, Size4KiB, Translate}};
use core::{
	arch::asm, fmt::Debug, future::Future, pin::Pin, ptr::write_bytes, sync::atomic::AtomicBool, task::{Context, Poll}
};

use crossbeam_queue::ArrayQueue;
use futures::task::AtomicWaker;
use hashbrown::HashMap;

use crate::{PHYS_MEM_OFFSET, allocator::ALLOCATOR_INFO, arch::x86_64::{bootinfo::MemoryRegion, user::setup_user_stack}, error::NullexError, gdt::{INTERRUPT_STACK_SIZE, interrupt_stack_top, user_code_selector, user_data_selector}, memory::{active_level_4_table, phys_to_virt}, serial_println, utils::{elf::{load_segment, parse_elf}, oncecell::spin::OnceCell}};

const KERNEL_STACK_PAGES_TO_MAP: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
/// Wrapper for a process id.
pub struct ProcessId(u64);

impl ProcessId {
	/// Creates a new `ProcessId` with the specified id.
	pub fn new(id: u64) -> Self {
		ProcessId(id)
	}

	/// Returns the `ProcessId`'s id.
	pub fn get(&self) -> u64 {
		self.0
	}
}

/// Struct to represent an open file in a process
pub struct OpenFile {
	/// The path to the open file.
	pub path: String,
	/// The current read offset to the open file.
	pub offset: usize
}

#[expect(clippy::type_complexity)]
/// Structure representing all information of a current processes state.
pub struct ProcessState {
	/// The current `ProcessId`
	pub id: ProcessId,
	/// Whether or not the running process is a child of another process.
	pub is_child: bool,
	/// The function that this process will be running.
	pub future_fn:
		Arc<dyn Fn(Arc<ProcessState>) -> Pin<Box<dyn Future<Output = i32>>> + Send + Sync>,
	/// Whether or not is it in the queued inside of the executor.
	pub queued: AtomicBool,
	/// Scancode queue incase some functions need the keyboard.
	pub scancode_queue: OnceCell<ArrayQueue<u8>>,
	/// Waker for functions that need the process now.
	pub waker: AtomicWaker
}

/// Structure representing a process running in the kernel.
pub struct Process {
	/// Current state of the process running.
	pub state: Arc<ProcessState>,
	/// The code that is running inside of the process.
	pub future: Pin<Box<dyn Future<Output = i32>>>,
	/// Registers saved from User Processes
	pub context: UserContext,
	/// Address space for the process
	pub address_space: Option<AddressSpace>,
	/// The File Descriptor to the `OpenFile` mapping.
	pub open_files: HashMap<u32, OpenFile>,
	/// The next available file descriptor.
	pub next_fd: u32,
}

impl Process {
	/// Creates a new process.
	pub fn new(state: Arc<ProcessState>) -> Result<Process, NullexError> {
		let future = (state.future_fn)(state.clone());
		Ok(Process {
			state,
			future,
			context: UserContext::default(),
			address_space: None,
			open_files: HashMap::new(),
			next_fd: 0 // start file descriptors at 0
		})
	}

	/// Creates a new process from an ELF binary.
	pub fn from_elf(state: Arc<ProcessState>, elf_bytes: &[u8], args: &[&str], envs: &[&str]) -> Result<Process, NullexError> {
		let elf = parse_elf(elf_bytes)?;

		let mut address_space = AddressSpace::new()?;

		for seg in &elf.segments {
			load_segment(&mut address_space, elf_bytes, seg)?;
		}

		let stack_top = unsafe {
			setup_user_stack(&mut address_space, args, envs)
		};

		let mut context = UserContext::default();
		context.rip = elf.entry;
        context.rsp = stack_top;
		context.cs = user_code_selector() as u64;
		context.ss = user_data_selector() as u64;
        context.rflags = 0x202;

		let future = (state.future_fn)(state.clone());

		Ok(Process {
			state,
			future,
			context,
			address_space: Some(address_space),
			open_files: HashMap::new(),
			next_fd: 0
		})
	}

	/// Tries to get the final result and signs the task up for a callback if its still pending.
	pub fn poll(&mut self, context: &mut Context) -> core::task::Poll<i32> {
		self.future.as_mut().poll(context)	
	}
}
unsafe impl Send for Process {}

/// Structure representing all saved registers for a process.
#[derive(Debug, Default)]
#[allow(unused)]
pub struct UserContext {
	// data registers saved by the software (pushaq/push)
	rax: u64,
	rbx: u64,
	rcx: u64,
	rdx: u64,
	rsi: u64,
	rdi: u64,
	rbp: u64,

	r8: u64,
	r9: u64,
	r10: u64,
	r11: u64,
	r12: u64,
	r13: u64,
	r14: u64,
	r15: u64,

	// pushed by isr
	int_no: u64,
	err_no: u64,

	// pushed by cpu on exception.
	/// RIP register
	pub rip: u64,
	/// CS register
	pub cs: u64,
	/// RFlags register
	pub rflags: u64,
	/// RSP register
	pub rsp: u64,
	/// SS register
	pub ss: u64,
}

/// Structure representing the memory region each `Process` has.
pub struct AddressSpace {
	/// Physical frame of the memory region.
	pub page_table: PhysFrame,
	/// Regions of the memory.
	pub regions: Vec<MemoryRegion>,
}

impl AddressSpace {
	/// Creates a new `AddressSpace` from the available memory.
    pub fn new() -> Result<AddressSpace, NullexError> {
        let mut frame_binding = ALLOCATOR_INFO.frame_allocator.lock();
        let frame_allocator = frame_binding
            .as_mut()
            .ok_or(NullexError::FrameAllocatorNotInitialized)?;

        let pml4_frame = frame_allocator
            .allocate_frame()
            .ok_or(NullexError::FrameAllocationFailed)?;

        let table_ptr = unsafe { phys_to_virt(pml4_frame.start_address()) };
        unsafe { write_bytes(table_ptr.as_mut_ptr::<u8>(), 0, 4096); }

        let phys_offset = *PHYS_MEM_OFFSET.lock();
        let kernel_pml4 = unsafe { active_level_4_table(phys_offset) };
        let new_pml4 = unsafe { &mut *table_ptr.as_mut_ptr::<PageTable>() };

        for i in 216..512 {
            if kernel_pml4[i].flags().contains(PageTableFlags::PRESENT) {
                new_pml4[i] = kernel_pml4[i].clone();
            }
        }

        let mut new_mapper = unsafe { OffsetPageTable::new(new_pml4, phys_offset) };
        let old_mapper = unsafe { OffsetPageTable::new(kernel_pml4, phys_offset) };

        let mut map_page = |virt: u64| -> Result<(), NullexError> {
            let page: Page<Size4KiB> = Page::containing_address(VirtAddr::new(virt));
            if new_mapper.translate_addr(page.start_address()).is_some() {
                return Ok(());
            }
            if let Some(phys) = old_mapper.translate_addr(page.start_address()) {
                let frame = PhysFrame::containing_address(phys);
                unsafe {
                    new_mapper.map_to(
                        page,
                        frame,
                        PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
                        *frame_allocator,
                    )
                    .map_err(|_| NullexError::FrameAllocationFailed)?
                    .flush();
                }
            }
            Ok(())
        };

        unsafe extern "C" {
            static __link_phys_base: u8;
            static _end: u8;
        }
        let kernel_start = core::ptr::addr_of!(__link_phys_base) as u64;
        let kernel_end   = core::ptr::addr_of!(_end) as u64;

        serial_println!("[INFO] Kernel Addresses: {:#x} .. {:#x}", kernel_start, kernel_end);

        let mut addr = 0x1000u64;
        while addr < (kernel_end + 0xFFF) & !0xFFF {
            map_page(addr)?;
            addr += 0x1000;
        }

		map_page(0xFEE00000)?;

        let heap_start = crate::allocator::HEAP_START as u64;
        let heap_size  = crate::allocator::HEAP_SIZE as u64;
        let mut addr = heap_start & !0xFFF;
        while addr < (heap_start + heap_size + 0xFFF) & !0xFFF {
            map_page(addr)?;
            addr += 0x1000;
        }

        let mut rsp: u64;
        unsafe { asm!("mov {}, rsp", out(reg) rsp); }
        for i in 0..KERNEL_STACK_PAGES_TO_MAP {
            let addr = match rsp.checked_sub((i as u64) * 0x1000) {
                Some(v) => v,
                None => break,
            };
            map_page(addr)?;
        }

        let int_stack_top = interrupt_stack_top();
        for i in 0..(INTERRUPT_STACK_SIZE / 4096) {
            let addr = int_stack_top - 1 - (i as u64) * 0x1000;
            map_page(addr)?;
        }

        Ok(AddressSpace {
            page_table: pml4_frame,
            regions: Vec::new(),
        })
    }
}
/// A future that never completes.
pub struct ForeverPending;

impl Future for ForeverPending {
	type Output = i32;

	fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> core::task::Poll<Self::Output> {
		core::task::Poll::Pending
	}
}

/// A yield future that yields control back to the executor once before
/// completing.
pub struct YieldNow {
	yielded: bool
}

impl Future for YieldNow {
	type Output = ();

	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
		if self.yielded {
			Poll::Ready(())
		} else {
			self.yielded = true;
			cx.waker().wake_by_ref();
			Poll::Pending
		}
	}
}

/// Yields control to the scheduler.
pub async fn yield_now() {
	YieldNow {
		yielded: false
	}
	.await
}
