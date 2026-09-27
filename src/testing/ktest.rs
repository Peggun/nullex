//!
//! ktest.rs
//!
//! Kernel testing framework module for nullex.

use alloc::vec::Vec;
use core::{slice::from_raw_parts, str::from_utf8_unchecked};

use x86_64::{instructions::interrupts, registers::control::Cr3};

use crate::{
	ProcessId,
	arch::x86_64::user::{USER_STACK_TOP, asm_copy_to_user, setup_user_stack},
	gdt::interrupt_stack_top,
	println,
	serial_println
};

#[derive(Debug)]
// todo! expand error types.
/// Generic enum representing all error types regarding tests.
pub enum TestError {
	/// A generic error.
	Error
}

type TestFn = fn() -> Result<(), TestError>;

#[repr(C)]
/// Structure representing all data needed for locating
/// and running tests.
pub struct TestDescriptor {
	/// The pointer to the name of the test
	pub name_ptr: *const u8,
	/// The length of the name of the test
	pub name_len: usize,
	/// The actual test code.
	pub func: TestFn
}

unsafe impl Send for TestDescriptor {}
unsafe impl Sync for TestDescriptor {}

impl TestDescriptor {
	/// Returns the test name from a `TestDescriptor`
	pub fn name(&self) -> &'static str {
		unsafe {
			let bytes = from_raw_parts(self.name_ptr, self.name_len);
			from_utf8_unchecked(bytes)
		}
	}
}

/// Runs a closure with a real userspace virtual address containing `data`.
///
/// The test remains in ring 0. A temporary `AddressSpace` is created, its user
/// stack is mapped, the address space is made active with CR3, and the supplied
/// bytes are copied into userspace using the same `asm_copy_to_user()` path
/// that the kernel uses for real userspace.
/// The original CR3 is restored before returning.
///
/// # Safety
///
/// This helper switches CR3 and executes kernel code against a separate page
/// table. The temporary address space must contain all kernel mappings required
/// by the closure.
//#[cfg(target_feature = "test")]
pub unsafe fn with_test_user_memory<T, F>(data: &[u8], f: F) -> Result<T, TestError>
where
	F: FnOnce(*const u8, usize) -> Result<T, TestError>
{
	use crate::task::address_space::AddressSpace;

	if data.is_empty() {
		return Err(TestError::Error);
	}

	let mut address_space = AddressSpace::new().map_err(|e| {
		serial_println!(
			"[TEST USER MEMORY] Failed to create user memory address space: {:?}",
			e
		);
		TestError::Error
	})?;

	unsafe { setup_user_stack(&mut address_space, Vec::new(), Vec::new()) }.map_err(|e| {
		serial_println!("[TEST USER MEMORY] Failed to create user stack: {:?}", e);
		TestError::Error
	})?;

	let user_addr = USER_STACK_TOP - 0x2000;
	let previous_cr3 = Cr3::read();

	let result = interrupts::without_interrupts(|| {
		unsafe {
			Cr3::write(address_space.page_table, previous_cr3.1);
		}

		serial_println!(
			"[TEST USER MEMORY] switched cr3: {:#x}",
			address_space.page_table.start_address().as_u64()
		);
		let written = unsafe { asm_copy_to_user(user_addr as *mut u8, data.as_ptr(), data.len()) };

		if written != data.len() {
			serial_println!(
				"[TEST USER MEMORY] Failed to copy data to user memory: expected {}, got {}",
				data.len(),
				written
			);
			return Err(TestError::Error);
		}

		serial_println!(
			"[TEST USER MEMORY] Successfully copied data to user memory: {} bytes",
			written
		);

		f(user_addr as *const u8, data.len())
	});

	unsafe {
		Cr3::write(previous_cr3.0, previous_cr3.1);
	}

	serial_println!(
		"[TEST USER MEMORY] switched cr3 back to: {:#x}",
		previous_cr3.0.start_address().as_u64()
	);

	result
}

#[macro_export]
// NOTE: macros get way more documentation for this kernel.

