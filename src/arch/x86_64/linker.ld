ENTRY(start)
KERNEL_PHYS_BASE = 0x00100000;
KERNEL_VIRT_BASE = 0xffffffff80000000;

SECTIONS
{
    . = 0x1000;
    .multiboot_header : {
        KEEP(*(.multiboot_header))
    }

    . = KERNEL_PHYS_BASE;
    __boot_start = .;

    .boot.text ALIGN(0x1000) : {
        KEEP(*(.boot.text))
        KEEP(*(.boot.text.*))
    }
    
    .boot.rodata ALIGN(0x1000) : {
        KEEP(*(.boot.rodata))
        KEEP(*(.boot.rodata.*))
    }
    
    .boot.bss (NOLOAD) : ALIGN(0x1000) {
        KEEP(*(.boot.bss))
        KEEP(*(.boot.bss.*))
    }
    __boot_end = .;

    /*
     * Real kernel is linked into the higher half
     * VMA = high virtual address
     * LMA = physical address
     */
    . = KERNEL_VIRT_BASE + __boot_end;
    __kernel_virt_start = .;
    __kernel_phys_start = . - KERNEL_VIRT_BASE;

    /* compatability symbol */
    __link_phys_base = __kernel_phys_start;

    .text ALIGN(0x1000) : AT(ADDR(.text) - KERNEL_VIRT_BASE) {
        __text_addr = .;
        *(.text .text.*)
        __text_end = .;
    }
    
    .rodata ALIGN(0x1000) : AT(ADDR(.rodata) - KERNEL_VIRT_BASE) {
        *(.rodata .rodata.*)
    }
    
    .got ALIGN(0x1000) : AT(ADDR(.got) - KERNEL_VIRT_BASE) {
        *(.got)
    }
    
    .got.plt ALIGN(0x1000) : AT(ADDR(.got.plt) - KERNEL_VIRT_BASE) {
        *(.got.plt)
    }
    
    .data.rel.ro ALIGN(0x1000) : AT(ADDR(.data.rel.ro) - KERNEL_VIRT_BASE) {
        *(.data.rel.ro.local*)
        *(.data.rel.ro .data.rel.ro.*)
    }
    
    .gcc_except_table ALIGN(0x1000) : AT(ADDR(.gcc_except_table) - KERNEL_VIRT_BASE) {
        *(.gcc_except_table)
    }
    
    .data ALIGN(0x1000) : AT(ADDR(.data) - KERNEL_VIRT_BASE) {
        *(.data .data.*)
    }
    
    .kernel_tests ALIGN(0x1000) : AT(ADDR(.kernel_tests) - KERNEL_VIRT_BASE) {
        __start_kernel_tests = .;
        KEEP(*(.kernel_tests))
        KEEP(*(.kernel_tests.*))
        KEEP(*(.kernel_tests.*.*))
        KEEP(*(.kernel_tests.*.*.*))
        __stop_kernel_tests = .;
    }
    
    .bss ALIGN(0x1000) (NOLOAD) : AT(ADDR(.bss) - KERNEL_VIRT_BASE) {
        _bss = .;
        *(.bss .bss.*)
        *(COMMON)
        _ebss = .;
    }
    
    .ndm ALIGN(0x1000) : AT(ADDR(.ndm) - KERNEL_VIRT_BASE) {
        __ndm_start = .;
        KEEP(*(.ndm))
        __ndm_end = .;
    }
    
    _end = .;
    __kernel_virt_end = .;
    __kernel_phys_end = . - KERNEL_VIRT_BASE;
}