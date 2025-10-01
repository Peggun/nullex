use alloc::{boxed::Box, string::ToString, vec::Vec};
use core::fmt;

use crate::programs::nulx::{
	errors::NulxError,
	lexer::{Span, Spanned}
};

#[derive(Clone, Debug, PartialEq)]
pub enum Token<'src> {
	Null,
	Bool(bool),
	Num(f64),
	Str(&'src str),
	Op(&'src str),
	Ctrl(char),
	Ident(&'src str),
	Func,
	Set,
	Print,
	If,
	Else
}

impl fmt::Display for Token<'_> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Token::Null => write!(f, "null"),
			Token::Bool(x) => write!(f, "{x}"),
			Token::Num(n) => write!(f, "{n}"),
			Token::Str(s) => write!(f, "{s}"),
			Token::Op(s) => write!(f, "{s}"),
			Token::Ctrl(c) => write!(f, "{c}"),
			Token::Ident(s) => write!(f, "{s}"),
			Token::Func => write!(f, "fn"),
			Token::Set => write!(f, "set"),
			Token::Print => write!(f, "print"),
			Token::If => write!(f, "if"),
			Token::Else => write!(f, "else")
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value<'src> {
	Null,
	Bool(bool),
	Num(f64),
	Str(&'src str),
	List(Vec<Self>),
	Func(&'src str)
}

impl Value<'_> {
	pub fn num(self, span: Span) -> Result<f64, NulxError> {
		if let Value::Num(x) = self {
			Ok(x)
		} else {
			Err(NulxError {
				span,
				msg: format!("'{self}' is not a number")
			})
		}
	}
}

impl fmt::Display for Value<'_> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Self::Null => write!(f, "null"),
			Self::Bool(x) => write!(f, "{x}"),
			Self::Num(x) => write!(f, "{x}"),
			Self::Str(x) => write!(f, "{x}"),
			Self::List(xs) => write!(
				f,
				"[{}]",
				xs.iter()
					.map(|x| x.to_string())
					.collect::<Vec<_>>()
					.join(", ")
			),
			Self::Func(name) => write!(f, "<function: {name}>")
		}
	}
}

#[derive(Clone, Debug)]
pub enum BinaryOp {
	Add,
	Sub,
	Mul,
	Div,
	Eq,
	NotEq
}

#[derive(Debug)]
pub enum Expr<'src> {
	Error,
	Value(Value<'src>),
	List(Vec<Spanned<Self>>),
	Local(&'src str),
	Set(&'src str, Box<Spanned<Self>>, Box<Spanned<Self>>),
	Then(Box<Spanned<Self>>, Box<Spanned<Self>>),
	Binary(Box<Spanned<Self>>, BinaryOp, Box<Spanned<Self>>),
	Call(Box<Spanned<Self>>, Spanned<Vec<Spanned<Self>>>),
	If(Box<Spanned<Self>>, Box<Spanned<Self>>, Box<Spanned<Self>>),
	Print(Box<Spanned<Self>>)
}

#[derive(Debug)]
pub struct Func<'src> {
	pub args: Vec<&'src str>,
	pub span: Span,
	pub body: Spanned<Expr<'src>>
}
