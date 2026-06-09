use crate::ast::Expr;
use crate::error::Error;
use crate::eval::{self, Ctx};
use crate::value::{Value, from_json};

pub fn call(
    name: &str,
    input: &[Value],
    args: &[Expr],
    focus: &[Value],
    ctx: &Ctx,
) -> Result<Vec<Value>, Error> {
    match (name, args) {
        ("exists", []) => ok_bool(!input.is_empty()),
        ("exists", [criteria]) => {
            for item in input {
                if criterion(criteria, item, ctx)? {
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
                if !criterion(criteria, item, ctx)? {
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
                if criterion(criteria, item, ctx)? {
                    out.push(item.clone());
                }
            }
            Ok(out)
        }
        ("select", [projection]) => {
            let mut out = Vec::new();
            for item in input {
                out.extend(eval::eval(projection, std::slice::from_ref(item), ctx)?);
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
        ("first", []) => Ok(input.first().cloned().into_iter().collect()),
        ("last", []) => Ok(input.last().cloned().into_iter().collect()),
        ("tail", []) => Ok(input.iter().skip(1).cloned().collect()),
        ("skip", [n]) => {
            let n = int_arg(n, focus, ctx)?;
            if n <= 0 {
                return Ok(input.to_vec());
            }
            Ok(input.iter().skip(n as usize).cloned().collect())
        }
        ("take", [n]) => {
            let n = int_arg(n, focus, ctx)?;
            if n <= 0 {
                return Ok(vec![]);
            }
            Ok(input.iter().take(n as usize).cloned().collect())
        }
        ("extension", [url_arg]) => {
            let url = string_arg(url_arg, focus, ctx)?;
            let mut out = Vec::new();
            for item in input {
                for ext in eval::access(item, "extension") {
                    if let Value::Complex { data, .. } = &ext
                        && data.get("url").and_then(|u| u.as_str()) == Some(url.as_str())
                    {
                        out.push(ext.clone());
                    }
                }
            }
            Ok(out)
        }
        ("resolve", []) => {
            let mut out = Vec::new();
            for item in input {
                let reference = match item {
                    Value::String(s) => Some(s.clone()),
                    Value::Complex { data, .. } => data
                        .get("reference")
                        .and_then(|r| r.as_str())
                        .map(String::from),
                    _ => None,
                };
                if let (Some(r), Some(resolver)) = (reference, ctx.resolver)
                    && let Some(resource) = resolver.resolve(&r)
                {
                    out.extend(from_json(&resource));
                }
            }
            Ok(out)
        }
        ("single", []) => match input {
            [] => Ok(vec![]),
            [v] => Ok(vec![v.clone()]),
            _ => Err(Error::Eval(
                "single() on a collection with more than one value".into(),
            )),
        },
        _ => Err(Error::Eval(format!("unknown function: {name}"))),
    }
}

// a plain argument is evaluated once, in the context the function is invoked from
fn int_arg(expr: &Expr, focus: &[Value], ctx: &Ctx) -> Result<i64, Error> {
    match eval::eval(expr, focus, ctx)?.as_slice() {
        [Value::Integer(n)] => Ok(*n),
        other => Err(Error::Eval(format!(
            "expected a single integer argument, got {other:?}"
        ))),
    }
}

fn string_arg(expr: &Expr, focus: &[Value], ctx: &Ctx) -> Result<String, Error> {
    match eval::eval(expr, focus, ctx)?.as_slice() {
        [Value::String(s)] => Ok(s.clone()),
        other => Err(Error::Eval(format!(
            "expected a single string argument, got {other:?}"
        ))),
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
fn criterion(expr: &Expr, item: &Value, ctx: &Ctx) -> Result<bool, Error> {
    let result = eval::eval(expr, std::slice::from_ref(item), ctx)?;
    Ok(eval::boolean_of(&result)?.unwrap_or(false))
}
