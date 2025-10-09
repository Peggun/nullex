use core::ops::{BitAnd, Shr};

fn one<N: TryFrom<u64>>() -> N {
    1u64.try_into().unwrap_or_else(|_| unreachable!()) // error handle here
}

pub fn get_lsb<N>(n: N) -> N
where
    N: BitAnd<Output = N> + TryFrom<u64>,
{   
    n & one()
}

pub fn get_msb<N>(n: N) -> N
where 
    N: Shr<usize, Output = N> + BitAnd<Output = N> + TryFrom<u64>,
{
    let shift = size_of::<N>() * 8 - 1;
    (n >> shift) & one()
}