#if defined(__x86_64__) || defined(_M_X64)
__attribute__((noreturn))
void _exit(int code) {
    __asm__ volatile (
        "mov $1, %%rax\n"
        "mov %0, %%rdi\n"
        "int $0x80\n"
        :
        : "r"((long)code)
        : "rax", "rdi"
    );
    __builtin_unreachable(); // like unreachable!()
}

extern int main(int argc, char *argv[]);

// eventually move to a .S file
// also need to test if argc, and argv work properly.
// https://www.youtube.com/watch?v=IbibjkI1kIs
__attribute__((noreturn, naked)) // same as -> !
void _start(void) {
    __asm__ __volatile__(
        "xor %ebp, %ebp\n"
        "mov (%rsp), %rdi\n"

        "lea 8(%rsp), %rsi\n"
        "and $-16, %rsp\n"
        "call main\n"

        "mov %rax, %rdi\n"
        "call _exit\n"
    );
    __builtin_unreachable();
}
#endif

#if defined(__aarch64__) || defined(_M_ARM64)
__attribute__((noreturn, naked))
void _start(void) {
    __asm__ __volatile__(
        "mov x0, #0\n\t"
        "mov w8, #93\n\t"
        "svc #0\n\t"
    );
    __builtin_unreachable();
}
#endif