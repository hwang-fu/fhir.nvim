//! Validates resources against the schema tables: unknown elements,
//! cardinality, value types, primitive formats, and choice element use.

use crate::error::Error;
use crate::schema::{self, Element, Severity, TypeDef};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, HashSet};

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
    let root: Value =
        serde_json::from_str(json).map_err(|e| Error::Validate(format!("bad json: {e}")))?;
    let obj = root
        .as_object()
        .ok_or_else(|| Error::Validate("not a json object".into()))?;
    let rt = obj
        .get("resourceType")
        .and_then(Value::as_str)
        .ok_or_else(|| Error::Validate("resourceType is missing".into()))?;
    let mut issues = Vec::new();
    walk_typed(rt, obj, rt, &mut issues);
    Ok(issues)
}

fn walk_typed(type_name: &str, obj: &Map<String, Value>, path: &str, issues: &mut Vec<Issue>) {
    match schema::type_def(type_name) {
        Some(ty) => walk_object(ty, "", obj, path, issues),
        None => issues.push(issue(
            path.to_string(),
            Category::Unknown,
            format!("unknown resource type {type_name:?}"),
        )),
    }
}

fn walk_object(
    ty: &'static TypeDef,
    prefix: &str,
    obj: &Map<String, Value>,
    path: &str,
    issues: &mut Vec<Issue>,
) {
    let mut present: HashSet<&'static str> = HashSet::new();
    let mut variants: BTreeMap<&'static str, Vec<&str>> = BTreeMap::new();
    for (key, value) in obj {
        if prefix.is_empty() && key == "resourceType" {
            continue;
        }
        if key.starts_with('_') {
            continue; // companion of a primitive element (e.g. _birthDate)
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
                walk_value(ty, &rel, t, value, &child_path, issues);
            }
        } else if let Some((el, variant)) = resolve_choice(ty, prefix, key) {
            present.insert(el.path);
            variants.entry(el.path).or_default().push(key);
            check_shape(el, value, &child_path, issues);
            walk_value(ty, el.path, variant, value, &child_path, issues);
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
    issues: &mut Vec<Issue>,
) {
    match value.as_array() {
        Some(items) => {
            for (i, item) in items.iter().enumerate() {
                walk_single(ty, rel, type_name, item, &format!("{path}[{i}]"), issues);
            }
        }
        None => walk_single(ty, rel, type_name, value, path, issues),
    }
}

fn walk_single(
    ty: &'static TypeDef,
    rel: &str,
    type_name: &str,
    value: &Value,
    path: &str,
    issues: &mut Vec<Issue>,
) {
    // a contentReference points back into this type: re-anchor and descend
    if let Some(target) = type_name.strip_prefix('#') {
        let prefix = target.split_once('.').map_or("", |(_, rest)| rest);
        if let Some(obj) = value.as_object() {
            walk_object(ty, prefix, obj, path, issues);
        }
        return;
    }
    match type_name {
        "BackboneElement" | "Element" => {
            if let Some(obj) = value.as_object() {
                walk_object(ty, rel, obj, path, issues);
            }
        }
        "Resource" => {
            if let Some(obj) = value.as_object() {
                match obj.get("resourceType").and_then(Value::as_str) {
                    Some(rt) => walk_typed(rt, obj, path, issues),
                    None => issues.push(issue(
                        path.to_string(),
                        Category::Type,
                        "expected a resource (resourceType is missing)".into(),
                    )),
                }
            }
        }
        "xhtml" => {}
        name => {
            let Some(target) = schema::type_def(name) else {
                return;
            };
            if target.kind == "primitive-type" {
                return; // primitives have no children to descend into
            }
            if let Some(obj) = value.as_object() {
                walk_object(target, "", obj, path, issues);
            }
        }
    }
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

    #[test]
    fn a_clean_resource_yields_no_issues() {
        let issues = validate(
            r#"{"resourceType":"Patient","id":"p1","active":true,
                "name":[{"family":"Chalmers","given":["Peter","James"]}],
                "deceasedBoolean":false,
                "maritalStatus":{"coding":[{"system":"http://x","code":"M"}],"text":"married"}}"#,
        )
        .unwrap();
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
        let issues = validate(
            r#"{"resourceType":"Patient","favouriteColor":"blue",
                "name":[{"family":"Chalmers"},{"familyy":"Oops"}],
                "contact":[{"gendr":"male"}]}"#,
        )
        .unwrap();
        assert_eq!(
            paths(&issues),
            [
                "Patient.contact[0].gendr",
                "Patient.favouriteColor",
                "Patient.name[1].familyy"
            ]
        );
        assert!(issues.iter().all(|i| i.category == Category::Unknown));
        assert!(issues.iter().all(|i| i.severity == Severity::Error));
    }

    #[test]
    fn unknown_resource_types_are_flagged() {
        let issues = validate(r#"{"resourceType":"Patiend"}"#).unwrap();
        assert_eq!(paths(&issues), ["Patiend"]);
        assert_eq!(issues[0].category, Category::Unknown);
    }

    #[test]
    fn choice_keys_resolve_to_their_variant() {
        let ok = validate(r#"{"resourceType":"Patient","deceasedDateTime":"2026"}"#).unwrap();
        assert_eq!(ok, vec![]);
        let bad = validate(r#"{"resourceType":"Patient","deceasedFoo":true}"#).unwrap();
        assert_eq!(paths(&bad), ["Patient.deceasedFoo"]);
    }

    #[test]
    fn recursion_covers_backbones_and_contained_resources() {
        let issues = validate(
            r#"{"resourceType":"Patient",
                "communication":[{"language":{"text":"en"},"fluent":true}],
                "contained":[{"resourceType":"Organization","nm":"x"}]}"#,
        )
        .unwrap();
        assert_eq!(
            paths(&issues),
            ["Patient.communication[0].fluent", "Patient.contained[0].nm"]
        );
    }

    #[test]
    fn required_elements_are_enforced() {
        let issues =
            validate(r#"{"resourceType":"Observation","valueQuantity":{"value":1}}"#).unwrap();
        assert_eq!(paths(&issues), ["Observation.code", "Observation.status"]);
        assert!(issues.iter().all(|i| i.category == Category::Cardinality));
    }

    #[test]
    fn required_is_scoped_to_present_parents() {
        let missing =
            validate(r#"{"resourceType":"Patient","communication":[{"preferred":true}]}"#).unwrap();
        assert_eq!(paths(&missing), ["Patient.communication[0].language"]);
        assert_eq!(validate(r#"{"resourceType":"Patient"}"#).unwrap(), vec![]);
    }

    #[test]
    fn array_shape_must_match_cardinality() {
        let issues = validate(
            r#"{"resourceType":"Patient","name":{"family":"X"},
                "gender":["male","female"],"photo":[]}"#,
        )
        .unwrap();
        assert_eq!(
            paths(&issues),
            ["Patient.gender", "Patient.name", "Patient.photo"]
        );
        assert!(issues.iter().all(|i| i.category == Category::Cardinality));
    }

    #[test]
    fn choice_variants_are_mutually_exclusive() {
        let issues = validate(
            r#"{"resourceType":"Observation","status":"final","code":{"text":"BP"},
                "valueQuantity":{"value":120},"valueString":"high"}"#,
        )
        .unwrap();
        assert_eq!(paths(&issues), ["Observation.value"]);
        assert_eq!(issues[0].category, Category::Choice);
    }

    #[test]
    fn bare_choice_names_are_rejected() {
        let issues = validate(r#"{"resourceType":"Patient","deceased":true}"#).unwrap();
        assert_eq!(paths(&issues), ["Patient.deceased"]);
        assert_eq!(issues[0].category, Category::Choice);
    }

    #[test]
    fn content_references_recurse() {
        let issues = validate(
            r#"{"resourceType":"Questionnaire","status":"draft",
                "item":[{"linkId":"1","type":"group",
                         "item":[{"linkId":"1.1","type":"string","blorb":true}]}]}"#,
        )
        .unwrap();
        assert_eq!(paths(&issues), ["Questionnaire.item[0].item[0].blorb"]);
    }
}
