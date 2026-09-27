__attribute__((noreturn)) void _exit(int code) {
  __asm__ volatile(
      "mov $1, %%rax\n"
      "mov %0, %%rdi\n"
      "int $0x80\n"
      :
      : "r"((long)code)
      : "rax", "rdi");
  __builtin_unreachable();  // like unreachable!()
}

extern int main(int argc, char *argv[]);

// eventually move to a .S file
// https://www.youtube.com/watch?v=IbibjkI1kIs
__attribute__((noreturn, naked)) // same as -> !
void _start(void) {
  __asm__ volatile(
      "xor %ebp, %ebp\n"
      "mov (%rsp), %rdi\n"

      "lea 8(%rsp), %rsi\n"
      "and $-16, %rsp\n"
      "call main\n"

      "mov %rax, %rdi\n"
      "call _exit\n");
}