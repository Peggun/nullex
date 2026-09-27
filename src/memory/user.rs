use alloc::string::String;

use crate::{
	arch::x86_64::user::{asm_copy_from_user, asm_copy_to_user},
	error::NullexError,
	serial_println
};

pub type UserResult<T> = Result<T, NullexError>;

pub const USER_SPACE_LIMIT: usize = 0x0000_8000_0000_0000;

fn validate_user_range(start: usize, size: usize) -> UserResult<usize> {
	let end = start.checked_add(size).ok_or(NullexError::InvalidPointer)?;
	if start >= USER_SPACE_LIMIT || end > USER_SPACE_LIMIT {
		return Err(NullexError::InvalidPointer);
	}
	Ok(end)
}

fn validate_user_ptr<T>(ptr: *const T, size: usize) -> UserResult<()> {
	if ptr.is_null() {
		return Err(NullexError::InvalidPointer);
	}

	let start = ptr as usize;

	if start % core::mem::align_of::<T>() != 0 {
		return Err(NullexError::InvalidPointer);
	}

	validate_user_range(start, size)?;

	Ok(())
}

fn validate_user_mut_ptr<T>(ptr: *mut T, size: usize) -> UserResult<()> {
	validate_user_ptr(ptr as *const T, size)
}

pub struct UserPtr<T> {
	pub ptr: *const T
}

pub struct UserMutPtr<T> {
	pub ptr: *mut T
}

pub struct UserSlice<T> {
	pub ptr: *const T,
	pub len: usize
}

pub struct UserMutSlice<T> {
	pub ptr: *mut T,
	pub len: usize
}

pub struct UserString {
	pub ptr: *const u8,
	pub len: usize
}

impl<T: Copy> UserPtr<T> {
	pub const fn new(ptr: *const T) -> Self {
		Self {
			ptr
		}
	}

	pub fn read(&self) -> UserResult<T> {
		let size = core::mem::size_of::<T>();

		validate_user_ptr(self.ptr, size)?;

		let mut value = core::mem::MaybeUninit::<T>::uninit();

		unsafe {
			let src = self.ptr as *const u8;
			let dst = value.as_mut_ptr() as *mut u8;

			let bytes_copied = asm_copy_from_user(dst, src, size);

			if bytes_copied != size {
				return Err(NullexError::PageFault);
			}

			Ok(value.assume_init())
		}
	}
}

impl<T: Copy> UserMutPtr<T> {
	pub const fn new(ptr: *mut T) -> Self {
		Self {
			ptr
		}
	}

	pub fn write(&self, value: &T) -> UserResult<()> {
		let size = core::mem::size_of::<T>();
		validate_user_mut_ptr(self.ptr, size)?;

		unsafe {
			let src = value as *const T as *const u8;
			let dst = self.ptr as *mut u8;
			if asm_copy_to_user(dst, src, size) != size {
				return Err(NullexError::PageFault);
			}
		}

		Ok(())
	}
}

impl<T: Copy> UserSlice<T> {
	pub const fn new(ptr: *const T, len: usize) -> Self {
		Self {
			ptr,
			len
		}
	}

	pub fn copy_to_kernel(&self, dst: &mut [T]) -> UserResult<()> {
		if dst.len() < self.len {
			return Err(NullexError::BufferTooSmall);
		}

		let size = self
			.len
			.checked_mul(core::mem::size_of::<T>())
			.ok_or(NullexError::InvalidPointer)?;

		validate_user_ptr(self.ptr, size)?;

		unsafe {
			let src = self.ptr as *const u8;
			let dst_ptr = dst.as_mut_ptr() as *mut u8;

			if asm_copy_from_user(dst_ptr, src, size) != size {
				return Err(NullexError::PageFault);
			}
		}

		Ok(())
	}
}

impl<T: Copy> UserMutSlice<T> {
	pub const fn new(ptr: *mut T, len: usize) -> Self {
		Self {
			ptr,
			len
		}
	}

