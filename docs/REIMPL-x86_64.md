# Nullex Kernel Primitives: A Ground-Up Re-Implementation (x86_64)



This is a ground-up re-implementation of low-level architectural components, inspired by the functionality of the `x86_64` crate. All code is being developed from first principles using architectural manuals, books, and research, without directly porting or copying existing source code.
However, each of the names of structs, traits, functions etc will be called the same, just so its easier to implement back into my code. Thanks so much to the `x86_64` crate and their contributors.

## Implementation Roadmap

The following checklist tracks the development of the types that are currently used by the kernel, but will get phased out.

---

### Memory Management & Paging

* [ ] **Virtual & Physical Addressing**
    * [X] `VirtAddr` (struct): A wrapper for a `u64` virtual address.
    * [ ] `PhysAddr` (struct): A wrapper for a `u64` physical address.
* [ ] **Paging Core Structures**
    * [ ] `Page<S>` (struct): Represents a virtual memory page of a given size `S`.
    * [ ] `PhysFrame` (struct): Represents a physical memory frame.
    * [ ] `PageTable` (struct): Represents a page table with its entries.
    * [ ] `PageTableFlags` (struct): Manages flags for page table entries (e.g., Present, Writable).
* [ ] **Paging Abstractions**
    * [ ] `FrameAllocator<S>` (trait): A generic trait for types that can allocate a physical frame.
    * [ ] `Mapper<S>` (trait): A trait for common page table operations.
    * [ ] `OffsetPageTable` (struct): A `Mapper` implementation that works with a physical memory offset.
    * [ ] `Translate` (trait): Provides methods for translating virtual to physical addresses.
* [ ] **Error Handling**
    * [ ] Define page size types (e.g., 4KiB, 2MiB).
    * [ ] Implement robust error handling for paging operations.

---

### Interrupts & Exceptions

* [ ] `InterruptDescriptorTable` (struct): Manages the 256-entry IDT.
* [ ] `InterruptStackFrame` (struct): A wrapper for the interrupt stack frame pushed by the CPU.
* [ ] `interrupts::enable()` (func): A wrapper for the `sti` instruction.
* [ ] `interrupts::disable()` (func): A wrapper for the `cli` instruction.
* [ ] `interrupts::without_interrupts(||)` (func): Executes a closure with interrupts disabled.
* [ ] Page Fault error code handling (AMD Vol 2: 8.4.2, Intel Vol 3A: 4.7).

---

### System Structures & Segmentation

* [ ] **Global Descriptor Table (GDT)**
    * [ ] `Descriptor` (struct): A 64-bit segment descriptor (for user or system segments).
    * [ ] `GlobalDescriptorTable` (struct): Represents the GDT itself.
    * [ ] `SegmentSelector` (struct): A `u16` wrapper for a segment selector.
* [ ] **Task State Segment (TSS)**
    * [ ] `TaskStateSegment` (struct): Defines kernel-level stacks for handling interrupts.

---

### CPU Registers & Instructions

* [ ] `Msr` (struct): Represents a Model-Specific Register.
* [ ] `Port<T>` (struct): For reading from and writing to I/O ports.
* [ ] `registers::control::Cr2` (struct): Contains the Page Fault Linear Address (PFLA).
* [ ] `registers::control::Cr3` (struct): Contains the physical address of the top-level page table.
* [ ] `instructions::<>` wrappers for common assembly instructions (e.g., `hlt`, `nop`).