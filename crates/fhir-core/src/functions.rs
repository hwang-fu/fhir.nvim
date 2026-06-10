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
        ("length", []) => map_str(input, |s| Value::Integer(s.chars().count() as i64)),
        ("upper", []) => map_str(input, |s| Value::String(s.to_uppercase())),
        ("lower", []) => map_str(input, |s| Value::String(s.to_lowercase())),
        ("trim", []) => map_str(input, |s| Value::String(s.trim().to_string())),
        ("startsWith", [p]) => {
            let p = string_arg(p, focus, ctx)?;
            map_str(input, |s| Value::Boolean(s.starts_with(&p)))
        }
        ("endsWith", [p]) => {
            let p = string_arg(p, focus, ctx)?;
            map_str(input, |s| Value::Boolean(s.ends_with(&p)))
        }
        ("contains", [p]) => {
            let p = string_arg(p, focus, ctx)?;
            map_str(input, |s| Value::Boolean(s.contains(&p)))
        }
        ("indexOf", [p]) => {
            let p = string_arg(p, focus, ctx)?;
            map_str(input, |s| match s.find(&p) {
                // char position, not byte position
                Some(byte) => Value::Integer(s[..byte].chars().count() as i64),
                None => Value::Integer(-1),
            })
        }
        ("replace", [from, to]) => {
            let from = string_arg(from, focus, ctx)?;
            let to = string_arg(to, focus, ctx)?;
            map_str(input, |s| Value::String(s.replace(&from, &to)))
        }
        ("substring", [start]) | ("substring", [start, _]) => {
            let begin = int_arg(start, focus, ctx)?;
            let length = match args {
                [_, l] => Some(int_arg(l, focus, ctx)?),
                _ => None,
            };
            let Some(s) = string_input(input)? else {
                return Ok(vec![]);
            };
            let chars: Vec<char> = s.chars().collect();
            if begin < 0 || begin as usize >= chars.len() {
                return Ok(vec![]);
            }
            let begin = begin as usize;
            let end = match length {
                Some(l) => (begin + l.max(0) as usize).min(chars.len()),
                None => chars.len(),
            };
            Ok(vec![Value::String(chars[begin..end].iter().collect())])
        }
        ("split", [sep]) => {
            let sep = string_arg(sep, focus, ctx)?;
            Ok(match string_input(input)? {
                None => vec![],
                Some(s) => s.split(&sep).map(|p| Value::String(p.to_string())).collect(),
            })
        }
        ("toChars", []) => Ok(match string_input(input)? {
            None => vec![],
            Some(s) => s.chars().map(|c| Value::String(c.to_string())).collect(),
        }),
        ("join", [sep]) => {
            let sep = string_arg(sep, focus, ctx)?;
            let mut parts = Vec::new();
            for v in input {
                match v {
                    Value::String(s) | Value::Date(s) | Value::DateTime(s) => {
                        parts.push(s.as_str())
                    }
                    other => {
                        return Err(Error::Eval(format!("join expects strings, got {other:?}")));
                    }
                }
            }
            Ok(vec![Value::String(parts.join(&sep))])
        }
        ("matches", [p]) => {
            let re = regex_arg(p, focus, ctx)?;
            map_str(input, |s| Value::Boolean(re.is_match(s)))
        }
        ("replaceMatches", [p, to]) => {
            let re = regex_arg(p, focus, ctx)?;
            let to = string_arg(to, focus, ctx)?;
            map_str(input, |s| Value::String(re.replace_all(s, to.as_str()).into_owned()))
        }
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

// singleton string-ish input (dates pass as their text); empty stays empty
fn string_input(input: &[Value]) -> Result<Option<&str>, Error> {
    match eval::singleton(input)? {
        None => Ok(None),
        Some(Value::String(s)) | Some(Value::Date(s)) | Some(Value::DateTime(s)) => {
            Ok(Some(s.as_str()))
        }
        Some(other) => Err(Error::Eval(format!("expected a string, got {other:?}"))),
    }
}

fn map_str(input: &[Value], f: impl Fn(&str) -> Value) -> Result<Vec<Value>, Error> {
    Ok(match string_input(input)? {
        None => vec![],
        Some(s) => vec![f(s)],
    })
}

fn regex_arg(expr: &Expr, focus: &[Value], ctx: &Ctx) -> Result<regex_lite::Regex, Error> {
    let pattern = string_arg(expr, focus, ctx)?;
    regex_lite::Regex::new(&pattern).map_err(|e| Error::Eval(format!("bad regex: {e}")))
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
