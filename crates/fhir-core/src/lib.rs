mod ast;
mod error;
mod eval;
mod functions;
mod lexer;
mod parser;
mod schema;
mod temporal;
mod validate;
mod value;

pub use error::Error;
pub use schema::Severity;
pub use validate::{Category, Issue, validate};
pub use value::Value;

pub fn evaluate(resource_json: &str, expression: &str) -> Result<Vec<Value>, Error> {
    Engine::new().evaluate(resource_json, expression)
}

/// Like [`evaluate`], but renders the result collection as a compact JSON
/// array string - the form foreign callers (e.g. the editor binding) consume.
pub fn evaluate_json(resource_json: &str, expression: &str) -> Result<String, Error> {
    Engine::new().evaluate_json(resource_json, expression)
}

/// Like [`validate`], but rendered as a compact JSON array string - the form
/// foreign callers (e.g. the editor binding) consume.
pub fn validate_json(resource_json: &str) -> Result<String, Error> {
    Engine::new().validate_json(resource_json)
}

/// Resolves a FHIR reference (e.g. "Patient/p1") to a resource. How references
/// are looked up is the caller's concern; the engine only asks.
pub trait Resolve {
    fn resolve(&self, reference: &str) -> Option<serde_json::Value>;
}

#[derive(Default)]
pub struct Engine {
    resolver: Option<Box<dyn Resolve>>,
}

