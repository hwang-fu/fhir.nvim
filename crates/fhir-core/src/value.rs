use rust_decimal::Decimal;

#[derive(Debug, Clone)]
pub enum Value {
    Boolean(bool),
    Integer(i64),
    Decimal(Decimal),
    String(String),
    Date(String),
    DateTime(String),
    Complex {
        data: serde_json::Map<String, serde_json::Value>,
        ty: Option<String>,
    },
}

pub fn from_json(v: &serde_json::Value) -> Vec<Value> {
    match v {
        serde_json::Value::Null => vec![],
        serde_json::Value::Array(items) => items.iter().flat_map(from_json).collect(),
        serde_json::Value::Bool(b) => vec![Value::Boolean(*b)],
        serde_json::Value::String(s) => vec![Value::String(s.clone())],
        serde_json::Value::Number(n) => match n.as_i64() {
            Some(i) => vec![Value::Integer(i)],
            // arbitrary_precision keeps the source text, so the decimal is exact
            None => match n.to_string().parse::<Decimal>() {
                Ok(d) => vec![Value::Decimal(d)],
                Err(_) => vec![],
            },
        },
        serde_json::Value::Object(map) => {
            let ty = map
                .get("resourceType")
                .and_then(|t| t.as_str())
                .map(String::from);
            vec![Value::Complex {
                data: map.clone(),
                ty,
            }]
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Boolean(a), Value::Boolean(b)) => a == b,
            (Value::Integer(a), Value::Integer(b)) => a == b,
            (Value::Decimal(a), Value::Decimal(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Date(a), Value::Date(b)) => a == b,
            (Value::DateTime(a), Value::DateTime(b)) => a == b,
            // integers and decimals compare numerically
            (Value::Integer(i), Value::Decimal(d)) | (Value::Decimal(d), Value::Integer(i)) => {
                Decimal::from(*i) == *d
            }
            // JSON has no date type, so date values equal strings with the same text
            (Value::Date(a), Value::String(b)) | (Value::String(b), Value::Date(a)) => a == b,
            (Value::DateTime(a), Value::String(b)) | (Value::String(b), Value::DateTime(a)) => {
                a == b
            }
            // deep json equality; the type tag does not affect identity
            (Value::Complex { data: a, .. }, Value::Complex { data: b, .. }) => a == b,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(json: &str) -> Value {
        let v: serde_json::Value = serde_json::from_str(json).unwrap();
        let mut items = from_json(&v);
        assert_eq!(items.len(), 1);
        items.remove(0)
    }

    #[test]
    fn primitives() {
        assert_eq!(one("true"), Value::Boolean(true));
        assert_eq!(one("3"), Value::Integer(3));
        assert_eq!(one("3.10"), Value::Decimal("3.10".parse().unwrap()));
        assert_eq!(one("\"hi\""), Value::String("hi".into()));
    }

    #[test]
    fn null_vanishes_and_arrays_flatten() {
        let v: serde_json::Value = serde_json::from_str("null").unwrap();
        assert!(from_json(&v).is_empty());
        let v: serde_json::Value = serde_json::from_str("[1, 2, 3]").unwrap();
        assert_eq!(from_json(&v).len(), 3);
    }

    #[test]
    fn objects_become_complex_with_resource_type_tag() {
        let v = one(r#"{"resourceType": "Patient", "id": "p"}"#);
        match v {
            Value::Complex { ty, .. } => assert_eq!(ty.as_deref(), Some("Patient")),
            other => panic!("expected Complex, got {other:?}"),
        }
        let v = one(r#"{"unit": "lbs"}"#);
        match v {
            Value::Complex { ty, .. } => assert_eq!(ty, None),
            other => panic!("expected Complex, got {other:?}"),
        }
    }

    #[test]
    fn equality_semantics() {
        // integer and decimal compare numerically
        assert_eq!(Value::Integer(1), Value::Decimal("1.0".parse().unwrap()));
        // exact decimal comparison, not f64
        assert_eq!(
            Value::Decimal("0.1".parse().unwrap()),
            Value::Decimal("0.10".parse().unwrap())
        );
        // a JSON string equals a date literal with the same text
        assert_eq!(
            Value::Date("1974-12-25".into()),
            Value::String("1974-12-25".into())
        );
        assert_ne!(Value::String("a".into()), Value::Integer(1));
        // complex values compare by deep json equality
        assert_eq!(
            one(r#"{"a": [1], "b": "x"}"#),
            one(r#"{"b": "x", "a": [1]}"#)
        );
    }
}
