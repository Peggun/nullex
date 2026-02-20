//! 
//! bits.rs
//! 
//! Bitwise operation helpers for the kernel.
//! 

use alloc::vec::Vec;
use core::range::Range;

// im not very good with bitwise ops. this helped alot.
/// Trait allowing for more complex operations with generic types.
pub trait BitsExt {
	/// Lenght of the type implementing `BitsExt`
	const LENGTH: usize;

	/// Gets a bits from a type which implements `BitsExt`
	fn get_bit(&self, index: usize) -> bool;
	/// Gets a range of bits from a type which implements `BitsExt`
	fn get_bits(&self, from: usize, to: usize) -> Self;
	/// Sets a bit from a type which implements `BitsExt`
	fn set_bit(&self, index: usize, value: bool) -> Self;
	/// Sets a range of bits from a type which implements `BitsExt`
	fn set_bits(&self, from: usize, to: usize, value: usize) -> Self;	
}

/// Implements the `BitsExt` trait for integer types.
///
/// This macro generates bitwise operation methods for unsigned and signed integer types,
/// including `get_bit`, `set_bit`, `get_bits`, and `set_bits` operations.
///
/// # Example
///
/// ```ignore
/// bits_ext_impl_for!(u32);
/// bits_ext_impl_for!(i64);
/// ```
///
/// After expansion, you can use bitwise operations like:
///
/// ```ignore
/// let value: u32 = 0b1010;
/// let bit = value.get_bit(2);        // Get bit at index 2
/// let modified = value.set_bit(0, true);  // Set bit 0 to 1
/// let range = value.get_bits(1, 3);  // Get bits from index 1 to 3
/// ```
macro_rules! bits_ext_impl_for {
	($t:ident) => {
		impl BitsExt for $t {
			// https://stackoverflow.com/questions/47981/how-to-set-clear-and-toggle-a-single-bit
			// https://stackoverflow.com/questions/22662807/c-most-efficient-way-to-set-all-bits-in-a-range-within-a-variable

			const LENGTH: usize = core::mem::size_of::<Self>() as usize * 8;

			fn get_bit(&self, index: usize) -> bool {
				if index < Self::LENGTH {
					return (*self & (1 << index)) != 0;
				}
				false
			}

			fn get_bits(&self, from: usize, to: usize) -> Self {
				if from >= Self::LENGTH || to > Self::LENGTH || from >= to {
					return 0;
				}

				let width = to - from;
				if width == 0 {
					return 0;
				}

				let mask: Self = (!0 as Self) >> (Self::LENGTH - width);

				(*self >> from) & mask
			}

			fn set_bit(&self, index: usize, val: bool) -> Self {
				let x: Self = if val == true { 1 } else { 0 };

				if index < Self::LENGTH {
					return (*self & !(1 << index)) | (x << index);
				}
				0
			}

			fn set_bits(&self, from: usize, to: usize, value: usize) -> Self {
				if from >= Self::LENGTH || to > Self::LENGTH || from >= to {
					return 0;
				}

				let width = to - from;
				if width == 0 {
					return 0;
				}

				let mask: Self = (!0 as Self) >> (Self::LENGTH >> width);

				let val_trunc = (value as Self) & mask;
				((*self & !(mask << from)) | (val_trunc << from))
			}
		}
	};
}

bits_ext_impl_for!(u8);
bits_ext_impl_for!(u16);
bits_ext_impl_for!(u32);
bits_ext_impl_for!(u64);
bits_ext_impl_for!(usize);

bits_ext_impl_for!(i8);
bits_ext_impl_for!(i16);
bits_ext_impl_for!(i32);
bits_ext_impl_for!(i64);
bits_ext_impl_for!(isize);

// https://graphics.stanford.edu/~seander/bithacks.html#FixedSignExtend
// https://stackoverflow.com/questions/5814072/sign-extend-a-nine-bit-number-in-c
// holy crap is that useful
/// Sign extends a number.
pub fn sign_extend(x: u64, b: u32) -> u64 {
	let m = 1u64 << (b - 1);
	let x = x & ((1u64 << b) - 1);

	(((x ^ m) as i64) - (m as i64)) as u64
}

// bitmaps
#[derive(Debug, Clone)]
/// A structure representing a Bitmap (an array of 1's and 0's)
pub struct BitMap {
	size: u64,
	table: Vec<bool>
}

impl BitMap {
	/// Creates a new `BitMap` of `size` size
	pub fn new(size: u64) -> BitMap {
		BitMap {
			size,
			table: vec![false; size as usize]
		}
	}

	/// Sets the index `idx` to either on (`true`) or off (`false`)
	pub fn set_idx(&mut self, idx: usize, set: bool) {
		if idx > self.size as usize {
			return
		}

		self.table[idx] = set;
	}

	/// Sets a range of indexes `idxs` to either on (`true`) or off (`false`)
	pub fn set_idxs(&mut self, idxs: Range<usize>, set: bool) {
		for idx in idxs {
			if idx > self.size as usize {
				return
			}

			self.set_idx(idx, set);
		}
	}

	/// Gets the index `idx` and returns either on (`true`) or off (`false`)
	pub fn get_idx(&self, idx: usize) -> bool {
		if idx > self.size as usize {
			return false
		}

		self.table[idx]
	}

	/// Gets a range of indexes `idxs` and returns a `Vec` with either on (`true`) or off (`false`) for each index.
	pub fn get_idxs(&self, idxs: Range<usize>) -> Vec<bool> {
		let mut indexs = Vec::new();
		for idx in idxs {
			if idx > self.size as usize {
				continue
			}

			indexs.push(self.get_idx(idx));
		}
		indexs
	}
}
