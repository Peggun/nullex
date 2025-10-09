// https://github.com/StephanvanSchaik/simple-bits/blob/main/src/lib.rs
// im not very good with bitwise ops. this helped alot.

use core::ops::Range;

use crate::println;

pub trait BitsExt {
    fn extract_bit(self, index: usize) -> bool;
    fn extract_bits(self, range: Range<usize>) -> Self;
    fn replace_bit(self, index: usize, value: bool) -> Self;
    fn replace_bits(self, range: Range<usize>, value: Self) -> Self;
}

macro_rules! bits_ext_impl_for {
    ($t:ident) => {
        impl BitsExt for $t {
            fn extract_bit(self, index: usize) -> bool {
                (self >> index) & 1 == 1
            }

            fn extract_bits(self, range: Range<usize>) -> Self {
                (self >> range.start) & ((1 << range.len()) - 1)
            }

            fn replace_bit(self, index: usize, value: bool) -> Self {
                (self & !(1 << index)) | ((value as Self) << index)
            }

            // had to change this.
            fn replace_bits(self, range: Range<usize>, value: Self) -> Self {
                let len = range.len();
                let mask: u64 = if len >= 64 {
                    !0u64
                } else {
                    (1u64 << len) - 1u64
                };

                (self & !(mask << range.start) as Self) | ((value & mask as Self) << range.start)
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
pub fn sign_extend(x: u64, b: u32) -> u64 {
    let m = 1u64 << (b - 1);
    let x = x & ((1u64 << b) - 1);
    let n = (((x ^ m) as i64) - (m as i64)) as u64;

    n
}