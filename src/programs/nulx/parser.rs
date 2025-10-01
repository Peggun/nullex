use alloc::{boxed::Box, vec::Vec};

use chumsky::{Parser, extra, input::ValueInput, prelude::*};
use hashbrown::HashMap;

use crate::programs::nulx::{
	ast::{BinaryOp, Expr, Func, Token, Value},
	lexer::{Span, Spanned}
};

pub fn expr_parser<'tokens, 'src: 'tokens, I>()
-> impl Parser<'tokens, I, Spanned<Expr<'src>>, extra::Err<Rich<'tokens, Token<'src>, Span>>> + Clone
where
	I: ValueInput<'tokens, Token = Token<'src>, Span = Span>
{
	recursive(|expr| {
		let inline_expr = recursive(|inline_expr| {
			let val = select! {
				Token::Null => Expr::Value(Value::Null),
				Token::Bool(x) => Expr::Value(Value::Bool(x)),
				Token::Num(n) => Expr::Value(Value::Num(n)),
				Token::Str(s) => Expr::Value(Value::Str(s)),
			}
			.labelled("value");

			let ident = select! { Token::Ident(ident) => ident }.labelled("identifier");

			// a list of expressions
			let items = expr
				.clone()
				.separated_by(just(Token::Ctrl(',')))
				.allow_trailing()
				.collect::<Vec<_>>();

			let set_ = just(Token::Set)
				.ignore_then(ident)
				.then_ignore(just(Token::Op("=")))
				.then(inline_expr)
				.then_ignore(just(Token::Ctrl(';')))
				.then(expr.clone())
				.map(|((name, val), body)| Expr::Set(name, Box::new(val), Box::new(body)));

			let list = items
				.clone()
				.map(Expr::List)
				.delimited_by(just(Token::Ctrl('[')), just(Token::Ctrl(']')));

			// atoms are expressions that contain no ambiguity
			let atom = val
                .or(ident.map(Expr::Local))
                .or(set_)
                .or(list)
                // keyword, not like macro, just easier tbh
                .or(just(Token::Print)
                    .ignore_then(
                        expr.clone()
                            .delimited_by(just(Token::Ctrl('(')), just(Token::Ctrl(')'))),
                    )
                    .map(|expr| Expr::Print(Box::new(expr))))
                .map_with(|expr, e| (expr, e.span()))
                // atoms can just be normal expressions, but surrounded with parentheses
                .or(expr
                    .clone()
                    .delimited_by(just(Token::Ctrl('(')), just(Token::Ctrl(')'))))
                // attempt to recover anything that looks like a parenthesised expression but contains errors
                .recover_with(via_parser(nested_delimiters(
                    Token::Ctrl('('),
                    Token::Ctrl(')'),
                    [
                        (Token::Ctrl('['), Token::Ctrl(']')),
                        (Token::Ctrl('{'), Token::Ctrl('}')),
                    ],
                    |span| (Expr::Error, span),
                )))
                // attempt to recover anything that looks like a list but contains errors
                .recover_with(via_parser(nested_delimiters(
                    Token::Ctrl('['),
                    Token::Ctrl(']'),
                    [
                        (Token::Ctrl('('), Token::Ctrl(')')),
                        (Token::Ctrl('{'), Token::Ctrl('}')),
                    ],
                    |span| (Expr::Error, span),
                )))
                .boxed();

			// function calls have very high precedence so we prioritise them
			let call = atom.foldl_with(
				items
					.delimited_by(just(Token::Ctrl('(')), just(Token::Ctrl(')')))
					.map_with(|args, e| (args, e.span()))
					.repeated(),
				|f, args, e| (Expr::Call(Box::new(f), args), e.span())
			);

			// product ops (multiply and divide) have equal precedence
			let op = just(Token::Op("*"))
				.to(BinaryOp::Mul)
				.or(just(Token::Op("/")).to(BinaryOp::Div));
			let product = call
				.clone()
				.foldl_with(op.then(call).repeated(), |a, (op, b), e| {
					(Expr::Binary(Box::new(a), op, Box::new(b)), e.span())
				});

			// sum ops (add and subtract) have equal precedence
			let op = just(Token::Op("+"))
				.to(BinaryOp::Add)
				.or(just(Token::Op("-")).to(BinaryOp::Sub));
			let sum = product
				.clone()
				.foldl_with(op.then(product).repeated(), |a, (op, b), e| {
					(Expr::Binary(Box::new(a), op, Box::new(b)), e.span())
				});

			// comparison ops (equal, not-equal) have equal precedence
			let op = just(Token::Op("=="))
				.to(BinaryOp::Eq)
				.or(just(Token::Op("!=")).to(BinaryOp::NotEq));
			let compare = sum
				.clone()
				.foldl_with(op.then(sum).repeated(), |a, (op, b), e| {
					(Expr::Binary(Box::new(a), op, Box::new(b)), e.span())
				});

			compare.labelled("expression").as_context()
		});

		// blocks are expressions but delimited with braces
		let block = expr
            .clone()
            .delimited_by(just(Token::Ctrl('{')), just(Token::Ctrl('}')))
            // attempt to recover anything that looks like a block but contains errors
            .recover_with(via_parser(nested_delimiters(
                Token::Ctrl('{'),
                Token::Ctrl('}'),
                [
                    (Token::Ctrl('('), Token::Ctrl(')')),
                    (Token::Ctrl('['), Token::Ctrl(']')),
                ],
                |span| (Expr::Error, span),
            )));

		let if_ = recursive(|if_| {
			just(Token::If)
				.ignore_then(expr.clone())
				.then(block.clone())
				.then(
					just(Token::Else)
						.ignore_then(block.clone().or(if_))
						.or_not()
				)
				.map_with(|((cond, a), b), e| {
					(
						Expr::If(
							Box::new(cond),
							Box::new(a),
							// If an if expression has no trailing else block, we magic up one that
							// just produces null
							Box::new(b.unwrap_or_else(|| (Expr::Value(Value::Null), e.span())))
						),
						e.span()
					)
				})
		});

		// both blocks and if are block expressions and can appear in the place of
		// statements
		let block_expr = block.or(if_);

		let block_chain = block_expr
			.clone()
			.foldl_with(block_expr.clone().repeated(), |a, b, e| {
				(Expr::Then(Box::new(a), Box::new(b)), e.span())
			});

		let block_recovery = nested_delimiters(
			Token::Ctrl('{'),
			Token::Ctrl('}'),
			[
				(Token::Ctrl('('), Token::Ctrl(')')),
				(Token::Ctrl('['), Token::Ctrl(']'))
			],
			|span| (Expr::Error, span)
		);

		block_chain
            .labelled("block")
            // expressions, chained by semicolons, are statements
            .or(inline_expr.clone())
            .recover_with(skip_then_retry_until(
                block_recovery.ignored().or(any().ignored()),
                one_of([
                    Token::Ctrl(';'),
                    Token::Ctrl('}'),
                    Token::Ctrl(')'),
                    Token::Ctrl(']'),
                ])
                .ignored(),
            ))
            .foldl_with(
                just(Token::Ctrl(';')).ignore_then(expr.or_not()).repeated(),
                |a, b, e| {
                    let span: Span = e.span();
                    (
                        Expr::Then(
                            Box::new(a),
                            // If there is no b expression then its span is the end of the statement/block.
                            Box::new(
                                b.unwrap_or_else(|| (Expr::Value(Value::Null), span.to_end())),
                            ),
                        ),
                        span,
                    )
                },
            )
	})
}

