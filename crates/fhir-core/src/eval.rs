//! Tree-walking evaluator for parsed FHIRPath expressions. It only navigates
//! JSON-derived values; nothing is compiled or executed.

use std::cmp::Ordering;

use rust_decimal::Decimal;

use crate::ast::{BinOp, Expr, Literal, TypeOp};
use crate::error::Error;
use crate::value::{Value, from_json, matches_type};

pub fn eval(expr: &Expr, focus: &[Value]) -> Result<Vec<Value>, Error> {
    match expr {
        Expr::Literal(lit) => Ok(vec![literal_value(lit)]),
        Expr::This => Ok(focus.to_vec()),
        Expr::Identifier(name) => {
            let mut out = Vec::new();
            for item in focus {
                // a head matching the resource type selects the resource itself
                if let Value::Complex { ty: Some(t), .. } = item
                    && t == name
                {
                    out.push(item.clone());
                    continue;
                }
                out.extend(access(item, name));
            }
            Ok(out)
        }
        Expr::Member { base, name } => {
            let base = eval(base, focus)?;
            Ok(base.iter().flat_map(|item| access(item, name)).collect())
        }
        Expr::Index { base, index } => {
            let items = eval(base, focus)?;
            let idx = eval(index, focus)?;
            let n = match idx.as_slice() {
                [Value::Integer(n)] => *n,
                other => {
                    return Err(Error::Eval(format!(
                        "index must be a single integer, got {other:?}"
                    )));
                }
            };
            if n < 0 {
                return Ok(vec![]);
            }
            Ok(items.into_iter().nth(n as usize).into_iter().collect())
        }
        Expr::Binary { op, lhs, rhs } => {
            let lhs = eval(lhs, focus)?;
            let rhs = eval(rhs, focus)?;
            binary(*op, lhs, rhs)
        }
        Expr::Call { base, name, args } => {
            // a method call's input is its base; a bare call's input is the focus
            let input = match base {
                Some(b) => eval(b, focus)?,
                None => focus.to_vec(),
            };
            crate::functions::call(name, &input, args, focus)
        }
        Expr::TypeTest {
            op,
            expr,
            type_name,
        } => {
            let operand = eval(expr, focus)?;
            match op {
                TypeOp::Is => match singleton(&operand)? {
                    None => Ok(vec![]),
                    Some(v) => Ok(vec![Value::Boolean(matches_type(v, type_name))]),
                },
                TypeOp::As => Ok(operand
                    .into_iter()
                    .filter(|v| matches_type(v, type_name))
                    .collect()),
            }
        }
    }
}

fn binary(op: BinOp, lhs: Vec<Value>, rhs: Vec<Value>) -> Result<Vec<Value>, Error> {
    let result = match op {
        // equality on empty is empty, not false: missing data is unknown, not unequal
        BinOp::Eq | BinOp::Ne => {
            if lhs.is_empty() || rhs.is_empty() {
                return Ok(vec![]);
            }
            let equal = lhs.len() == rhs.len() && lhs.iter().zip(&rhs).all(|(a, b)| a == b);
            Value::Boolean(if op == BinOp::Eq { equal } else { !equal })
        }
        BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
            let (Some(a), Some(b)) = (singleton(&lhs)?, singleton(&rhs)?) else {
                return Ok(vec![]);
            };
            let ord = compare(a, b)?;
            Value::Boolean(match op {
                BinOp::Lt => ord == Ordering::Less,
                BinOp::Le => ord != Ordering::Greater,
                BinOp::Gt => ord == Ordering::Greater,
                _ => ord != Ordering::Less,
            })
        }
        // three-valued logic: an empty operand is "unknown", not false
        BinOp::And | BinOp::Or => {
            let a = boolean_of(&lhs)?;
            let b = boolean_of(&rhs)?;
            let known = match (op, a, b) {
                (BinOp::And, Some(false), _) | (BinOp::And, _, Some(false)) => Some(false),
                (BinOp::And, Some(true), Some(true)) => Some(true),
                (BinOp::Or, Some(true), _) | (BinOp::Or, _, Some(true)) => Some(true),
                (BinOp::Or, Some(false), Some(false)) => Some(false),
                _ => None,
            };
            match known {
                Some(b) => Value::Boolean(b),
                None => return Ok(vec![]),
            }
        }
        BinOp::In => {
            let Some(a) = singleton(&lhs)? else {
                return Ok(vec![]);
            };
            Value::Boolean(rhs.contains(a))
        }
        // union is a set merge: duplicates collapse
        BinOp::Union => {
            let mut out: Vec<Value> = Vec::new();
            for v in lhs.into_iter().chain(rhs) {
                if !out.contains(&v) {
                    out.push(v);
                }
            }
            return Ok(out);
        }
        BinOp::Concat => {
            let mut s = concat_operand(&lhs)?;
            s.push_str(&concat_operand(&rhs)?);
            Value::String(s)
        }
    };
    Ok(vec![result])
}

