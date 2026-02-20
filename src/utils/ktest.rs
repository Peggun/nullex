//!
//! ktest.rs
//! 
//! Kernel testing framework module for nullex.
//! 

use core::{slice::from_raw_parts, str::from_utf8_unchecked};

use crate::{println, serial_println};

#[cfg(feature = "test")]
include!(concat!(env!("OUT_DIR"), "/tests_registry.rs"));

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
	name_ptr: *const u8,
	name_len: usize,
	func: TestFn
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

#[macro_export]
// NOTE: macros get way more documentation for this kernel.

/// Creates a test descriptor for kernel tests.
///
/// This macro generates a `TestDescriptor` static variable that registers a test function
/// with the kernel test framework. The test is placed in the `.kernel_tests` section for
/// discovery and execution by the test harness.
///
/// # Syntax
///
/// - `create_test!(function_name)` - Registers a test function in the current module
/// - `create_test!(path::to::function)` - Registers a test function at a specific path
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
/// - The macro automatically handles name mangling using `__kernel_test_` prefix
/// - Test descriptors are marked with `#[used]` to prevent linker removal
/// - The first variant suppresses warnings for non-snake-case identifiers and non-upper-case globals
macro_rules! create_test {
	($fn_ident:ident) => {
		#[allow(non_snake_case)]
		#[allow(non_upper_case_globals)]
		mod $fn_ident {
			#[used]
			#[unsafe(link_section = ".kernel_tests")]
			#[unsafe(export_name = concat!("__kernel_test_", stringify!($fn_ident), "_", line!()))]
			pub static TEST_DESCRIPTOR: $crate::utils::ktest::TestDescriptor =
				$crate::utils::ktest::TestDescriptor {
					name_ptr: concat!(stringify!($fn_ident), "\0").as_ptr() as *const u8,
					name_len: stringify!($fn_ident).len(),
					func: super::$fn_ident
				};
		}
	};
	($fn_path:path) => {
		#[used]
		#[unsafe(link_section = ".kernel_tests")]
		#[unsafe(export_name = concat!("__kernel_test_", line!()))]
		pub static TEST_DESCRIPTOR: $crate::utils::ktest::TestDescriptor =
			$crate::utils::ktest::TestDescriptor {
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

/// Runs all tests that have been generated.
/// Can only run on `#cfg[feature = "test"]`
pub fn run_all_tests() {
	#[cfg(feature = "test")]
	{
		use crate::{
			qemu_exit,
			utils::ktest::__generated_test_registry::__kernel_test_registry_refs
		};

		// deref the wrapper newtype to get the array of pointers
		let ptrs = &__kernel_test_registry_refs.0;

		println!("Running {} tests...", ptrs.len());
		serial_println!("Running {} tests...", ptrs.len());

		let mut passed = 0;
		let mut failed = 0;

		for (i, ptr) in ptrs.iter().enumerate() {
			// deref the pointer to get the TestDescriptor
			let desc = unsafe { &**ptr };
			let name = desc.name();

			println!("test {} ({})... ", i + 1, name);
			serial_println!("test {} ({})... ", i + 1, name);

			let result = (desc.func)();
			match result {
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
			println!("test result: FAILED");
			serial_println!("test result: FAILED");
			qemu_exit(1);
		} else {
			println!("test result: ok");
			serial_println!("test result: ok");
			qemu_exit(0)
		}
	}

	#[cfg(not(feature = "test"))]
	{
		println!("Tests not compiled (feature 'test' not enabled)");
		serial_println!("Tests not compiled (feature 'test' not enabled)");
	}
}
