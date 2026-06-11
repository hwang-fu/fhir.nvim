//! Validates resources against the schema tables: unknown elements,
//! cardinality, value types, primitive formats, and choice element use.

use crate::error::Error;
use crate::eval::{self, Ctx};
use crate::schema::{self, Element, Severity, TypeDef};
use crate::{Resolve, parser, value};
use regex_lite::Regex;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::OnceLock;

#[derive(Debug, Clone, PartialEq)]
pub struct Issue {
    pub path: String,
    pub severity: Severity,
    pub category: Category,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Category {
    Unknown,
    Cardinality,
    Type,
    Format,
    Choice,
    Invariant,
}

impl Issue {
    // the wire form for foreign callers: four flat fields, lowercase names
    pub(crate) fn to_json(&self) -> serde_json::Value {
        let severity = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Information => "information",
        };
        let category = match self.category {
            Category::Unknown => "unknown",
            Category::Cardinality => "cardinality",
            Category::Type => "type",
            Category::Format => "format",
            Category::Choice => "choice",
            Category::Invariant => "invariant",
        };
        serde_json::json!({
            "path": self.path,
            "severity": severity,
            "category": category,
            "message": self.message,
        })
    }
}

fn issue(path: String, category: Category, message: String) -> Issue {
    Issue {
        path,
        severity: Severity::Error,
        category,
        message,
    }
}

/// Validates a FHIR resource; Err means there was nothing to validate.
pub fn validate(json: &str) -> Result<Vec<Issue>, Error> {
    run(json, None)
}

pub(crate) fn run(json: &str, resolver: Option<&dyn Resolve>) -> Result<Vec<Issue>, Error> {
    let root: Value =
        serde_json::from_str(json).map_err(|e| Error::Validate(format!("bad json: {e}")))?;
    let obj = root
        .as_object()
        .ok_or_else(|| Error::Validate("not a json object".into()))?;
    let rt = obj
        .get("resourceType")
        .and_then(Value::as_str)
        .ok_or_else(|| Error::Validate("resourceType is missing".into()))?;
    let ctx = Ctx { resolver };
    let mut issues = Vec::new();
    walk_typed(rt, &root, rt, &ctx, &mut issues);
    Ok(issues)
}

fn walk_typed(type_name: &str, focus: &Value, path: &str, ctx: &Ctx, issues: &mut Vec<Issue>) {
    match schema::type_def(type_name) {
        Some(ty) => walk_object(ty, "", focus, path, ctx, issues),
        None => issues.push(issue(
            path.to_string(),
            Category::Unknown,
            format!("unknown resource type {type_name:?}"),
        )),
    }
}

// constraints attach to the element they were declared on: the type root
// (prefix ""), a backbone, or a contentReference target - evaluate them
// with that instance as the focus
fn check_invariants(
    ty: &TypeDef,
    prefix: &str,
    focus: &Value,
    path: &str,
    ctx: &Ctx,
    issues: &mut Vec<Issue>,
) {
    let mut input = None; // converted on the first matching constraint
    for c in ty.constraints.iter().filter(|c| c.path == prefix) {
        let Some(expr) = parsed(c.expression) else {
            continue;
        };
        let input = input.get_or_insert_with(|| value::from_json(focus));
        let Ok(result) = eval::eval(expr, input, ctx) else {
            continue;
        };
        // only a positive false fails: an engine gap must not invent issues
        if result == [crate::Value::Boolean(false)] {
            issues.push(Issue {
                path: path.to_string(),
                severity: c.severity,
                category: Category::Invariant,
                message: format!("{}: {}", c.key, c.human),
            });
        }
    }
}

