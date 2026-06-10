//! Parses FHIR StructureDefinitions into the intermediate model the
//! renderer consumes.

use serde_json::Value;

pub struct TypeModel {
    pub name: String,
    pub kind: String,
    pub elements: Vec<ElementModel>,
    pub constraints: Vec<ConstraintModel>,
}

pub struct ElementModel {
    /// Relative to the type, `[x]` stripped.
    pub path: String,
    pub min: u32,
    pub max_many: bool,
    pub choice: bool,
    pub types: Vec<String>,
}

pub struct ConstraintModel {
    /// Relative element path; empty for root-level constraints.
    pub path: String,
    pub key: String,
    pub severity: String,
    pub human: String,
    pub expression: String,
}

#[cfg(test)]
impl TypeModel {
    pub fn element(&self, path: &str) -> Option<&ElementModel> {
        self.elements.iter().find(|e| e.path == path)
    }
}

// The FHIRPath "magic" type urls on primitive value elements map to plain names.
fn type_code(t: &Value) -> Option<String> {
    let code = t.get("code")?.as_str()?;
    Some(match code.strip_prefix("http://hl7.org/fhirpath/System.") {
        Some(name) => name.to_lowercase(),
        None => code.to_string(),
    })
}

pub fn parse_structure_definition(sd: &Value) -> Option<TypeModel> {
    if sd.get("abstract").and_then(Value::as_bool) == Some(true) {
        return None;
    }
    // constraint profiles (e.g. SimpleQuantity) restate a base type, not define one
    if sd.get("derivation").and_then(Value::as_str) == Some("constraint") {
        return None;
    }
    let name = sd.get("type")?.as_str()?.to_string();
    let kind = sd.get("kind")?.as_str()?.to_string();
    let rows = sd.get("snapshot")?.get("element")?.as_array()?;

    let prefix = format!("{name}.");
    let mut elements = Vec::new();
    let mut constraints = Vec::new();
    for (i, el) in rows.iter().enumerate() {
        let full = el.get("path")?.as_str()?;
        let rel = if i == 0 {
            String::new()
        } else {
            full.strip_prefix(&prefix)?.to_string()
        };
        let (rel, choice) = match rel.strip_suffix("[x]") {
            Some(s) => (s.to_string(), true),
            None => (rel, false),
        };

        if let Some(cs) = el.get("constraint").and_then(Value::as_array) {
            for c in cs {
                let get = |k: &str| c.get(k).and_then(Value::as_str).unwrap_or("").to_string();
                constraints.push(ConstraintModel {
                    path: rel.clone(),
                    key: get("key"),
                    severity: get("severity"),
                    human: get("human"),
                    expression: get("expression"),
                });
            }
        }
        if i == 0 {
            // the root row defines the type itself, not an element
            continue;
        }

        let types = match el.get("contentReference").and_then(Value::as_str) {
            Some(cr) => vec![cr.to_string()],
            None => el
                .get("type")
                .and_then(Value::as_array)
                .map(|ts| ts.iter().filter_map(type_code).collect())
                .unwrap_or_default(),
        };
        elements.push(ElementModel {
            path: rel,
            min: el.get("min").and_then(Value::as_u64).unwrap_or(0) as u32,
            max_many: el.get("max").and_then(Value::as_str) == Some("*"),
            choice,
            types,
        });
    }
    Some(TypeModel {
        name,
        kind,
        elements,
        constraints,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // a trimmed StructureDefinition with the shapes that matter
    const SD: &str = r##"{
      "resourceType": "StructureDefinition",
      "name": "Demo", "kind": "resource", "type": "Demo",
      "snapshot": { "element": [
        { "path": "Demo", "min": 0, "max": "*",
          "constraint": [ { "key": "dem-1", "severity": "error",
            "human": "name or id", "expression": "name.exists() or id.exists()" } ] },
        { "path": "Demo.name", "min": 1, "max": "*",
          "type": [ { "code": "HumanName" } ] },
        { "path": "Demo.value[x]", "min": 0, "max": "1",
          "type": [ { "code": "Quantity" }, { "code": "string" } ] },
        { "path": "Demo.part", "min": 0, "max": "1",
          "type": [ { "code": "BackboneElement" } ] },
        { "path": "Demo.part.item", "min": 0, "max": "*",
          "contentReference": "#Demo.part" }
      ] }
    }"##;

    #[test]
    fn parses_a_structure_definition() {
        let sd: serde_json::Value = serde_json::from_str(SD).unwrap();
        let ty = parse_structure_definition(&sd).unwrap();
        assert_eq!(ty.name, "Demo");

        let name = ty.element("name").unwrap();
        assert_eq!((name.min, name.max_many), (1, true));
        assert_eq!(name.types, vec!["HumanName"]);
        assert!(!name.choice);

        let value = ty.element("value").unwrap(); // [x] stripped
        assert!(value.choice);
        assert_eq!(value.types, vec!["Quantity", "string"]);
        assert!(!value.max_many);

        let item = ty.element("part.item").unwrap(); // backbone child, relative path
        assert_eq!(item.types, vec!["#Demo.part"]); // contentReference as a type

        assert_eq!(ty.constraints.len(), 1);
        assert_eq!(ty.constraints[0].key, "dem-1");
        assert_eq!(ty.constraints[0].path, ""); // root-level constraint
    }

    #[test]
    fn skips_constraint_profiles() {
        // e.g. SimpleQuantity: a profile on Quantity, not a type of its own
        let sd: serde_json::Value = serde_json::from_str(
            r#"{"resourceType":"StructureDefinition","name":"SimpleQuantity","kind":"complex-type",
                "type":"Quantity","derivation":"constraint",
                "snapshot":{"element":[{"path":"Quantity","min":0,"max":"*"}]}}"#,
        )
        .unwrap();
        assert!(parse_structure_definition(&sd).is_none());
    }

    #[test]
    fn skips_abstract_and_non_snapshot_definitions() {
        let sd: serde_json::Value = serde_json::from_str(
            r#"{"resourceType":"StructureDefinition","name":"X","kind":"resource","type":"X","abstract":true}"#,
        )
        .unwrap();
        assert!(parse_structure_definition(&sd).is_none());
    }
}
