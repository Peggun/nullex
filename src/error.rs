use thiserror::Error;

#[derive(Error, Debug, Clone, Copy)]
pub enum NullexError {
	/// --- Serial Output Errors --- ///
	#[error("generic serial error")]
	GenericSerialError
}

// error consts
pub const EBADF: i32 = 9;
pub const ENOTTY: i32 = 25;
