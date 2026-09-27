use bytemuck::{Pod, Zeroable};

pub const NDM_FILE_NAME_SIZE: usize = 64;
pub const NDM_FUNCTION_NAME_SIZE: usize = 64;

#[used]
#[unsafe(link_section = ".ndm")]
pub static NDM_DATA: [u8; include_bytes!(concat!(
	env!("CARGO_MANIFEST_DIR"),
	"/build/nullex.ndm"
))
.len()] = *include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/build/nullex.ndm"));

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct NDMEntry {
	pub address: u64,
	pub line: u32,

	pub file_name: [u8; NDM_FILE_NAME_SIZE],
	pub func_name: [u8; NDM_FUNCTION_NAME_SIZE],

	_padding: u32 // 4 bytes
}

const _: () = assert!(core::mem::size_of::<NDMEntry>() == 144);