/// Creates a test descriptor for kernel tests.
///
/// This macro generates a `TestDescriptor` static variable that registers a
/// test function with the kernel test framework. The test is placed in the
/// `.kernel_tests` section for discovery and execution by the test harness.
///
/// # Syntax
///
/// - `create_test!(function_name)` - Registers a test function in the current
///   module
/// - `create_test!(path::to::function)` - Registers a test function at a
///   specific path
///
/// # Examples
///
/// Register a test function in the current module:
///
/// ```ignore
/// fn my_test() -> Result<(), TestError> {
///     assert_eq!(2 + 2, 4);
/// }
/// create_test!(my_test);
/// ```
///
/// Register a test function from another module:
///
/// ```ignore
/// create_test!(crate::tests::validate_kernel_state);
/// ```
///
/// # Notes
///
/// - The macro automatically handles name mangling using `__kernel_test_`
///   prefix
/// - Test descriptors are marked with `#[used]` to prevent linker removal
/// - The first variant suppresses warnings for non-snake-case identifiers and
///   non-upper-case globals
macro_rules! create_test {
	($fn_ident:ident) => {
		#[allow(non_snake_case)]
		#[allow(non_upper_case_globals)]
		mod $fn_ident {
			#[used]
			#[unsafe(link_section = ".kernel_tests")]
			pub static TEST_DESCRIPTOR: $crate::testing::ktest::TestDescriptor =
				$crate::testing::ktest::TestDescriptor {
					name_ptr: concat!(stringify!($fn_ident), "\0").as_ptr() as *const u8,
					name_len: stringify!($fn_ident).len(),
					func: super::$fn_ident
				};
		}
	};
	($fn_path:path) => {
		#[used]
		#[unsafe(link_section = ".kernel_tests")]
		pub static TEST_DESCRIPTOR: $crate::testing::ktest::TestDescriptor =
			$crate::testing::ktest::TestDescriptor {
				name_ptr: concat!(stringify!($fn_path), "\0").as_ptr() as *const u8,
				name_len: stringify!($fn_path).len(),
				func: $fn_path
			};
	};
}

unsafe extern "C" {
	/// The starting address where the kernel tests are stored.
	unsafe static __start_kernel_tests: u8;
	/// The ending address where the kernel tests are stored.
	unsafe static __stop_kernel_tests: u8;
}

/// Test assertion macro.
///
/// Behaves like `assert!`, but returns `Err(TestError::Error)` instead of
/// panicking.
///
/// This allows the test runner to continue even when an assertion fails.
///
/// # Examples
///
/// ```
/// tassert!(true);
/// ```
#[macro_export]
macro_rules! tassert {
    ($condition:expr) => {
        if !$condition {
            return Err($crate::testing::ktest::TestError::Error);
        }
    };
    ($condition:expr, $($arg:tt)+) => {
        if !$condition {
            $crate::serial_println!("[TEST ASSERTION FAILED] {}", format_args!($($arg)+));
            return Err($crate::testing::ktest::TestError::Error);
        }
    };
}

/// Test equality assertion.
///
/// Returns `Err(TestError::Error)` instead of panicking when the two
/// expressions are not equal, allowing the test runner to continue.
#[macro_export]
macro_rules! tassert_eq {
	($left:expr, $right:expr) => {{
		let left = &$left;
		let right = &$right;

		if left != right {
			$crate::serial_println!(
				"[TEST ASSERTION FAILED] left = {:?}, right = {:?}",
				left,
				right
			);

			return Err($crate::testing::ktest::TestError::Error);
		}
	}};

	($left:expr, $right:expr, $($arg:tt)+) => {{
		let left = &$left;
		let right = &$right;

		if left != right {
			$crate::serial_println!(
				"[TEST ASSERTION FAILED] left = {:?}, right = {:?}: {}",
				left,
				right,
				format_args!($($arg)+)
			);

			return Err($crate::testing::ktest::TestError::Error);
		}
	}};
}

