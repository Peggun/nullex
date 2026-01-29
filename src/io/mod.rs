use crate::{
	common::ports::{inb, inl, inq, inw, outb, outl, outq, outw},
	utils::types::{BYTE, DWORD, QWORD, WORD}
};

pub mod keyboard;
pub mod pci;

pub fn io_read<N>(base: usize, offset: usize) -> Result<N, <N as TryFrom<u64>>::Error>
where
	N: TryFrom<u64> + Copy
{
	if size_of::<N>() == 1 {
		let val = unsafe { inb((base + offset) as u16) };
		N::try_from(val as u64)
	} else if size_of::<N>() == 2 {
		let val = unsafe { inw((base + offset) as u16) };
		N::try_from(val as u64)
	} else if size_of::<N>() == 4 {
		let val = unsafe { inl((base + offset) as u16) };
		N::try_from(val as u64)
	} else if size_of::<N>() == 8 {
		let val = unsafe { inq((base + offset) as u16) };
		N::try_from(val)
	} else {
		unimplemented!("not sure how you got here.")
	}
}

pub fn io_write<N>(base: usize, offset: usize, value: N) -> Result<(), &'static str>
where
	N: Into<u64> + Copy
{
	let value = value.into();

	if size_of::<N>() == 1 {
		unsafe { outb((base + offset) as u16, value as BYTE) };
	} else if size_of::<N>() == 2 {
		unsafe { outw((base + offset) as u16, value as WORD) };
	} else if size_of::<N>() == 4 {
		unsafe { outl((base + offset) as u16, value as DWORD) };
	} else if size_of::<N>() == 8 {
		unsafe { outq((base + offset) as u16, value as QWORD) };
	} else {
		unimplemented!("not sure how you got here.")
	}

	Ok(())
}
