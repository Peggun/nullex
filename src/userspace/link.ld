ENTRY(_start)
SECTIONS {
    . = 0x100000; /* Arbitrary user-mode address */
    .text : { *(.text) }
    .rodata : { *(.rodata) }
    .data : { *(.data) }
    .bss : { *(.bss) }
}