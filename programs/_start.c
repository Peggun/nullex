__attribute__((noreturn))
static void _exit(int code) {
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

extern int main(void);

__attribute__((noreturn)) // same as -> !
void _start(void) {
    int ret = main();
    _exit(ret);
}