# Architecture Overview

This chapter is a high-level tour of how the nullex kernel is put together.
It is intentionally not exhaustive — its goal is to give you a mental map so
you always know *where* to look. For per-function detail, see the
[Kernel API](./kernel_api.md) and the [nlibc API](./nlibc_api.md).

> nullex targets **x86_64 only**. The kernel is written in Rust; userspace
> programs are written in C against the custom `nlibc` library.

## The Big Picture

| Layer | Components / Description |
| :--- | :--- |
| **Userspace (Ring 3)** | C programs, nlibc, C API (wraps raw syscalls) |
| **Syscall Layer** | The kernel/userspace boundary |
| **Kernel (Ring 0)** | Scheduler, memory, drivers, GDT/IDT, paging, CPU setup |
| **Hardware** | x86_64 hardware |



Everything below the syscall layer is kernel-internal. Everything above it
is documented in the nlibc reference, which defines the public ABI.

## Boot Process

1. The firmware loads the bootloader, which loads the kernel ELF, sets up
   long mode and initial page tables, and hands over control.
   multiboot2 is being used as the bootloader, and all of the code for it is in `boot/`.
2. Control lands in a small assembly entry point (`_start`) which sets up a
   stack and jumps into Rust.
3. The Rust entry point performs early init, in roughly this order:
   - bring up the console / serial port for early logging
   - enable logging certain CPU features. (EFER and SSE)
   - load multiboot2 information and allocate memory and allocator.
   - load our own GDT and TSS
   - build the IDT and set up interrupt handling
   - map IOAPIC and APIC memory addresses and enable them.
   - initialize the RTC and calibrate APIC
   - setup the RamFS
   - initialize the virtio-net driver
   - discover other PCI devices
   - link ISOs and program the IOAPIC
   - finalize PCI devices and enable interrupts
   - initalise the network
   - create kernel processes (keyboard)
   - spawn the nush shell
   - create the executor and load processes in a round-robin fashion.

## Memory Management

### Physical memory
The bootloader memory map is parsed into a **page frame allocator**.
Frames are 4 KiB. All kernel allocations ultimately come from here.

### Virtual memory
Standard x86_64 4-level paging: `PML4 → PDPT → PD → PT`.
The kernel sets up its own page tables during early boot and programs
`CR3` accordingly.
The kernel is higher-half, with each process having its own PML4.

### Kernel heap
A kernel heap allocator is registered via `#[global_allocator]`, which means
normal Rust `alloc` types (`Vec`, `Box`, `String`) work in kernel space.

## Interrupts and Exceptions

- A single **IDT** is built at boot. Each vector points at a small assembly
  stub that saves CPU state and calls into a Rust handler.
- CPU exceptions (page fault, general protection fault, double fault, ...)
  print diagnostics and panic cleanly instead of silently triple-faulting.
- Hardware IRQs are currently driven mainly by the timer and keyboard.
  APIC is being used to handle these interrupts and balance between userspace and kernel space.
- Each process runs from a internal rust `loop` call. Round-robin    scheduling is used to switch between processes.

## Syscalls and Userspace

Userspace C programs never touch hardware. They call `nlibc` functions, which
issue the actual syscall instruction with a numbered ABI. The kernel's
syscall dispatcher validates the arguments and performs the work on the
program's behalf.

The authoritative list of syscalls and the C API they map to lives in the
[nlibc API reference](./nlibc_api.md).
To issue a syscall, use the `int 0x80` instruction with the syscall number in `rax`, and arguments in `rdi`, `rsi`, `rdx`, `r10`, `r8`, `r9`.

## Repository Layout

| Path | What lives here |
| --- | --- |
| `src/` | Rust kernel source |
| `programs/` | userspace C programs |
| `programs/include/` | nlibc headers — the userspace ABI |
| `docs/book/` | this guide |

## Where to look when...

- **Garbage on screen** → console / serial driver
- **Triple fault at boot** → GDT / IDT / long mode setup
- **Allocations failing** → frame allocator, then heap allocator
- **A program misbehaves** → syscall dispatcher, then the nlibc wrapper for that call
- **and more** → all files are labelled and organized accordingly so look there first.

## See also

- [Kernel API](./kernel_api.md) — rustdoc for the kernel internals
- [Userspace / nlibc API](./nlibc_api.md) — Doxygen reference for the C ABI