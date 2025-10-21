/* linker-identity.ld */
ENTRY(start)

OUTPUT_FORMAT("elf64-x86-64")
OUTPUT_ARCH(i386:x86-64)

SECTIONS {
  /* Link / load at 1 MiB (common for kernels and GRUB) */
  . = 1M;

  /* Put multiboot header first in the image */
  .multiboot_header :
  {
    KEEP(*(.multiboot_header))
  }

  .rodata : ALIGN(4K)
  {
    *(.rodata .rodata.*)
    . = ALIGN(4K);
  }

  .text : ALIGN(4K)
  {
    *(.text .text.*)
    . = ALIGN(4K);
  }

  .data : ALIGN(4K)
  {
    *(.data .data.*)
    . = ALIGN(4K);
  }

  .bss : ALIGN(4K)
  {
    __bss_start = .;
    *(.bss .bss.*)
    *(COMMON)
    . = ALIGN(4K);
    __bss_end = .;
  }

  .got :
  {
    *(.got)
    . = ALIGN(4K);
  }

  .got.plt :
  {
    *(.got.plt)
    . = ALIGN(4K);
  }

  .data.rel.ro : ALIGN(4K)
  {
    *(.data.rel.ro.local*) *(.data.rel.ro .data.rel.ro.*)
    . = ALIGN(4K);
  }

  .gcc_except_table : ALIGN(4K)
  {
    *(.gcc_except_table)
    . = ALIGN(4K);
  }

  /* exported helper symbols */
  _kernel_start = 1M;
  _kernel_end = .;
}