fn walk_object(
    ty: &'static TypeDef,
    prefix: &str,
    focus: &Value,
    path: &str,
    ctx: &Ctx,
    issues: &mut Vec<Issue>,
) {
    let Some(obj) = focus.as_object() else {
        return; // callers report non-objects
    };
    check_invariants(ty, prefix, focus, path, ctx, issues);
    let mut present: HashSet<&'static str> = HashSet::new();
    let mut variants: BTreeMap<&'static str, Vec<&str>> = BTreeMap::new();
    for (key, value) in obj {
        if prefix.is_empty() && key == "resourceType" {
            continue;
        }
        // a primitive's companion (_birthDate) carries its id/extensions and
        // may stand alone; resolve it against the base element, look no deeper
        if let Some(base) = key.strip_prefix('_') {
            let base_rel = join(prefix, base);
            let el = ty
                .element(&base_rel)
                .or_else(|| resolve_choice(ty, prefix, base).map(|(el, _)| el));
            match el {
                Some(el) => {
                    present.insert(el.path);
                }
                None => issues.push(issue(
                    format!("{path}.{key}"),
                    Category::Unknown,
                    format!("{key:?} is not an element of {}", scope(ty, prefix)),
                )),
            }
            continue;
        }
        let rel = join(prefix, key);
        let child_path = format!("{path}.{key}");
        if let Some(el) = ty.element(&rel) {
            present.insert(el.path);
            if el.choice {
                // the bare name never appears in json; the type is in the key
                issues.push(issue(
                    child_path,
                    Category::Choice,
                    format!(
                        "{key:?} takes a typed form (e.g. \"{key}{}\")",
                        capitalized(el.types.first().unwrap_or(&""))
                    ),
                ));
                continue;
            }
            check_shape(el, value, &child_path, issues);
            if let Some(t) = el.types.first() {
                walk_value(ty, &rel, t, value, &child_path, ctx, issues);
            }
        } else if let Some((el, variant)) = resolve_choice(ty, prefix, key) {
            present.insert(el.path);
            variants.entry(el.path).or_default().push(key);
            check_shape(el, value, &child_path, issues);
            walk_value(ty, el.path, variant, value, &child_path, ctx, issues);
        } else {
            issues.push(issue(
                child_path,
                Category::Unknown,
                format!("{key:?} is not an element of {}", scope(ty, prefix)),
            ));
        }
    }
    for (el_path, keys) in &variants {
        if keys.len() > 1 {
            let seg = last_segment(el_path);
            issues.push(issue(
                format!("{path}.{seg}"),
                Category::Choice,
                format!(
                    "only one form of {seg:?} is allowed (found {})",
                    keys.join(", ")
                ),
            ));
        }
    }
    for child in ty.children_of(prefix) {
        if child.min > 0 && !present.contains(child.path) {
            let seg = last_segment(child.path);
            issues.push(issue(
                format!("{path}.{seg}"),
                Category::Cardinality,
                format!("required element {seg:?} is missing"),
            ));
        }
    }
}

fn check_shape(el: &Element, value: &Value, path: &str, issues: &mut Vec<Issue>) {
    match value.as_array() {
        Some(items) => {
            if !el.max_many {
                issues.push(issue(
                    path.to_string(),
                    Category::Cardinality,
                    "did not expect an array (the element does not repeat)".into(),
                ));
            } else if items.is_empty() {
                issues.push(issue(
                    path.to_string(),
                    Category::Cardinality,
                    "arrays must not be empty".into(),
                ));
            }
        }
        None => {
            if el.max_many {
                issues.push(issue(
                    path.to_string(),
                    Category::Cardinality,
                    "expected an array (the element repeats)".into(),
                ));
            }
        }
    }
}

fn walk_value(
    ty: &'static TypeDef,
    rel: &str,
    type_name: &str,
    value: &Value,
    path: &str,
    ctx: &Ctx,
    issues: &mut Vec<Issue>,
) {
    match value.as_array() {
        Some(items) => {
            for (i, item) in items.iter().enumerate() {
                walk_single(
                    ty,
                    rel,
                    type_name,
                    item,
                    &format!("{path}[{i}]"),
                    ctx,
                    issues,
                );
            }
        }
        None => walk_single(ty, rel, type_name, value, path, ctx, issues),
    }
}

fn walk_single(
    ty: &'static TypeDef,
    rel: &str,
    type_name: &str,
    value: &Value,
    path: &str,
    ctx: &Ctx,
    issues: &mut Vec<Issue>,
) {
    if value.is_null() {
        issues.push(issue(
            path.to_string(),
            Category::Type,
            "null is not allowed here".into(),
        ));
        return;
    }
    // a contentReference points back into this type: re-anchor and descend
    if let Some(target) = type_name.strip_prefix('#') {
        let prefix = target.split_once('.').map_or("", |(_, rest)| rest);
        if value.is_object() {
            walk_object(ty, prefix, value, path, ctx, issues);
        } else {
            issues.push(expected_object(path));
        }
        return;
    }
    match type_name {
        "BackboneElement" | "Element" => {
            if value.is_object() {
                walk_object(ty, rel, value, path, ctx, issues);
            } else {
                issues.push(expected_object(path));
            }
        }
        "Resource" => match value.as_object() {
            Some(obj) => match obj.get("resourceType").and_then(Value::as_str) {
                Some(rt) => walk_typed(rt, value, path, ctx, issues),
                None => issues.push(issue(
                    path.to_string(),
                    Category::Type,
                    "expected a resource (resourceType is missing)".into(),
                )),
            },
            None => issues.push(expected_object(path)),
        },
        name => {
            let Some(target) = schema::type_def(name) else {
                return;
            };
            if target.kind == "primitive-type" {
                check_primitive(name, value, path, issues);
                return;
            }
            if value.is_object() {
                walk_object(target, "", value, path, ctx, issues);
            } else {
                issues.push(expected_object(path));
            }
        }
    }
}

