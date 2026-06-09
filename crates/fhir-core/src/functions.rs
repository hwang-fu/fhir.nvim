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
        ("where", [criteria]) => {
            let mut out = Vec::new();
            for item in input {
                if criterion(criteria, item)? {
                    out.push(item.clone());
                }
            }
            Ok(out)
        }
        ("select", [projection]) => {
            let mut out = Vec::new();
            for item in input {
                out.extend(eval::eval(projection, std::slice::from_ref(item))?);
            }
            Ok(out)
        }
        ("ofType", [type_arg]) => {
            let ty = type_name_of(type_arg)
                .ok_or_else(|| Error::Eval("ofType expects a type name".into()))?;
            Ok(input
                .iter()
                .filter(|v| crate::value::matches_type(v, &ty))
                .cloned()
                .collect())
        }
        _ => Err(Error::Eval(format!("unknown function: {name}"))),
    }
}

// a type argument is a name, not an expression to evaluate: `string`,
// `Quantity`, or qualified `System.String` (parsed as member access)
fn type_name_of(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Identifier(s) => Some(s.clone()),
        Expr::Member { base, name } => match base.as_ref() {
            Expr::Identifier(b) => Some(format!("{b}.{name}")),
            _ => None,
        },
        _ => None,
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
