mod ast;
mod error;
mod eval;
mod functions;
mod lexer;
mod parser;
mod value;

pub use error::Error;
pub use value::Value;

pub fn evaluate(resource_json: &str, expression: &str) -> Result<Vec<Value>, Error> {
    let json: serde_json::Value =
        serde_json::from_str(resource_json).map_err(|e| Error::Eval(format!("bad json: {e}")))?;
    let expr = parser::parse(expression)?;
    let input = value::from_json(&json);
    eval::eval(&expr, &input)
}

pub fn ping() -> &'static str {
    "hello from fhir-core"
}

#[cfg(test)]
mod tests {
    use super::*;

    const PATIENT: &str = r#"{
      "resourceType": "Patient", "id": "p1", "active": true,
      "birthDate": "1974-12-25",
      "name": [
        { "use": "official", "family": "Chalmers", "given": ["Peter", "James"] },
        { "use": "usual", "given": ["Jim"] }
      ]
    }"#;

    const OBS: &str = r#"{
      "resourceType": "Observation", "id": "o1", "status": "final",
      "subject": { "reference": "Patient/p1" },
      "valueQuantity": { "value": 185, "unit": "lbs" }
    }"#;

    fn ev(json: &str, expr: &str) -> Vec<Value> {
        evaluate(json, expr).unwrap()
    }

    fn strings(vals: &[Value]) -> Vec<String> {
        vals.iter()
            .map(|v| match v {
                Value::String(s) => s.clone(),
                other => panic!("expected string, got {other:?}"),
            })
            .collect()
    }

    #[test]
    fn ping_returns_greeting() {
        assert_eq!(ping(), "hello from fhir-core");
    }

    #[test]
    fn paths_navigate_and_flatten() {
        assert_eq!(
            strings(&ev(PATIENT, "Patient.name.given")),
            ["Peter", "James", "Jim"]
        );
        assert_eq!(
            strings(&ev(PATIENT, "name.given")),
            ["Peter", "James", "Jim"]
        );
        assert_eq!(strings(&ev(PATIENT, "name.family")), ["Chalmers"]);
        assert!(ev(PATIENT, "name.nothing").is_empty());
        assert!(ev(PATIENT, "nothing.at.all").is_empty());
        // a wrong resource-type head is just a missing member
        assert!(ev(PATIENT, "Observation.status").is_empty());
    }

    #[test]
    fn indexing() {
        assert_eq!(strings(&ev(PATIENT, "name[0].family")), ["Chalmers"]);
        assert_eq!(strings(&ev(PATIENT, "name[1].given")), ["Jim"]);
        assert!(ev(PATIENT, "name[5]").is_empty());
    }

    #[test]
    fn literal_expressions() {
        assert_eq!(ev(PATIENT, "'hi'"), [Value::String("hi".into())]);
        assert_eq!(ev(PATIENT, "42"), [Value::Integer(42)]);
        assert_eq!(ev(PATIENT, "true"), [Value::Boolean(true)]);
    }

    #[test]
    fn this_is_the_input() {
        assert_eq!(strings(&ev(PATIENT, "$this.id")), ["p1"]);
    }

    #[test]
    fn choice_elements_resolve_with_type_tag() {
        assert_eq!(strings(&ev(OBS, "Observation.value.unit")), ["lbs"]);
        match &ev(OBS, "value")[0] {
            Value::Complex { ty, .. } => assert_eq!(ty.as_deref(), Some("Quantity")),
            other => panic!("expected Complex, got {other:?}"),
        }
    }

    fn ev1(json: &str, expr: &str) -> Value {
        let mut r = ev(json, expr);
        assert_eq!(r.len(), 1, "expected singleton from {expr}");
        r.remove(0)
    }

    #[test]
    fn equality_and_comparison() {
        assert_eq!(ev1(PATIENT, "active = true"), Value::Boolean(true));
        assert_eq!(
            ev1(PATIENT, "name[0].family = 'Chalmers'"),
            Value::Boolean(true)
        );
        assert_eq!(ev1(PATIENT, "name[0].family != 'X'"), Value::Boolean(true));
        assert_eq!(ev1(PATIENT, "1 < 2"), Value::Boolean(true));
        assert_eq!(ev1(PATIENT, "2 <= 2"), Value::Boolean(true));
        assert_eq!(ev1(PATIENT, "'abc' > 'abd'"), Value::Boolean(false));
        assert_eq!(
            ev1(PATIENT, "birthDate = @1974-12-25"),
            Value::Boolean(true)
        );
        // empty operand propagates to empty
        assert!(ev(PATIENT, "name[9].family = 'x'").is_empty());
        // non-singleton comparison is an error
        assert!(matches!(
            evaluate(PATIENT, "name.given < 'x'"),
            Err(Error::Eval(_))
        ));
    }

    #[test]
    fn logic_three_valued() {
        assert_eq!(ev1(PATIENT, "(1 = 1) and (2 = 2)"), Value::Boolean(true));
        assert_eq!(ev1(PATIENT, "(1 = 2) or (2 = 2)"), Value::Boolean(true));
        // true and empty -> empty; false and empty -> false
        assert!(ev(PATIENT, "(1 = 1) and (name[9].family = 'x')").is_empty());
        assert_eq!(
            ev1(PATIENT, "(1 = 2) and (name[9].family = 'x')"),
            Value::Boolean(false)
        );
        // true or empty -> true
        assert_eq!(
            ev1(PATIENT, "(1 = 1) or (name[9].family = 'x')"),
            Value::Boolean(true)
        );
    }

    #[test]
    fn membership_union_concat() {
        assert_eq!(ev1(PATIENT, "'Jim' in name.given"), Value::Boolean(true));
        assert_eq!(ev1(PATIENT, "'Bob' in name.given"), Value::Boolean(false));
        // union merges and dedupes
        assert_eq!(ev(PATIENT, "name.given | name.given").len(), 3);
        assert_eq!(ev1(PATIENT, "'a' & 'b'"), Value::String("ab".into()));
        // empty operand acts as ''
        assert_eq!(
            ev1(PATIENT, "name[9].family & 'x'"),
            Value::String("x".into())
        );
    }

    #[test]
    fn existence_functions() {
        assert_eq!(ev1(PATIENT, "name.exists()"), Value::Boolean(true));
        assert_eq!(ev1(PATIENT, "nothing.exists()"), Value::Boolean(false));
        assert_eq!(
            ev1(PATIENT, "name.exists(use = 'usual')"),
            Value::Boolean(true)
        );
        assert_eq!(
            ev1(PATIENT, "name.exists(use = 'x')"),
            Value::Boolean(false)
        );
        assert_eq!(ev1(PATIENT, "name.empty()"), Value::Boolean(false));
        assert_eq!(ev1(PATIENT, "nothing.empty()"), Value::Boolean(true));
        assert_eq!(ev1(PATIENT, "name.given.count()"), Value::Integer(3));
        assert_eq!(ev1(PATIENT, "nothing.count()"), Value::Integer(0));
        assert_eq!(
            ev1(PATIENT, "(name.given | name.given).distinct().count()"),
            Value::Integer(3)
        );
        assert_eq!(
            ev1(PATIENT, "name.all(given.exists())"),
            Value::Boolean(true)
        );
        assert_eq!(
            ev1(PATIENT, "name.all(use = 'official')"),
            Value::Boolean(false)
        );
        // all() on empty input is vacuously true
        assert_eq!(ev1(PATIENT, "nothing.all(use = 'x')"), Value::Boolean(true));
        assert_eq!(ev1(PATIENT, "(1 = 2).not()"), Value::Boolean(true));
        assert_eq!(ev1(PATIENT, "nothing.exists().not()"), Value::Boolean(true));
        assert!(matches!(
            evaluate(PATIENT, "name.unknownFn()"),
            Err(Error::Eval(_))
        ));
    }

    #[test]
    fn filtering_and_projection() {
        assert_eq!(
            strings(&ev(PATIENT, "name.where(use = 'official').family")),
            ["Chalmers"]
        );
        assert!(ev(PATIENT, "name.where(use = 'nope')").is_empty());
        assert_eq!(
            ev1(PATIENT, "name.where(given.exists()).count()"),
            Value::Integer(2)
        );
        assert_eq!(
            strings(&ev(PATIENT, "name.select(given)")),
            ["Peter", "James", "Jim"]
        );
        assert_eq!(
            ev1(PATIENT, "name.given.select($this).count()"),
            Value::Integer(3)
        );
    }

    #[test]
    fn of_type_filters_by_type() {
        assert_eq!(
            strings(&ev(OBS, "Observation.value.ofType(Quantity).unit")),
            ["lbs"]
        );
        assert!(ev(OBS, "Observation.value.ofType(string)").is_empty());
        assert_eq!(
            ev1(PATIENT, "name.given.ofType(string).count()"),
            Value::Integer(3)
        );
        assert_eq!(
            ev1(PATIENT, "name.given.ofType(System.String).count()"),
            Value::Integer(3)
        );
        assert_eq!(
            ev1(PATIENT, "name.given.ofType(integer).count()"),
            Value::Integer(0)
        );
    }

    #[test]
    fn subsetting() {
        assert_eq!(
            ev1(PATIENT, "name.given.first()"),
            Value::String("Peter".into())
        );
        assert_eq!(
            ev1(PATIENT, "name.given.last()"),
            Value::String("Jim".into())
        );
        assert_eq!(ev1(PATIENT, "name.given.tail().count()"), Value::Integer(2));
        assert_eq!(
            ev1(PATIENT, "name.given.skip(1).first()"),
            Value::String("James".into())
        );
        assert_eq!(
            ev1(PATIENT, "name.given.take(2).count()"),
            Value::Integer(2)
        );
        assert_eq!(ev1(PATIENT, "id.single()"), Value::String("p1".into()));
        assert!(ev(PATIENT, "nothing.first()").is_empty());
        assert!(ev(PATIENT, "nothing.single()").is_empty());
        assert!(matches!(
            evaluate(PATIENT, "name.given.single()"),
            Err(Error::Eval(_))
        ));
    }

    #[test]
    fn is_and_as() {
        assert_eq!(
            ev1(OBS, "Observation.value is Quantity"),
            Value::Boolean(true)
        );
        assert_eq!(
            ev1(OBS, "Observation.value is string"),
            Value::Boolean(false)
        );
        assert_eq!(
            strings(&ev(OBS, "(Observation.value as Quantity).unit")),
            ["lbs"]
        );
        assert!(ev(OBS, "Observation.value as string").is_empty());
        assert_eq!(ev1(PATIENT, "birthDate is date"), Value::Boolean(true));
        // is on empty -> empty
        assert!(ev(PATIENT, "nothing is string").is_empty());
    }
}
