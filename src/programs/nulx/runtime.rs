use alloc::{
	string::{String, ToString},
	vec::Vec
};

use chumsky::{Parser, error::Rich, input::Input};

use crate::{
	fs::{self, resolve_path},
	println,
	programs::nulx::{errors::NulxInputError, eval::eval_expr, lexer::lexer, parser::funcs_parser}
};

pub fn run(args: &[&str]) {
	println!("code is depreciated. a kernel doesnt actually need these. when a package manager becomes available for nullex, i will happily add this to the repo.");
	return;

	if args.is_empty() {
		println!("nulx: missing file operand");
		return;
	}

	let filename = args[0];
	let path = resolve_path(filename);
	// never knew you can specify a return type through this |_|
	let src = fs::with_fs(|fs| -> Result<String, NulxInputError> {
		match fs.read_file(&path) {
			Ok(content) => {
				let s = String::from_utf8_lossy_owned(content.to_vec());
				Ok(s)
			}
			Err(_) => Err(NulxInputError::NoSuchFile(path))
		}
	})
	.unwrap();

	let (tokens, mut errs) = lexer().parse(&src).into_output_errors();

	let parse_errs = if let Some(tokens) = &tokens {
		let (ast, parse_errs) = funcs_parser()
			.map_with(|ast, e| (ast, e.span()))
			.parse(
				tokens
					.as_slice()
					.map((src.len()..src.len()).into(), |(t, s)| (t, s))
			)
			.into_output_errors();

		if let Some((funcs, file_span)) = ast.filter(|_| errs.len() + parse_errs.len() == 0) {
			if let Some(main) = funcs.get("main") {
				if !main.args.is_empty() {
					errs.push(Rich::custom(
						main.span,
						"The main function cannot have arguments".to_string()
					))
				} else {
					match eval_expr(&main.body, &funcs, &mut Vec::new()) {
						Ok(val) => println!("Return value: {val}"),
						Err(e) => errs.push(Rich::custom(e.span, e.msg))
					}
				}
			} else {
				errs.push(Rich::custom(
					file_span,
					"Programs need a main function but none was found".to_string()
				));
			}
		}

		parse_errs
	} else {
		Vec::new()
	};

	println!("errors: {:#?}", parse_errs);
}