	pub fn copy_from_kernel(&self, src: &[T]) -> UserResult<usize> {
		let elements_to_copy = core::cmp::min(self.len, src.len());

		if elements_to_copy == 0 {
			return Ok(0);
		}

		let byte_size = elements_to_copy
			.checked_mul(core::mem::size_of::<T>())
			.ok_or(NullexError::InvalidPointer)?;

		validate_user_mut_ptr(self.ptr, byte_size)?;

		unsafe {
			let src_ptr = src.as_ptr() as *const u8;
			let dst_ptr = self.ptr as *mut u8;

			let bytes_written = asm_copy_to_user(dst_ptr, src_ptr, byte_size);

			let elements_written = bytes_written / core::mem::size_of::<T>();

			if bytes_written != byte_size {
				return Ok(elements_written);
			}
		}

		Ok(elements_to_copy)
	}
}

impl UserString {
	pub fn read_to_kernel_string(&self, max_len: usize) -> UserResult<String> {
		let limit = core::cmp::min(self.len, max_len);
		let start = self.ptr as usize;

		validate_user_range(start, limit)?;

		let mut buffer = vec![0u8; limit];

		unsafe {
			let bytes_copied = asm_copy_from_user(buffer.as_mut_ptr(), self.ptr, limit);
			buffer.truncate(bytes_copied);
		}

		if let Some(pos) = buffer.iter().position(|&b| b == 0) {
			buffer.truncate(pos);
		}

		let str = String::from_utf8(buffer);
		serial_println!("{:?}", str);
		str.map_err(|_| NullexError::InvalidEncoding)
	}
}

#[cfg(feature = "test")]
pub mod tests {
	use crate::{
		arch::x86_64::user::USER_STACK_TOP,
		error::NullexError,
		memory::user::{
			USER_SPACE_LIMIT,
			UserMutPtr,
			UserMutSlice,
			UserPtr,
			UserSlice,
			UserString,
			asm_copy_to_user,
			validate_user_range
		},
		serial_println,
		tassert,
		tassert_eq,
		testing::ktest::{TestError, with_test_user_memory}
	};

	pub fn test_user_space_limit_value() -> Result<(), TestError> {
		tassert_eq!(USER_SPACE_LIMIT, 0x0000_8000_0000_0000);
		Ok(())
	}
	crate::create_test!(test_user_space_limit_value);

	pub fn test_validate_user_range_valid() -> Result<(), TestError> {
		tassert_eq!(validate_user_range(0, 100).unwrap(), 100);
		tassert_eq!(
			validate_user_range(USER_SPACE_LIMIT - 100, 100).unwrap(),
			USER_SPACE_LIMIT
		);
		Ok(())
	}
	crate::create_test!(test_validate_user_range_valid);

	pub fn test_validate_user_range_overflow() -> Result<(), TestError> {
		let res = validate_user_range(usize::MAX, 100);
		tassert!(matches!(res, Err(NullexError::InvalidPointer)));
		Ok(())
	}
	crate::create_test!(test_validate_user_range_overflow);

	pub fn test_validate_user_range_start_exceeds_limit() -> Result<(), TestError> {
		let res = validate_user_range(USER_SPACE_LIMIT, 100);
		tassert!(matches!(res, Err(NullexError::InvalidPointer)));
		Ok(())
	}
	crate::create_test!(test_validate_user_range_start_exceeds_limit);

	pub fn test_validate_user_range_end_exceeds_limit() -> Result<(), TestError> {
		let res = validate_user_range(USER_SPACE_LIMIT - 50, 100);
		tassert!(matches!(res, Err(NullexError::InvalidPointer)));
		Ok(())
	}
	crate::create_test!(test_validate_user_range_end_exceeds_limit);

	pub fn test_user_ptr_new() -> Result<(), TestError> {
		let ptr = UserPtr::new(0x1000 as *const u32);
		tassert_eq!(ptr.ptr, 0x1000 as *const u32);
		Ok(())
	}
	crate::create_test!(test_user_ptr_new);

	pub fn test_user_ptr_read_misaligned() -> Result<(), TestError> {
		let ptr = UserPtr::new(0x1001 as *const u32);
		let res = ptr.read();
		tassert!(matches!(res, Err(NullexError::InvalidPointer)));
		Ok(())
	}
	crate::create_test!(test_user_ptr_read_misaligned);

