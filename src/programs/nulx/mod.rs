// the basis of this code is from here
// https://github.com/zesterer/chumsky/blob/main/examples/nano_rust.rs
// as the nulx language will be similar to rust but not quite
// slightly simpler syntax
// and directly interacts within the kernel

// TODO: split this code into a seperate crate later on.
pub mod lexer;
pub mod ast;
pub mod parser;
pub mod eval;
pub mod errors;
mod runtime;

pub use runtime::run;