global long_mode_start
extern kernel_main

section .bss
align 16
higher_half_stack_bottom:
    resb 4096 * 16 ; 16 kib higher half stack
higher_half_stack_top:

section .text
bits 64

long_mode_start:
    ; We are now executing in the higher half.
    ;
    ; RIP should be somewhere around:
    ; 0xffffffff801xxxxx

    ; switch rsp to the higher kernel stack
    mov rsp, higher_half_stack_top
    
    xor eax, eax

    mov ss, ax
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

    ; NULL frame pointer terminates kernel stack walking.
    xor rbp, rbp

    ; Multiboot pointer is still in RDI.
    ;
    ; EDI was set in boot.asm:
    ;     mov edi, ebx
    ;
    ; The low identity mapping still exists, so the physical
    ; Multiboot structure remains accessible.
    call kernel_main

.hang:
    cli
    hlt
    jmp .hang