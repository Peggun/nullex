//! error.rs
//! 
//! Error handling module for the kernel.

use thiserror::Error;

#[derive(Error, Debug, Clone, Copy)]
/// A enum representing all Nullex Errors
// TODO: Actually start using this, ngl i completely forgot this was here.
pub enum NullexError {
	/// --- Serial Output Errors --- ///
	#[error("generic serial error")]
	GenericSerialError
}

// error consts
#[allow(dead_code)]
const EBADF: i32 = 9;
#[allow(dead_code)]
const ENOTTY: i32 = 25;