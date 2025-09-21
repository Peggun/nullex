use alloc::vec::Vec;
use hashbrown::HashMap;

use crate::{println, programs::nulx::{ast::{BinaryOp, Expr, Func, Value}, errors::NulxError, lexer::Spanned}};

pub fn eval_expr<'src>(
    expr: &Spanned<Expr<'src>>,
    funcs: &HashMap<&'src str, Func<'src>>,
    stack: &mut Vec<(&'src str, Value<'src>)>,
) -> Result<Value<'src>, NulxError> {
    Ok(match &expr.0 {
        Expr::Error => unreachable!(), // err expressions only get created by parser errors, so cannot exist in a valid AST
        Expr::Value(val) => val.clone(),
        Expr::List(items) => Value::List(
            items
                .iter()
                .map(|item| eval_expr(item, funcs, stack))
                .collect::<Result<_, _>>()?,
        ),
        Expr::Local(name) => stack
            .iter()
            .rev()
            .find(|(l, _)| l == name)
            .map(|(_, v)| v.clone())
            .or_else(|| Some(Value::Func(name)).filter(|_| funcs.contains_key(name)))
            .ok_or_else(|| NulxError {
                span: expr.1,
                msg: format!("No such variable '{name}' in scope"),
            })?,
        Expr::Set(local, val, body) => {
            let val = eval_expr(val, funcs, stack)?;
            stack.push((local, val));
            let res = eval_expr(body, funcs, stack)?;
            stack.pop();
            res
        }
        Expr::Then(a, b) => {
            eval_expr(a, funcs, stack)?;
            eval_expr(b, funcs, stack)?
        }
        Expr::Binary(a, BinaryOp::Add, b) => Value::Num(
            eval_expr(a, funcs, stack)?.num(a.1)? + eval_expr(b, funcs, stack)?.num(b.1)?,
        ),
        Expr::Binary(a, BinaryOp::Sub, b) => Value::Num(
            eval_expr(a, funcs, stack)?.num(a.1)? - eval_expr(b, funcs, stack)?.num(b.1)?,
        ),
        Expr::Binary(a, BinaryOp::Mul, b) => Value::Num(
            eval_expr(a, funcs, stack)?.num(a.1)? * eval_expr(b, funcs, stack)?.num(b.1)?,
        ),
        Expr::Binary(a, BinaryOp::Div, b) => Value::Num(
            eval_expr(a, funcs, stack)?.num(a.1)? / eval_expr(b, funcs, stack)?.num(b.1)?,
        ),
        Expr::Binary(a, BinaryOp::Eq, b) => {
            Value::Bool(eval_expr(a, funcs, stack)? == eval_expr(b, funcs, stack)?)
        }
        Expr::Binary(a, BinaryOp::NotEq, b) => {
            Value::Bool(eval_expr(a, funcs, stack)? != eval_expr(b, funcs, stack)?)
        }
        Expr::Call(func, args) => {
            let f = eval_expr(func, funcs, stack)?;
            match f {
                Value::Func(name) => {
                    let f = &funcs[&name];
                    let mut stack = if f.args.len() != args.0.len() {
                        return Err(NulxError {
                            span: expr.1,
                            msg: format!("'{}' called with wrong number of arguments (expected {name}, found {})", f.args.len(), args.0.len()),
                        });
                    } else {
                        f.args
                            .iter()
                            .zip(args.0.iter())
                            .map(|(name, arg)| Ok((*name, eval_expr(arg, funcs, stack)?)))
                            .collect::<Result<_, _>>()?
                    };
                    eval_expr(&f.body, funcs, &mut stack)?
                }
                f => {
                    return Err(NulxError {
                        span: func.1,
                        msg: format!("'{f:?}' is not callable"),
                    })
                }
            }
        }
        Expr::If(cond, a, b) => {
            let c = eval_expr(cond, funcs, stack)?;
            match c {
                Value::Bool(true) => eval_expr(a, funcs, stack)?,
                Value::Bool(false) => eval_expr(b, funcs, stack)?,
                c => {
                    return Err(NulxError {
                        span: cond.1,
                        msg: format!("Conditions must be booleans, found '{c:?}'"),
                    })
                }
            }
        }
        Expr::Print(a) => {
            let val = eval_expr(a, funcs, stack)?;
            println!("{val}");
            val
        }
    })
}