	pub fn test_user_ptr_read_out_of_bounds() -> Result<(), TestError> {
		let ptr = UserPtr::new((USER_SPACE_LIMIT - 2) as *const u32);
		let res = ptr.read();
		tassert!(matches!(res, Err(NullexError::InvalidPointer)));
		Ok(())
	}
	crate::create_test!(test_user_ptr_read_out_of_bounds);

	pub fn test_user_mut_ptr_new() -> Result<(), TestError> {
		let ptr = UserMutPtr::new(0x2000 as *mut u32);
		tassert_eq!(ptr.ptr, 0x2000 as *mut u32);
		Ok(())
	}
	crate::create_test!(test_user_mut_ptr_new);

	pub fn test_user_mut_ptr_write_misaligned() -> Result<(), TestError> {
		let ptr = UserMutPtr::new(0x2001 as *mut u32);
		let val = 42u32;
		let res = ptr.write(&val);
		tassert!(matches!(res, Err(NullexError::InvalidPointer)));
		Ok(())
	}
	crate::create_test!(test_user_mut_ptr_write_misaligned);

	pub fn test_user_mut_ptr_write_out_of_bounds() -> Result<(), TestError> {
		let ptr = UserMutPtr::new((USER_SPACE_LIMIT - 2) as *mut u32);
		let val = 42u32;
		let res = ptr.write(&val);
		tassert!(matches!(res, Err(NullexError::InvalidPointer)));
		Ok(())
	}
	crate::create_test!(test_user_mut_ptr_write_out_of_bounds);

	pub fn test_user_slice_copy_to_kernel_buffer_too_small() -> Result<(), TestError> {
		let slice = UserSlice {
			ptr: 0x3000 as *const u32,
			len: 10
		};
		let mut dst = [0u32; 5];
		let res = slice.copy_to_kernel(&mut dst);
		tassert!(matches!(res, Err(NullexError::BufferTooSmall)));
		Ok(())
	}
	crate::create_test!(test_user_slice_copy_to_kernel_buffer_too_small);

	pub fn test_user_slice_copy_to_kernel_overflow() -> Result<(), TestError> {
		let slice = UserSlice {
			ptr: 0x3000 as *const u64,
			len: usize::MAX
		};
		let mut dst = [0u64; 10];
		let res = slice.copy_to_kernel(&mut dst);
		tassert!(matches!(res, Err(NullexError::BufferTooSmall)));
		Ok(())
	}
	crate::create_test!(test_user_slice_copy_to_kernel_overflow);

	pub fn test_user_slice_copy_to_kernel_out_of_bounds() -> Result<(), TestError> {
		let slice = UserSlice {
			ptr: (USER_SPACE_LIMIT - 10) as *const u32,
			len: 10
		};
		let mut dst = [0u32; 10];
		let res = slice.copy_to_kernel(&mut dst);
		tassert!(matches!(res, Err(NullexError::InvalidPointer)));
		Ok(())
	}
	crate::create_test!(test_user_slice_copy_to_kernel_out_of_bounds);

	pub fn test_user_mut_slice_new() -> Result<(), TestError> {
		let slice = UserMutSlice::new(0x4000 as *mut u32, 5);
		tassert_eq!(slice.ptr, 0x4000 as *mut u32);
		tassert_eq!(slice.len, 5);
		Ok(())
	}
	crate::create_test!(test_user_mut_slice_new);

	pub fn test_user_mut_slice_copy_from_kernel_zero_length() -> Result<(), TestError> {
		let slice = UserMutSlice::new(0x4000 as *mut u32, 0);
		let src = [1u32, 2, 3];
		let res = slice.copy_from_kernel(&src);
		tassert_eq!(res.unwrap(), 0);
		Ok(())
	}
	crate::create_test!(test_user_mut_slice_copy_from_kernel_zero_length);

