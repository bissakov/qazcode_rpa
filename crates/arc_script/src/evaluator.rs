use crate::ast::{Expr, InterpolationSegment};
use crate::value::{Value, ValueExt, VariableResolver};
use std::fmt::Write;

/// Evaluates an expression to a value.
///
/// # Errors
///
/// Returns an error if the expression cannot be evaluated (e.g., type mismatch, division by zero).
pub fn eval_expr(expr: &Expr, resolver: &dyn VariableResolver) -> Result<Value, String> {
    match expr {
        Expr::Const(v) => Ok(v.clone()),

        Expr::Load(name) => resolver.resolve(name),

        Expr::Add(a, b) => {
            let left = eval_expr(a, resolver)?;
            match left {
                Value::String(_) => {
                    let right = eval_expr(b, resolver)?;
                    Ok(Value::String(format!("{left}{right}")))
                }
                Value::Number(_) => {
                    let right = eval_expr(b, resolver)?;
                    let right_num = right.to_number()?;
                    Ok(Value::Number(left.to_number()? + right_num))
                }
                Value::Boolean(_) => Err("Cannot use + with boolean on left side".to_string()),
                Value::Undefined => Err("Cannot use + with undefined".to_string()),
            }
        }

        Expr::Sub(a, b) => Ok(Value::Number(
            eval_expr(a, resolver)?.to_number()? - eval_expr(b, resolver)?.to_number()?,
        )),

        Expr::Mul(a, b) => Ok(Value::Number(
            eval_expr(a, resolver)?.to_number()? * eval_expr(b, resolver)?.to_number()?,
        )),

        Expr::Div(a, b) => {
            let rhs = eval_expr(b, resolver)?.to_number()?;
            if rhs.abs() < f64::EPSILON {
                return Err("Division by zero".into());
            }
            Ok(Value::Number(eval_expr(a, resolver)?.to_number()? / rhs))
        }

        Expr::Mod(a, b) => {
            let rhs = eval_expr(b, resolver)?.to_number()?;
            if rhs.abs() < f64::EPSILON {
                return Err("Division by zero".into());
            }
            Ok(Value::Number(eval_expr(a, resolver)?.to_number()? % rhs))
        }

        Expr::Neg(e) => Ok(Value::Number(-eval_expr(e, resolver)?.to_number()?)),

        Expr::Eq(a, b) => {
            let lhs = eval_expr(a, resolver)?;
            let rhs = eval_expr(b, resolver)?;
            if std::mem::discriminant(&lhs) != std::mem::discriminant(&rhs) {
                return Err("Type mismatch in '=='".to_string());
            }
            Ok(Value::Boolean(lhs == rhs))
        }

        Expr::Ne(a, b) => {
            let lhs = eval_expr(a, resolver)?;
            let rhs = eval_expr(b, resolver)?;
            if std::mem::discriminant(&lhs) != std::mem::discriminant(&rhs) {
                return Err("Type mismatch in '!='".to_string());
            }
            Ok(Value::Boolean(lhs != rhs))
        }

        Expr::Gt(a, b) => Ok(Value::Boolean(
            eval_expr(a, resolver)?.to_number()? > eval_expr(b, resolver)?.to_number()?,
        )),

        Expr::Ge(a, b) => Ok(Value::Boolean(
            eval_expr(a, resolver)?.to_number()? >= eval_expr(b, resolver)?.to_number()?,
        )),

        Expr::Lt(a, b) => Ok(Value::Boolean(
            eval_expr(a, resolver)?.to_number()? < eval_expr(b, resolver)?.to_number()?,
        )),

        Expr::Le(a, b) => Ok(Value::Boolean(
            eval_expr(a, resolver)?.to_number()? <= eval_expr(b, resolver)?.to_number()?,
        )),

        Expr::And(a, b) => Ok(Value::Boolean(
            eval_expr(a, resolver)?.to_bool()? && eval_expr(b, resolver)?.to_bool()?,
        )),

        Expr::Or(a, b) => Ok(Value::Boolean(
            eval_expr(a, resolver)?.to_bool()? || eval_expr(b, resolver)?.to_bool()?,
        )),

        Expr::Not(e) => Ok(Value::Boolean(!eval_expr(e, resolver)?.to_bool()?)),

        Expr::InterpolatedString(segments) => {
            let mut result = String::new();
            for segment in segments {
                match segment {
                    InterpolationSegment::Literal(s) => result.push_str(s),
                    InterpolationSegment::Expression(expr) => {
                        let val = eval_expr(expr, resolver)?;
                        write!(&mut result, "{val}").unwrap();
                    }
                }
            }
            Ok(Value::String(result))
        }
    }
}
