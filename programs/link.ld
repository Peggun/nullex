OUTPUT_FORMAT("elf64-x86-64")
OUTPUT_ARCH(i386:x86-64)

ENTRY(_start)

PHDRS {
    text PT_LOAD FLAGS(5); /* R_X */
    data PT_LOAD FLAGS(6); /* R_W */
}

SECTIONS {
    . = 0x0000000000400000;

    .text : ALIGN(0x1000) {
        *(.text._start)
        *(.text .text.*)
    } :text

    .rodata : ALIGN(8) 
    {
        *(.rodata)
        *(.rodata.*)
    } :text

    .user_tests : ALIGN(8) {
        PROVIDE(__start_user_tests = .);
        KEEP(*(.user_tests .user_tests.*))
        PROVIDE(__stop_user_tests = .);
    } :text

    .data : ALIGN(0x1000)
    {
        *(.data .data.*)
        *(.got .got.*)
    } :data

    . = ALIGN(0x1000);
    
    .bss : ALIGN(16)
    {
        PROVIDE(__bss_start = .);
        *(COMMON)
        *(.bss .bss.*)
        PROVIDE(__bss_end = .);
    } :data

    . = ALIGN(0x1000);
    PROVIDE(_end = .);

    /DISCARD/ :
    {
        *(.comment)
        *(.note.*)
        *(.eh_frame*)
    }
}
