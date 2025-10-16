
// im not very good with bitwise ops. this helped alot.
pub trait BitsExt {
    const LENGTH: usize;

    fn get_bit(&self, index: usize) -> bool;
    fn get_bits(&self, from: usize, to: usize) -> Self;
    fn set_bit(&self, index: usize, value: bool) -> Self;
    fn set_bits(&self, from: usize, to: usize, value: usize) -> Self;
}

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
    }
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