//! Tree-walking evaluator for parsed FHIRPath expressions. It only navigates
//! JSON-derived values; nothing is compiled or executed.

use crate::ast::{Expr, Literal};
use crate::error::Error;
use crate::value::{Value, from_json};

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
        Expr::Call { .. } | Expr::Binary { .. } | Expr::TypeTest { .. } => {
            Err(Error::Eval("not implemented".into()))
        }
    }
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