fn expected_object(path: &str) -> Issue {
    issue(
        path.to_string(),
        Category::Type,
        "expected an object".into(),
    )
}

fn check_primitive(name: &str, value: &Value, path: &str, issues: &mut Vec<Issue>) {
    let mut bad = |message: String, category: Category| {
        issues.push(issue(path.to_string(), category, message));
    };
    match name {
        "boolean" => {
            if !value.is_boolean() {
                bad("expected true or false".into(), Category::Type);
            }
        }
        "integer" | "positiveInt" | "unsignedInt" => match value.as_i64() {
            None => bad("expected an integer".into(), Category::Type),
            Some(n) if name == "positiveInt" && n < 1 => {
                bad("expected a positive integer".into(), Category::Type)
            }
            Some(n) if name == "unsignedInt" && n < 0 => {
                bad("expected a non-negative integer".into(), Category::Type)
            }
            Some(_) => {}
        },
        "decimal" => {
            if !value.is_number() {
                bad("expected a number".into(), Category::Type);
            }
        }
        // every other primitive is a json string, many with a declared format
        _ => match value.as_str() {
            None => bad("expected a string".into(), Category::Type),
            Some(s) => {
                if format_regex(name).is_some_and(|re| !re.is_match(s)) {
                    bad(
                        format!("does not match the {name} format"),
                        Category::Format,
                    );
                }
            }
        },
    }
}

// every distinct constraint expression parses once per process
fn parsed(expression: &'static str) -> Option<&'static crate::ast::Expr> {
    static CACHE: OnceLock<HashMap<&'static str, Option<crate::ast::Expr>>> = OnceLock::new();
    CACHE
        .get_or_init(|| {
            let mut cache: HashMap<&'static str, Option<crate::ast::Expr>> = HashMap::new();
            for ty in schema::TYPES {
                for c in ty.constraints {
                    cache
                        .entry(c.expression)
                        .or_insert_with(|| parser::parse(c.expression).ok());
                }
            }
            cache
        })
        .get(expression)
        .and_then(|e| e.as_ref())
}

// the declared format regexes, anchored and compiled once
fn format_regex(name: &str) -> Option<&'static Regex> {
    static COMPILED: OnceLock<HashMap<&'static str, Regex>> = OnceLock::new();
    COMPILED
        .get_or_init(|| {
            schema::PRIMITIVE_PATTERNS
                .iter()
                .filter_map(|(n, p)| Regex::new(&format!("^(?:{p})$")).ok().map(|re| (*n, re)))
                .collect()
        })
        .get(name)
}

// valueQuantity = the "Quantity" form of the collapsed choice element "value"
fn resolve_choice(
    ty: &'static TypeDef,
    prefix: &str,
    key: &str,
) -> Option<(&'static Element, &'static str)> {
    for el in ty.children_of(prefix) {
        if !el.choice {
            continue;
        }
        let base = last_segment(el.path);
        let Some(suffix) = key.strip_prefix(base) else {
            continue;
        };
        if suffix.is_empty() {
            continue;
        }
        if let Some(t) = el.types.iter().find(|t| variant_matches(suffix, t)) {
            return Some((el, t));
        }
    }
    None
}

// the suffix is the type name with its first letter capitalized (valueString)
fn variant_matches(suffix: &str, type_name: &str) -> bool {
    if suffix == type_name {
        return true;
    }
    let mut cs = suffix.chars();
    match cs.next() {
        Some(c) => format!("{}{}", c.to_ascii_lowercase(), cs.as_str()) == type_name,
        None => false,
    }
}

fn last_segment(path: &str) -> &str {
    path.rsplit('.').next().unwrap_or(path)
}

fn capitalized(name: &str) -> String {
    let mut cs = name.chars();
    match cs.next() {
        Some(c) => format!("{}{}", c.to_ascii_uppercase(), cs.as_str()),
        None => String::new(),
    }
}

fn join(prefix: &str, key: &str) -> String {
    if prefix.is_empty() {
        key.to_string()
    } else {
        format!("{prefix}.{key}")
    }
}