/// Test inequality assertion.
///
/// Returns `Err(TestError::Error)` instead of panicking when the two
/// expressions are equal, allowing the test runner to continue.
#[macro_export]
macro_rules! tassert_ne {
	($left:expr, $right:expr) => {{
		let left = &$left;
		let right = &$right;

		if left == right {
			$crate::serial_println!(
				"[TEST ASSERTION FAILED] left = {:?}, right = {:?}",
				left,
				right
			);

			return Err($crate::testing::ktest::TestError::Error);
		}
	}};

	($left:expr, $right:expr, $($arg:tt)+) => {{
		let left = &$left;
		let right = &$right;

		if left == right {
			$crate::serial_println!(
				"[TEST ASSERTION FAILED] values are equal: {:?} == {:?}: {}",
				left,
				right,
				format_args!($($arg)+)
			);

			return Err($crate::testing::ktest::TestError::Error);
		}
	}};
}

/// Runs all tests that have been generated.
/// Can only run on `#cfg[feature = "test"]`
pub fn run_all_tests() {
	#[cfg(feature = "test")]
	{
		use crate::{qemu_exit, serial_println};

		let tests: &[TestDescriptor] = unsafe {
			let start = &__start_kernel_tests as *const u8 as *const TestDescriptor;
			let end = &__stop_kernel_tests as *const u8 as *const TestDescriptor;

			// the section is page-aligned and may have padding at the end,
			// so use offset_from to get the exact count
			let count = end.offset_from(start) as usize;
			core::slice::from_raw_parts(start, count)
		};

		println!("Running {} tests...", tests.len());
		serial_println!("Running {} tests...", tests.len());

		let mut passed = 0;
		let mut failed = 0;

		unsafe {
			let start = &__start_kernel_tests as *const u8 as usize;
			let end = &__stop_kernel_tests as *const u8 as usize;
			let size = end - start;
			serial_println!(
				"kernel_tests section: start={:#x} end={:#x} size={} sizeof(TestDescriptor)={} count={}",
				start,
				end,
				size,
				core::mem::size_of::<TestDescriptor>(),
				size / core::mem::size_of::<TestDescriptor>()
			);
		}

		for (i, desc) in tests.iter().enumerate() {
			if desc.name_ptr.is_null() || desc.func as usize == 0 {
				break;
			}
			serial_println!(
				"test {} ptr={:p} len={}",
				i + 1,
				desc.name_ptr,
				desc.name_len
			);
			let name = desc.name();
			serial_println!("test {} ({})... ", i + 1, name);
			println!("test {} ({})... ", i + 1, name);

			match (desc.func)() {
				Ok(_) => {
					println!("ok");
					serial_println!("ok");
					passed += 1;
				}
				Err(e) => {
					println!("FAILED: {:?}", e);
					serial_println!("FAILED: {:?}", e);
					failed += 1;
				}
			}
		}

		println!("\n{} passed, {} failed", passed, failed);
		serial_println!("\n{} passed, {} failed", passed, failed);

		if failed > 0 {
			println!("kernel test result: FAILED");
			serial_println!("kernel test result: FAILED");
			//qemu_exit(1);
		} else {
			println!("kernel test result: ok");
			serial_println!("kernel test result: ok");
			//qemu_exit(0);
		}
	}

	#[cfg(not(feature = "test"))]
	{
		println!("Tests not compiled (feature 'test' not enabled)");
		serial_println!("Tests not compiled (feature 'test' not enabled)");
	}
}

#[cfg(feature = "test")]
pub fn run_user_tests() -> ProcessId {
	use crate::{EXECUTOR, ProcessId, fs, process::util::spawn_user_process};

	println!("Running user tests...");
	let ktest_pid = fs::with_fs(|fs| match fs.read_file("/init/ktest.elf") {
		Ok(bytes) => match spawn_user_process(bytes, Vec::new(), Vec::new()) {
			Ok(proc) => {
				let pid = proc.state.id;
				{
					let mut executor = EXECUTOR.lock();
					if let Err(e) = executor.spawn_process(proc) {
						serial_println!("[ERROR] Failed to queue ktest process: {}", e);
						return ProcessId::new(0);
					}
				}
				pid
			}
			Err(e) => {
				serial_println!("[ERROR] Failed to spawn ktest process: {}", e);
				ProcessId::new(0)
			}
		},
		Err(_) => {
			serial_println!("[ERROR] ktest is missing!");
			ProcessId::new(0)
		}
	});

	ktest_pid
}