// concat treats an empty operand as the empty string
fn concat_operand(vals: &[Value]) -> Result<String, Error> {
    match singleton(vals)? {
        None => Ok(String::new()),
        Some(Value::String(s)) => Ok(s.clone()),
        Some(other) => Err(Error::Eval(format!("expected a string, got {other:?}"))),
    }
}

fn singleton(vals: &[Value]) -> Result<Option<&Value>, Error> {
    match vals {
        [] => Ok(None),
        [v] => Ok(Some(v)),
        _ => Err(Error::Eval("expected a single value".into())),
    }
}

pub(crate) fn boolean_of(vals: &[Value]) -> Result<Option<bool>, Error> {
    match singleton(vals)? {
        None => Ok(None),
        Some(Value::Boolean(b)) => Ok(Some(*b)),
        Some(other) => Err(Error::Eval(format!("expected a boolean, got {other:?}"))),
    }
}

fn compare(a: &Value, b: &Value) -> Result<Ordering, Error> {
    let ord = match (a, b) {
        (Value::Integer(x), Value::Integer(y)) => x.cmp(y),
        (Value::Decimal(x), Value::Decimal(y)) => x.cmp(y),
        (Value::Integer(x), Value::Decimal(y)) => Decimal::from(*x).cmp(y),
        (Value::Decimal(x), Value::Integer(y)) => x.cmp(&Decimal::from(*y)),
        (Value::String(x), Value::String(y))
        | (Value::Date(x), Value::Date(y))
        | (Value::DateTime(x), Value::DateTime(y))
        | (Value::Date(x), Value::String(y))
        | (Value::String(x), Value::Date(y))
        | (Value::DateTime(x), Value::String(y))
        | (Value::String(x), Value::DateTime(y)) => x.cmp(y),
        _ => return Err(Error::Eval(format!("cannot compare {a:?} and {b:?}"))),
    };
    Ok(ord)
}

fn literal_value(lit: &Literal) -> Value {
    match lit {
        Literal::Boolean(b) => Value::Boolean(*b),
        Literal::Integer(i) => Value::Integer(*i),
        Literal::Decimal(d) => Value::Decimal(*d),
        Literal::Str(s) => Value::String(s.clone()),
        Literal::Date(s) => Value::Date(s.clone()),
        Literal::DateTime(s) => Value::DateTime(s.clone()),
    }
}

pub(crate) fn access(item: &Value, name: &str) -> Vec<Value> {
    let Value::Complex { data, .. } = item else {
        return vec![];
    };
    if let Some(child) = data.get(name) {
        return from_json(child);
    }
    // FHIR choice elements: a miss on `value` may be stored as `valueQuantity`,
    // `valueDate`, ... - the suffix names the type
    for (key, child) in data {
        if let Some(suffix) = key.strip_prefix(name)
            && suffix
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_uppercase())
        {
            return tag_choice(from_json(child), suffix);
        }
    }
    vec![]
}

fn tag_choice(values: Vec<Value>, suffix: &str) -> Vec<Value> {
    values
        .into_iter()
        .map(|v| match v {
            Value::Complex { data, .. } => Value::Complex {
                data,
                ty: Some(suffix.to_string()),
            },
            Value::String(s) if suffix == "Date" => Value::Date(s),
            Value::String(s) if suffix == "DateTime" => Value::DateTime(s),
            other => other,
        })
        .collect()
}
