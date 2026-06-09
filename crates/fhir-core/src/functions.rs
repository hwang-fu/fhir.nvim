use crate::ast::Expr;
use crate::error::Error;
use crate::eval;
use crate::value::Value;

pub fn call(name: &str, input: &[Value], args: &[Expr]) -> Result<Vec<Value>, Error> {
    match (name, args) {
        ("exists", []) => ok_bool(!input.is_empty()),
        ("exists", [criteria]) => {
            for item in input {
                if criterion(criteria, item)? {
                    return ok_bool(true);
                }
            }
            ok_bool(false)
        }
        ("empty", []) => ok_bool(input.is_empty()),
        ("count", []) => Ok(vec![Value::Integer(input.len() as i64)]),
        ("distinct", []) => {
            let mut out: Vec<Value> = Vec::new();
            for v in input {
                if !out.contains(v) {
                    out.push(v.clone());
                }
            }
            Ok(out)
        }
        // all() on empty input is vacuously true
        ("all", [criteria]) => {
            for item in input {
                if !criterion(criteria, item)? {
                    return ok_bool(false);
                }
            }
            ok_bool(true)
        }
        ("not", []) => match eval::boolean_of(input)? {
            Some(b) => ok_bool(!b),
            None => Ok(vec![]),
        },
        _ => Err(Error::Eval(format!("unknown function: {name}"))),
    }
}

fn ok_bool(b: bool) -> Result<Vec<Value>, Error> {
    Ok(vec![Value::Boolean(b)])
}

// a criteria argument is evaluated per item, with that item as the focus;
// an empty or false result both mean the criteria does not hold
fn criterion(expr: &Expr, item: &Value) -> Result<bool, Error> {
    let result = eval::eval(expr, std::slice::from_ref(item))?;
    Ok(eval::boolean_of(&result)?.unwrap_or(false))
}