	pub fn test_user_mut_slice_huge_len_is_bounded() -> Result<(), TestError> {
		let slice = UserMutSlice::new(0x4000 as *mut u64, usize::MAX);
		let src = [1u64, 2, 3];

		let res = slice.copy_from_kernel(&src);

		tassert_eq!(res, Ok(3));
		Ok(())
	}
	crate::create_test!(test_user_mut_slice_huge_len_is_bounded);

	pub fn test_user_mut_slice_copy_from_kernel_misaligned() -> Result<(), TestError> {
		let slice = UserMutSlice::new(0x4001 as *mut u32, 5);
		let src = [1u32, 2, 3, 4, 5];
		let res = slice.copy_from_kernel(&src);
		tassert!(matches!(res, Err(NullexError::InvalidPointer)));
		Ok(())
	}
	crate::create_test!(test_user_mut_slice_copy_from_kernel_misaligned);

	pub fn test_user_mut_slice_copy_from_kernel_out_of_bounds() -> Result<(), TestError> {
		let slice = UserMutSlice::new((USER_SPACE_LIMIT - 10) as *mut u32, 10);
		let src = [1u32, 2, 3, 4, 5, 6, 7, 8, 9, 10];
		let res = slice.copy_from_kernel(&src);
		tassert!(matches!(res, Err(NullexError::InvalidPointer)));
		Ok(())
	}
	crate::create_test!(test_user_mut_slice_copy_from_kernel_out_of_bounds);

	pub fn test_user_string_read_to_kernel_string_valid() -> Result<(), TestError> {
		unsafe {
			with_test_user_memory(b"hello", |user_addr, len| {
				let user_str = UserString {
					ptr: user_addr as *const u8,
					len
				};

				let res = user_str
					.read_to_kernel_string(10)
					.map_err(|_| TestError::Error)?;

				tassert_eq!(res, "hello");

				Ok(())
			})
		}
	}
	crate::create_test!(test_user_string_read_to_kernel_string_valid);

	pub fn test_user_string_read_to_kernel_string_null_terminated() -> Result<(), TestError> {
		unsafe {
			with_test_user_memory(b"hello\0world", |user_addr, len| {
				let user_str = UserString {
					ptr: user_addr as *const u8,
					len
				};

				let res = user_str.read_to_kernel_string(20).map_err(|e| {
					serial_println!("Function returned error: {:?}", e);
					TestError::Error
				})?;

				tassert_eq!(res, "hello");

				Ok(())
			})
		}
	}
	crate::create_test!(test_user_string_read_to_kernel_string_null_terminated);

	pub fn test_user_string_read_to_kernel_string_invalid_utf8() -> Result<(), TestError> {
		unsafe {
			with_test_user_memory(b"hello\xffworld", |user_addr, len| {
				let user_str = UserString {
					ptr: user_addr as *const u8,
					len
				};

				let res = user_str.read_to_kernel_string(20);

				tassert!(
					matches!(res, Err(NullexError::InvalidEncoding)),
					"expected InvalidEncoding, got {:?}",
					res
				);

				Ok(())
			})
		}
	}
	crate::create_test!(test_user_string_read_to_kernel_string_invalid_utf8);

	pub fn test_user_string_read_to_kernel_string_out_of_bounds() -> Result<(), TestError> {
		let user_str = UserString {
			ptr: (USER_SPACE_LIMIT - 5) as *const u8,
			len: 10
		};
		let res = user_str.read_to_kernel_string(20);
		tassert!(matches!(res, Err(NullexError::InvalidPointer)));
		Ok(())
	}
	crate::create_test!(test_user_string_read_to_kernel_string_out_of_bounds);

	pub fn test_user_string_read_to_kernel_string_truncates_to_max_len() -> Result<(), TestError> {
		unsafe {
			with_test_user_memory(b"hello world", |user_addr, len| {
				let user_str = UserString {
					ptr: user_addr as *const u8,
					len
				};
				let res = user_str
					.read_to_kernel_string(5)
					.map_err(|e| TestError::Error)?;
				if res != "hello" {
					return Err(TestError::Error);
				}
				Ok(())
			})
		}
	}
	crate::create_test!(test_user_string_read_to_kernel_string_truncates_to_max_len);
}