pub fn funcs_parser<'tokens, 'src: 'tokens, I>()
-> impl Parser<'tokens, I, HashMap<&'src str, Func<'src>>, extra::Err<Rich<'tokens, Token<'src>, Span>>>
+ Clone
where
	I: ValueInput<'tokens, Token = Token<'src>, Span = Span>
{
	let ident = select! { Token::Ident(ident) => ident };

	// arg lists are just identifiers separated by commas, surrounded by parentheses
	let args = ident
		.separated_by(just(Token::Ctrl(',')))
		.allow_trailing()
		.collect()
		.delimited_by(just(Token::Ctrl('(')), just(Token::Ctrl(')')))
		.labelled("function args");

	let func = just(Token::Func)
		.ignore_then(
			ident
				.map_with(|name, e| (name, e.span()))
				.labelled("function name")
		)
		.then(args)
		.map_with(|start, e| (start, e.span()))
		.then(
			expr_parser()
                .delimited_by(just(Token::Ctrl('{')), just(Token::Ctrl('}')))
                // attempt to recover anything that looks like a function body but contains errors
                .recover_with(via_parser(nested_delimiters(
                    Token::Ctrl('{'),
                    Token::Ctrl('}'),
                    [
                        (Token::Ctrl('('), Token::Ctrl(')')),
                        (Token::Ctrl('['), Token::Ctrl(']')),
                    ],
                    |span| (Expr::Error, span),
                )))
		)
		.map(|(((name, args), span), body)| {
			(name, Func {
				args,
				span,
				body
			})
		})
		.labelled("function");

	func.repeated()
		.collect::<Vec<_>>()
		.validate(|fs, _, emitter| {
			let mut funcs = HashMap::new();
			for ((name, name_span), f) in fs {
				if funcs.insert(name, f).is_some() {
					emitter.emit(Rich::custom(
						name_span,
						format!("Function '{name}' already exists")
					));
				}
			}
			funcs
		})
}