fn scope(ty: &TypeDef, prefix: &str) -> String {
    if prefix.is_empty() {
        ty.name.to_string()
    } else {
        format!("{}.{prefix}", ty.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(issues: &[Issue]) -> Vec<&str> {
        issues.iter().map(|i| i.path.as_str()).collect()
    }

    /// Error-severity issues only: advisory findings are asserted explicitly.
    fn errors(json: &str) -> Vec<Issue> {
        validate(json)
            .unwrap()
            .into_iter()
            .filter(|i| i.severity == Severity::Error)
            .collect()
    }

    #[test]
    fn a_clean_resource_yields_no_issues() {
        let issues = errors(
            r#"{"resourceType":"Patient","id":"p1","active":true,
                "name":[{"family":"Chalmers","given":["Peter","James"]}],
                "deceasedBoolean":false,
                "maritalStatus":{"coding":[{"system":"http://x","code":"M"}],"text":"married"}}"#,
        );
        assert_eq!(issues, vec![]);
    }

    #[test]
    fn malformed_input_is_an_error_not_an_issue() {
        assert!(validate("{not json").is_err());
        assert!(validate(r#"{"no":"resourceType"}"#).is_err());
        assert!(validate("[1,2]").is_err());
    }

    #[test]
    fn unknown_elements_are_flagged_with_indexed_paths() {
        let issues = errors(
            r#"{"resourceType":"Patient","favouriteColor":"blue",
                "name":[{"family":"Chalmers"},{"familyy":"Oops"}],
                "contact":[{"gendr":"male","name":{"family":"Du"}}]}"#,
        );
        assert_eq!(
            paths(&issues),
            [
                "Patient.contact[0].gendr",
                "Patient.favouriteColor",
                "Patient.name[1].familyy"
            ]
        );
        assert!(issues.iter().all(|i| i.category == Category::Unknown));
    }

    #[test]
    fn unknown_resource_types_are_flagged() {
        let issues = validate(r#"{"resourceType":"Patiend"}"#).unwrap();
        assert_eq!(paths(&issues), ["Patiend"]);
        assert_eq!(issues[0].category, Category::Unknown);
    }

    #[test]
    fn choice_keys_resolve_to_their_variant() {
        let ok = errors(r#"{"resourceType":"Patient","deceasedDateTime":"2026"}"#);
        assert_eq!(ok, vec![]);
        let bad = errors(r#"{"resourceType":"Patient","deceasedFoo":true}"#);
        assert_eq!(paths(&bad), ["Patient.deceasedFoo"]);
    }

    #[test]
    fn recursion_covers_backbones_and_contained_resources() {
        let issues = errors(
            r#"{"resourceType":"Patient",
                "communication":[{"language":{"text":"en"},"fluent":true}],
                "contained":[{"resourceType":"Organization","name":"Acme","nm":"x"}]}"#,
        );
        assert_eq!(
            paths(&issues),
            ["Patient.communication[0].fluent", "Patient.contained[0].nm"]
        );
    }

    #[test]
    fn required_elements_are_enforced() {
        let issues = errors(r#"{"resourceType":"Observation","valueQuantity":{"value":1}}"#);
        assert_eq!(paths(&issues), ["Observation.code", "Observation.status"]);
        assert!(issues.iter().all(|i| i.category == Category::Cardinality));
    }

    #[test]
    fn required_is_scoped_to_present_parents() {
        let missing = errors(r#"{"resourceType":"Patient","communication":[{"preferred":true}]}"#);
        assert_eq!(paths(&missing), ["Patient.communication[0].language"]);
        assert_eq!(errors(r#"{"resourceType":"Patient"}"#), vec![]);
    }

    #[test]
    fn array_shape_must_match_cardinality() {
        let issues = errors(
            r#"{"resourceType":"Patient","name":{"family":"X"},
                "gender":["male","female"],"photo":[]}"#,
        );
        assert_eq!(
            paths(&issues),
            ["Patient.gender", "Patient.name", "Patient.photo"]
        );
        assert!(issues.iter().all(|i| i.category == Category::Cardinality));
    }

    #[test]
    fn choice_variants_are_mutually_exclusive() {
        let issues = errors(
            r#"{"resourceType":"Observation","status":"final","code":{"text":"BP"},
                "valueQuantity":{"value":120},"valueString":"high"}"#,
        );
        assert_eq!(paths(&issues), ["Observation.value"]);
        assert_eq!(issues[0].category, Category::Choice);
    }

    #[test]
    fn bare_choice_names_are_rejected() {
        let issues = errors(r#"{"resourceType":"Patient","deceased":true}"#);
        assert_eq!(paths(&issues), ["Patient.deceased"]);
        assert_eq!(issues[0].category, Category::Choice);
    }

    #[test]
    fn value_shapes_must_match_declared_types() {
        let issues = errors(
            r#"{"resourceType":"Patient","active":"yes",
                "name":[{"family":{"x":1}}],
                "maritalStatus":"M",
                "multipleBirthInteger":2.5}"#,
        );
        assert_eq!(
            paths(&issues),
            [
                "Patient.active",
                "Patient.maritalStatus",
                "Patient.multipleBirthInteger",
                "Patient.name[0].family"
            ]
        );
        assert!(issues.iter().all(|i| i.category == Category::Type));
    }

    #[test]
    fn primitive_formats_are_checked() {
        // NB not Patient.id: R4 snapshots type id/url elements as plain "string"
        // (the System.String magic url), so the string regex would accept spaces
        let issues = errors(
            r#"{"resourceType":"Patient","birthDate":"06/10/2026","gender":"male",
                "meta":{"lastUpdated":"yesterday"}}"#,
        );
        assert_eq!(
            paths(&issues),
            ["Patient.birthDate", "Patient.meta.lastUpdated"]
        );
        assert!(issues.iter().all(|i| i.category == Category::Format));
    }

    #[test]
    fn nulls_are_rejected() {
        let issues = errors(r#"{"resourceType":"Patient","active":null}"#);
        assert_eq!(paths(&issues), ["Patient.active"]);
        assert_eq!(issues[0].category, Category::Type);
    }

    #[test]
    fn underscore_companions_are_recognized() {
        let ok = errors(
            r#"{"resourceType":"Patient",
                "_birthDate":{"extension":[{"url":"http://x","valueCode":"asked-unknown"}]}}"#,
        );
        assert_eq!(ok, vec![]);
        let bad = errors(r#"{"resourceType":"Patient","_favouriteColor":{}}"#);
        assert_eq!(paths(&bad), ["Patient._favouriteColor"]);
        assert_eq!(bad[0].category, Category::Unknown);
    }

    #[test]
    fn content_references_recurse() {
        let issues = errors(
            r#"{"resourceType":"Questionnaire","status":"draft",
                "item":[{"linkId":"1","type":"group",
                         "item":[{"linkId":"1.1","type":"string","blorb":true}]}]}"#,
        );
        assert_eq!(paths(&issues), ["Questionnaire.item[0].item[0].blorb"]);
    }

    #[test]
    fn failing_invariants_are_reported() {
        // bdl-1: total only belongs on search/history bundles
        let issues =
            validate(r#"{"resourceType":"Bundle","type":"collection","total":1}"#).unwrap();
        assert_eq!(paths(&issues), ["Bundle"]);
        let i = &issues[0];
        assert_eq!(i.category, Category::Invariant);
        assert_eq!(i.severity, Severity::Error);
        assert!(i.message.starts_with("bdl-1:"), "got: {}", i.message);
    }

    #[test]
    fn datatype_instances_run_their_invariants() {
        // qty-3: a coded quantity needs a system
        let issues = errors(
            r#"{"resourceType":"Observation","status":"final","code":{"text":"BP"},
                "valueQuantity":{"value":1,"code":"kg"}}"#,
        );
        assert_eq!(paths(&issues), ["Observation.valueQuantity"]);
        assert!(issues[0].message.starts_with("qty-3:"));
    }

    #[test]
    fn backbone_invariants_run_with_the_backbone_as_focus() {
        // pat-1: a contact needs name, telecom, address, or organization
        let issues = errors(r#"{"resourceType":"Patient","contact":[{"gender":"male"}]}"#);
        assert_eq!(paths(&issues), ["Patient.contact[0]"]);
        assert!(issues[0].message.starts_with("pat-1:"));
    }

    #[test]
    fn invariant_severities_map_through() {
        // dom-6: a resource should have narrative - advisory, not an error
        let all = validate(r#"{"resourceType":"Patient"}"#).unwrap();
        let dom6 = all
            .iter()
            .find(|i| i.message.starts_with("dom-6:"))
            .unwrap();
        assert_eq!(dom6.severity, Severity::Warning);
        assert_eq!(dom6.category, Category::Invariant);
        assert_eq!(dom6.path, "Patient");
        // the error view of the same document is clean
        assert_eq!(errors(r#"{"resourceType":"Patient"}"#), vec![]);
    }

    #[test]
    fn unrunnable_expressions_never_invent_issues() {
        // dom-3 (%resource) and ele-1 (hasValue()) are beyond the engine; they
        // must skip silently, so a valid contained resource stays quiet
        let issues = errors(
            r#"{"resourceType":"Patient",
                "contained":[{"resourceType":"Organization","name":"Acme"}]}"#,
        );
        assert_eq!(issues, vec![]);
    }
}
