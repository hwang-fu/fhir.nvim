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
    eval::eval(&expr, &input, &input)
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
}
