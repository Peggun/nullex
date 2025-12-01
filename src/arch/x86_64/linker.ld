ENTRY(start)

SECTIONS
{
    . = 0x1000;
    .multiboot_header : {
        KEEP(*(.multiboot_header))
    }

    PROVIDE(__link_phys_base = 0x00100000);
    . = __link_phys_base;

    PROVIDE(__text_addr = .);

    .rodata : ALIGN(0x1000) {
        *(.rodata .rodata.*)
        . = ALIGN(0x1000);
    }

    .text : ALIGN(0x1000) {
        *(.text .text.*)
        . = ALIGN(0x1000);
    }

    .got : ALIGN(0x1000) {
        *(.got)
        . = ALIGN(0x1000);
    }

    .got.plt : ALIGN(0x1000) {
        *(.got.plt)
        . = ALIGN(0x1000);
    }

    .data.rel.ro : ALIGN(0x1000) {
        *(.data.rel.ro.local*) *(.data.rel.ro .data.rel.ro.*)
        . = ALIGN(0x1000);
    }

    .gcc_except_table : ALIGN(0x1000) {
        *(.gcc_except_table)
        . = ALIGN(0x1000);
    }

    .data : ALIGN(0x1000) {
        *(.data .data.*)
        . = ALIGN(0x1000);
    }

    .kernel_tests : ALIGN(0x1000) {
        __start_kernel_tests = .;
        KEEP(*(.kernel_tests))
        KEEP(*(.kernel_tests.*))
        KEEP(*(.kernel_tests.*.*))
        __stop_kernel_tests = .;
        . = ALIGN(0x1000);
    }

    .bss (NOLOAD) : ALIGN(0x1000) {
        _bss = .;
        *(.bss .bss.*)
        *(COMMON)
        _ebss = .;
        . = ALIGN(0x1000);
    }

    /* Final end symbol */
    PROVIDE(_end = .);
}
