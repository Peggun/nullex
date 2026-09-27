/// Structure representing all saved registers for a process.
#[repr(C)]
#[derive(Debug, Default)]
#[allow(unused)]
pub struct UserContext {
	// data registers saved by the software (pushaq/push)
	pub rax: u64,
	pub rbx: u64,
	pub rcx: u64,
	pub rdx: u64,
	pub rsi: u64,
	pub rdi: u64,
	pub rbp: u64,

	pub r8: u64,
	pub r9: u64,
	pub r10: u64,
	pub r11: u64,
	pub r12: u64,
	pub r13: u64,
	pub r14: u64,
	pub r15: u64,

	// pushed by cpu on interrupt entry.
	/// RIP register
	pub rip: u64,
	/// CS register
	pub cs: u64,
	/// RFlags register
	pub rflags: u64,
	/// RSP register
	pub rsp: u64,
	/// SS register
	pub ss: u64
}

#[repr(C)]
pub struct UserRegisters;
