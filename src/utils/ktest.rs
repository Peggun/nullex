use core::{slice::from_raw_parts, str::from_utf8_unchecked};

use crate::{println, serial_println};

#[cfg(feature = "test")]
include!(concat!(env!("OUT_DIR"), "/tests_registry.rs"));

#[derive(Debug)]
pub enum TestError {
	Error
}

pub type TestFn = fn() -> Result<(), TestError>;

#[repr(C)]
pub struct TestDescriptor {
	pub name_ptr: *const u8,
	pub name_len: usize,
	pub func: TestFn
}

unsafe impl Send for TestDescriptor {}
unsafe impl Sync for TestDescriptor {}

impl TestDescriptor {
	pub fn name(&self) -> &'static str {
		unsafe {
			let bytes = from_raw_parts(self.name_ptr, self.name_len);
			from_utf8_unchecked(bytes)
		}
	}
}

#[macro_export]
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
	static __start_kernel_tests: u8;
	static __stop_kernel_tests: u8;
}

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
		serial_println!("Tests no compiled (feature 'test' not enable)");
	}
}
