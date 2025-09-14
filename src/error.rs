use thiserror::Error;

#[derive(Error, Debug, Clone, Copy)]
pub enum NullexError {
	/// --- Serial Output Errors --- ///
	#[error("generic serial error")]
	GenericSerialError
}