impl Engine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_resolver(mut self, resolver: Box<dyn Resolve>) -> Self {
        self.resolver = Some(resolver);
        self
    }

    pub fn evaluate(&self, resource_json: &str, expression: &str) -> Result<Vec<Value>, Error> {
        let json: serde_json::Value = serde_json::from_str(resource_json)
            .map_err(|e| Error::Eval(format!("bad json: {e}")))?;
        let expr = parser::parse(expression)?;
        let input = value::from_json(&json);
        let ctx = eval::Ctx {
            resolver: self.resolver.as_deref(),
        };
        eval::eval(&expr, &input, &ctx)
    }

    /// Like [`Engine::evaluate`], rendered as a compact JSON array string.
    pub fn evaluate_json(&self, resource_json: &str, expression: &str) -> Result<String, Error> {
        let values = self.evaluate(resource_json, expression)?;
        let rendered: Vec<serde_json::Value> = values.iter().map(value::to_json).collect();
        serde_json::to_string(&rendered).map_err(|e| Error::Eval(format!("render: {e}")))
    }

    /// Like [`validate`], but constraint expressions that resolve references
    /// do so through the engine's resolver.
    pub fn validate(&self, resource_json: &str) -> Result<Vec<Issue>, Error> {
        validate::run(resource_json, self.resolver.as_deref())
    }

    /// Like [`Engine::validate`], rendered as a compact JSON array string.
    pub fn validate_json(&self, resource_json: &str) -> Result<String, Error> {
        let issues = self.validate(resource_json)?;
        let rendered: Vec<serde_json::Value> = issues.iter().map(Issue::to_json).collect();
        serde_json::to_string(&rendered).map_err(|e| Error::Validate(format!("render: {e}")))
    }
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

    const EXT_PATIENT: &str = r#"{
      "resourceType": "Patient", "id": "p2",
      "extension": [
        { "url": "http://example.org/bday", "valueDate": "1970-01-01" },
        { "url": "http://example.org/other", "valueString": "x" }
      ]
    }"#;

    #[test]
    fn extension_by_url() {
        assert_eq!(
            ev1(
                EXT_PATIENT,
                "extension('http://example.org/bday').value = @1970-01-01"
            ),
            Value::Boolean(true)
        );
        assert_eq!(
            ev1(EXT_PATIENT, "extension('http://nope').count()"),
            Value::Integer(0)
        );
    }

    struct FakeResolver;

    impl Resolve for FakeResolver {
        fn resolve(&self, reference: &str) -> Option<serde_json::Value> {
            (reference == "Patient/p1").then(|| serde_json::from_str(PATIENT).unwrap())
        }
    }

    #[test]
    fn resolve_uses_the_engine_hook() {
        // default engine: resolve() yields empty
        assert!(ev(OBS, "subject.resolve()").is_empty());
        let engine = Engine::new().with_resolver(Box::new(FakeResolver));
        let got = engine.evaluate(OBS, "subject.resolve().id").unwrap();
        assert_eq!(got, [Value::String("p1".into())]);
    }

    #[test]
    fn evaluate_json_renders_a_json_array() {
        assert_eq!(
            evaluate_json(PATIENT, "name.given").unwrap(),
            r#"["Peter","James","Jim"]"#
        );
        assert_eq!(evaluate_json(PATIENT, "nothing").unwrap(), "[]");
        assert_eq!(evaluate_json(PATIENT, "active").unwrap(), "[true]");
        assert_eq!(evaluate_json(PATIENT, "name.given.count()").unwrap(), "[3]");
        // exact decimals survive the round trip
        assert_eq!(evaluate_json(r#"{"a": 0.10}"#, "a").unwrap(), "[0.10]");
        // complex values render as their json objects
        assert_eq!(evaluate_json(OBS, "value.unit").unwrap(), r#"["lbs"]"#);
        assert!(evaluate_json(PATIENT, "1 +").is_err());
        assert!(evaluate_json("not json", "name").is_err());
    }

    #[test]
    fn validate_json_renders_an_issue_array() {
        let out = validate_json(r#"{"resourceType":"Patient","favouriteColor":"blue"}"#).unwrap();
        let issues: serde_json::Value = serde_json::from_str(&out).unwrap();
        let issues = issues.as_array().unwrap();
        // instance invariants emit first, so look the structural finding up
        let unknown = issues.iter().find(|i| i["category"] == "unknown").unwrap();
        assert_eq!(unknown["path"], "Patient.favouriteColor");
        assert_eq!(unknown["severity"], "error");
        assert!(
            unknown["message"]
                .as_str()
                .unwrap()
                .contains("favouriteColor")
        );
        // the array also carries the advisory findings (dom-6 here)
        assert!(out.contains(r#""severity":"warning""#));
        assert!(validate_json("{").is_err());
    }

    #[test]
    fn engine_evaluate_json_uses_the_resolver() {
        let engine = Engine::new().with_resolver(Box::new(FakeResolver));
        assert_eq!(
            engine.evaluate_json(OBS, "subject.resolve().id").unwrap(),
            r#"["p1"]"#
        );
    }

    #[test]
    fn arithmetic_operators() {
        assert_eq!(ev1(PATIENT, "2 + 2"), Value::Integer(4));
        assert_eq!(
            ev1(PATIENT, "2 + 2.5"),
            Value::Decimal("4.5".parse().unwrap())
        );
        assert_eq!(ev1(PATIENT, "5 - 7"), Value::Integer(-2));
        assert_eq!(ev1(PATIENT, "3 * 3"), Value::Integer(9));
        // / always yields a decimal; div is integer division
        assert_eq!(
            ev1(PATIENT, "5 / 2"),
            Value::Decimal("2.5".parse().unwrap())
        );
        assert_eq!(ev1(PATIENT, "5 div 2"), Value::Integer(2));
        assert_eq!(ev1(PATIENT, "5 mod 2"), Value::Integer(1));
        // division by zero is empty, not an error
        assert!(ev(PATIENT, "5 / 0").is_empty());
        assert!(ev(PATIENT, "5 div 0").is_empty());
        // + concatenates strings, empty propagates (unlike &)
        assert_eq!(ev1(PATIENT, "'a' + 'b'"), Value::String("ab".into()));
        assert!(ev(PATIENT, "name[9].family + 'x'").is_empty());
        // unary minus on an expression
        assert_eq!(ev1(PATIENT, "-(2 + 3)"), Value::Integer(-5));
    }

    #[test]
    fn logic_and_equivalence_operators() {
        assert_eq!(ev1(PATIENT, "true xor false"), Value::Boolean(true));
        assert_eq!(ev1(PATIENT, "true xor true"), Value::Boolean(false));
        assert_eq!(ev1(PATIENT, "false implies false"), Value::Boolean(true));
        assert_eq!(ev1(PATIENT, "true implies false"), Value::Boolean(false));
        // false implies anything (even empty) is true
        assert_eq!(
            ev1(PATIENT, "(1 = 2) implies (name[9].family = 'x')"),
            Value::Boolean(true)
        );
        // equivalence: empty ~ empty is true; = would be empty
        assert_eq!(ev1(PATIENT, "nothing ~ alsonothing"), Value::Boolean(true));
        assert_eq!(ev1(PATIENT, "1 ~ 1"), Value::Boolean(true));
        assert_eq!(ev1(PATIENT, "'ABC' ~ 'abc'"), Value::Boolean(true));
        assert_eq!(ev1(PATIENT, "'a' !~ 'b'"), Value::Boolean(true));
        // contains is in, reversed
        assert_eq!(
            ev1(PATIENT, "name.given contains 'Jim'"),
            Value::Boolean(true)
        );
    }

    #[test]
    fn string_functions() {
        assert_eq!(ev1(PATIENT, "'hello'.length()"), Value::Integer(5));
        assert_eq!(
            ev1(PATIENT, "'hello'.upper()"),
            Value::String("HELLO".into())
        );
        assert_eq!(
            ev1(PATIENT, "'HELLO'.lower()"),
            Value::String("hello".into())
        );
        assert_eq!(
            ev1(PATIENT, "'hello'.startsWith('he')"),
            Value::Boolean(true)
        );
        assert_eq!(ev1(PATIENT, "'hello'.endsWith('lo')"), Value::Boolean(true));
        assert_eq!(
            ev1(PATIENT, "'hello'.contains('ell')"),
            Value::Boolean(true)
        );
        assert_eq!(
            ev1(PATIENT, "'hello'.substring(1)"),
            Value::String("ello".into())
        );
        assert_eq!(
            ev1(PATIENT, "'hello'.substring(1, 3)"),
            Value::String("ell".into())
        );
        assert!(ev(PATIENT, "'hello'.substring(9)").is_empty());
        assert_eq!(ev1(PATIENT, "'hello'.indexOf('ll')"), Value::Integer(2));
        assert_eq!(ev1(PATIENT, "'hello'.indexOf('x')"), Value::Integer(-1));
        assert_eq!(
            ev1(PATIENT, "'hello'.replace('l', 'L')"),
            Value::String("heLLo".into())
        );
        assert_eq!(ev1(PATIENT, "'  hi  '.trim()"), Value::String("hi".into()));
        assert_eq!(
            ev1(PATIENT, "'a,b,c'.split(',').count()"),
            Value::Integer(3)
        );
        assert_eq!(
            ev1(PATIENT, "name.given.join('-')"),
            Value::String("Peter-James-Jim".into())
        );
        assert_eq!(ev1(PATIENT, "'abc'.toChars().count()"), Value::Integer(3));
        assert_eq!(
            ev1(PATIENT, "'hello'.matches('^h.*o$')"),
            Value::Boolean(true)
        );
        assert_eq!(
            ev1(PATIENT, "'hello'.replaceMatches('l+', 'L')"),
            Value::String("heLo".into())
        );
        // empty input propagates; bad regex errors
        assert!(ev(PATIENT, "nothing.upper()").is_empty());
        assert!(matches!(
            evaluate(PATIENT, "'x'.matches('(')"),
            Err(Error::Eval(_))
        ));
    }

    #[test]
    fn math_functions() {
        assert_eq!(ev1(PATIENT, "(-5).abs()"), Value::Integer(5));
        assert_eq!(
            ev1(PATIENT, "(-5.5).abs()"),
            Value::Decimal("5.5".parse().unwrap())
        );
        assert_eq!(ev1(PATIENT, "(2.4).ceiling()"), Value::Integer(3));
        assert_eq!(ev1(PATIENT, "(2.6).floor()"), Value::Integer(2));
        // round() is half away from zero, not banker's
        assert_eq!(ev1(PATIENT, "(2.5).round()"), Value::Integer(3));
        assert_eq!(
            ev1(PATIENT, "(2.345).round(2)"),
            Value::Decimal("2.35".parse().unwrap())
        );
        assert_eq!(ev1(PATIENT, "(2.7).truncate()"), Value::Integer(2));
        assert_eq!(
            ev1(PATIENT, "(9).sqrt()"),
            Value::Decimal("3".parse().unwrap())
        );
        assert!(ev(PATIENT, "(-1).sqrt()").is_empty());
        assert_eq!(ev1(PATIENT, "(2).power(10)"), Value::Integer(1024));
        assert!(ev(PATIENT, "(0).ln()").is_empty());
        assert!(ev(PATIENT, "nothing.abs()").is_empty());
    }

    #[test]
    fn quantity_values() {
        assert_eq!(ev1(PATIENT, "1 year = 1 year"), Value::Boolean(true));
        assert_eq!(ev1(PATIENT, "1 'mg' = 2 'mg'"), Value::Boolean(false));
        assert_eq!(
            evaluate_json(PATIENT, "1 year").unwrap(),
            r#"[{"unit":"year","value":1}]"#
        );
        assert_eq!(ev1(PATIENT, "(1 year) is Quantity"), Value::Boolean(true));
        assert_eq!(
            ev1(PATIENT, "(4 days).toString()"),
            Value::String("4 'day'".into())
        );
    }

    #[test]
    fn temporal_and_quantity_arithmetic() {
        assert_eq!(ev1(PATIENT, "@2014 + 1 year"), Value::Date("2015".into()));
        assert_eq!(
            ev1(PATIENT, "@2014-01-31 + 1 month"),
            Value::Date("2014-02-28".into())
        );
        assert_eq!(
            ev1(PATIENT, "@2015-02-04T23:30:00Z + 1 hour"),
            Value::DateTime("2015-02-05T00:30:00Z".into())
        );
        assert_eq!(ev1(PATIENT, "@2015 - 1 year"), Value::Date("2014".into()));
        // unit finer than precision -> empty
        assert!(ev(PATIENT, "@2014 + 1 hour").is_empty());
        // same-unit quantity arithmetic; mismatches are empty
        assert_eq!(
            ev1(PATIENT, "(1 'mg' + 2 'mg') = 3 'mg'"),
            Value::Boolean(true)
        );
        assert!(ev(PATIENT, "1 'mg' + 1 'kg'").is_empty());
        assert_eq!(ev1(PATIENT, "(2 'mg' * 3) = 6 'mg'"), Value::Boolean(true));
        assert_eq!(ev1(PATIENT, "1 'mg' < 2 'mg'"), Value::Boolean(true));
        assert!(ev(PATIENT, "1 'mg' < 1 'kg'").is_empty());
        // mismatched units are unknown for equality too
        assert!(ev(PATIENT, "1 'mg' = 1 'kg'").is_empty());
        // quantities from FHIR data compare with literals
        assert_eq!(
            ev1(OBS, "Observation.value > 100 'lbs'"),
            Value::Boolean(true)
        );
        assert_eq!(
            ev1(OBS, "Observation.value = 185 'lbs'"),
            Value::Boolean(true)
        );
    }

    #[test]
    fn conversion_functions() {
        assert_eq!(ev1(PATIENT, "(42).toString()"), Value::String("42".into()));
        assert_eq!(
            ev1(PATIENT, "true.toString()"),
            Value::String("true".into())
        );
        assert_eq!(ev1(PATIENT, "'42'.toInteger()"), Value::Integer(42));
        assert!(ev(PATIENT, "'x'.toInteger()").is_empty());
        assert_eq!(
            ev1(PATIENT, "'3.14'.toDecimal()"),
            Value::Decimal("3.14".parse().unwrap())
        );
        assert_eq!(ev1(PATIENT, "'true'.toBoolean()"), Value::Boolean(true));
        assert_eq!(ev1(PATIENT, "(1).toBoolean()"), Value::Boolean(true));
        assert_eq!(
            ev1(PATIENT, "'42'.convertsToInteger()"),
            Value::Boolean(true)
        );
        assert_eq!(
            ev1(PATIENT, "'x'.convertsToInteger()"),
            Value::Boolean(false)
        );
        assert_eq!(
            ev1(PATIENT, "'2015-02-04'.convertsToDate()"),
            Value::Boolean(true)
        );
        assert_eq!(ev1(PATIENT, "'x'.convertsToDate()"), Value::Boolean(false));
        assert_eq!(ev1(PATIENT, "(1).convertsToString()"), Value::Boolean(true));
        assert_eq!(
            ev1(PATIENT, "(1.5).convertsToInteger()"),
            Value::Boolean(false)
        );
    }

    #[test]
    fn today_and_now() {
        assert_eq!(ev1(PATIENT, "today() = today()"), Value::Boolean(true));
        assert!(matches!(ev1(PATIENT, "today()"), Value::Date(_)));
        assert!(matches!(ev1(PATIENT, "now()"), Value::DateTime(_)));
        assert_eq!(ev1(PATIENT, "birthDate <= today()"), Value::Boolean(true));
        // the clock plays with the calendar module
        assert_eq!(
            ev1(PATIENT, "today() < today() + 1 day"),
            Value::Boolean(true)
        );
    }

    #[test]
    fn iif_and_tree_functions() {
        assert_eq!(
            ev1(PATIENT, "iif(active, 'yes', 'no')"),
            Value::String("yes".into())
        );
        assert!(ev(PATIENT, "iif(1 = 2, 'yes')").is_empty());
        // lazy: the untaken branch may be nonsense and nothing errors
        assert_eq!(
            ev1(PATIENT, "iif(true, 'ok', 1.single().substring(9))"),
            Value::String("ok".into())
        );
        // children: every direct child value; descendants: transitive
        let kids = ev1(PATIENT, "children().count()");
        assert!(matches!(kids, Value::Integer(n) if n > 0));
        let all = ev1(PATIENT, "descendants().count()");
        assert!(matches!((kids, all), (Value::Integer(k), Value::Integer(d)) if d > k));
        assert_eq!(
            ev1(PATIENT, "repeat(name).count()"),
            ev1(PATIENT, "name.count()")
        );
    }
}
