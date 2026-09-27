global start
extern long_mode_start

KERNEL_VIRT_BASE equ 0xffffffff80000000

section .boot.text
bits 32

start:
    mov esp, stack_top
    mov edi, ebx

    call check_multiboot
    call check_cpuid
    call check_long_mode

    call set_up_page_tables
    call enable_paging

    lgdt [gdt64.pointer]

    jmp gdt64.code:long_mode_trampoline


check_multiboot:
    cmp eax, 0x36d76289
    jne .no_multiboot
    ret

.no_multiboot:
    mov al, "0"
    jmp error


check_cpuid:
    pushfd
    pop eax

    mov ecx, eax
    xor eax, 1 << 21

    push eax
    popfd

    pushfd
    pop eax

    push ecx
    popfd

    cmp eax, ecx
    je .no_cpuid

    ret

.no_cpuid:
    mov al, "1"
    jmp error


check_long_mode:
    mov eax, 0x80000000
    cpuid

    cmp eax, 0x80000001
    jb .no_long_mode

    mov eax, 0x80000001
    cpuid

    test edx, 1 << 29
    jz .no_long_mode

    ret

.no_long_mode:
    mov al, "2"
    jmp error


set_up_page_tables:
    ; ---------------------------------------------------------
    ; Recursive mapping at P4[510]
    ; ---------------------------------------------------------

    mov eax, p4_table
    or eax, 0b11
    mov [p4_table + 510 * 8], eax


    ; ---------------------------------------------------------
    ; Identity mapping
    ;
    ; P4[0] -> P3 low
    ; P3[0] -> P2 low
    ;
    ; Maps:
    ; 0x00000000 -> 0x00000000
    ; ---------------------------------------------------------

    mov eax, p3_low
    or eax, 0b11
    mov [p4_table + 0 * 8], eax

    mov eax, p2_low
    or eax, 0b11
    mov [p3_low + 0 * 8], eax


    ; ---------------------------------------------------------
    ; Higher-half kernel mapping
    ;
    ; 0xffffffff80000000
    ;
    ; P4 index = 511
    ; P3 index = 510
    ; ---------------------------------------------------------

    mov eax, p3_high
    or eax, 0b11
    mov [p4_table + 511 * 8], eax

    mov eax, p2_high
    or eax, 0b11
    mov [p3_high + 510 * 8], eax


    ; ---------------------------------------------------------
    ; Map first 1 GiB using 2 MiB huge pages.
    ;
    ; Both low and high aliases point to the same physical RAM.
    ; ---------------------------------------------------------

    xor ecx, ecx

.map_p2_table:
    mov eax, 0x200000
    mul ecx

    or eax, 0b10000011

    mov [p2_low  + ecx * 8], eax
    mov [p2_high + ecx * 8], eax

    inc ecx
    cmp ecx, 512
    jne .map_p2_table

    ret


enable_paging:
    mov eax, p4_table
    mov cr3, eax

    mov eax, cr4
    or eax, 1 << 5
    mov cr4, eax

    mov ecx, 0xC0000080
    rdmsr

    or eax, 1 << 8

    wrmsr

    mov eax, cr0
    or eax, 1 << 31
    mov cr0, eax

    ret


error:
    mov dword [0xb8000], 0x4f524f45
    mov dword [0xb8004], 0x4f3a4f52
    mov dword [0xb8008], 0x4f204f20
    mov byte [0xb800a], al

    cli

.hang:
    hlt
    jmp .hang


; -------------------------------------------------------------
; We enter here after enabling long mode.
;
; Still executing through the low identity mapping.
; -------------------------------------------------------------

bits 64

long_mode_trampoline:
    mov rax, long_mode_start
    jmp rax


section .boot.bss
align 4096

p4_table:
    resb 4096

p3_low:
    resb 4096

p2_low:
    resb 4096

p3_high:
    resb 4096

p2_high:
    resb 4096

stack_bottom:
    resb 4096 * 16

stack_top:


section .boot.rodata

gdt64:
    dq 0

.code: equ $ - gdt64

    dq (1 << 43) | \
       (1 << 44) | \
       (1 << 47) | \
       (1 << 53)

.pointer:
    dw $ - gdt64 - 1
    dq gdt64