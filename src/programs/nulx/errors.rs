use thiserror::Error;
use alloc::string::String;

use crate::programs::nulx::lexer::Span;

pub struct NulxError {
    pub span: Span,
    pub msg: String,
}

#[derive(Error, Debug)]
pub enum NulxInputError {
    #[error("nulx: No such file {0}")]
    NoSuchFile(String),
}