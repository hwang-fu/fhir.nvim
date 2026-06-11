//! Static R4 schema tables (see generated.rs) and their lookup API.
#![allow(dead_code)] // consumed by validation

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Severity {
    Error,
    Warning,
    Information,
}

pub struct Element {
    pub path: &'static str,
    pub min: u32,
    pub max_many: bool,
    pub choice: bool,
    pub types: &'static [&'static str],
}

pub struct Constraint {
    pub path: &'static str,
    pub key: &'static str,
    pub severity: Severity,
    pub human: &'static str,
    pub expression: &'static str,
}

pub struct TypeDef {
    pub name: &'static str,
    pub kind: &'static str,
    pub elements: &'static [Element],
    pub constraints: &'static [Constraint],
}

// generated.rs must stay byte-identical to the generator's output -
// reformatting it would break the regeneration diff contract
#[rustfmt::skip]
mod generated;
pub use generated::{PRIMITIVE_PATTERNS, TYPES};

/// The value format regex a primitive type's definition declares.
pub fn pattern(name: &str) -> Option<&'static str> {
    PRIMITIVE_PATTERNS
        .binary_search_by(|(n, _)| (*n).cmp(name))
        .ok()
        .map(|i| PRIMITIVE_PATTERNS[i].1)
}

pub fn type_def(name: &str) -> Option<&'static TypeDef> {
    TYPES
        .binary_search_by(|t| t.name.cmp(name))
        .ok()
        .map(|i| &TYPES[i])
}

impl TypeDef {
    pub fn element(&self, path: &str) -> Option<&'static Element> {
        self.elements
            .binary_search_by(|e| e.path.cmp(path))
            .ok()
            .map(|i| &self.elements[i])
    }

    // direct children of a backbone path (one extra segment, no further dots)
    pub fn children_of<'a>(&'a self, prefix: &'a str) -> impl Iterator<Item = &'static Element> + 'a {
        self.elements.iter().filter(move |e| {
            let rest = if prefix.is_empty() {
                Some(e.path)
            } else {
                e.path.strip_prefix(prefix).and_then(|r| r.strip_prefix('.'))
            };
            rest.is_some_and(|r| !r.is_empty() && !r.contains('.'))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_r4_facts_hold() {
        let patient = type_def("Patient").unwrap();
        let name = patient.element("name").unwrap();
        assert!(name.max_many && name.min == 0);
        assert_eq!(name.types, ["HumanName"]);

        let birth = patient.element("birthDate").unwrap();
        assert!(!birth.max_many);
        assert_eq!(birth.types, ["date"]);

        let obs = type_def("Observation").unwrap();
        assert!(obs.element("status").unwrap().min == 1);
        let value = obs.element("value").unwrap();
        assert!(value.choice && value.types.len() > 5);

        // backbone children resolve by prefix
        assert!(patient.element("contact.relationship").is_some());
        assert!(patient.children_of("contact").any(|e| e.path == "contact.name"));

        // datatypes are present too
        assert!(type_def("HumanName").unwrap().element("family").is_some());

        // invariants made it across, with severities
        assert!(obs.constraints.iter().any(|c| c.severity == Severity::Error));
        assert!(type_def("Nope").is_none());
    }

    #[test]
    fn primitive_patterns_are_available() {
        assert!(pattern("date").is_some());
        assert!(pattern("boolean").is_some());
        assert!(pattern("HumanName").is_none());
        assert!(PRIMITIVE_PATTERNS.windows(2).all(|w| w[0].0 < w[1].0));
    }

    #[test]
    fn tables_are_sorted_for_binary_search() {
        assert!(TYPES.windows(2).all(|w| w[0].name < w[1].name));
        for t in TYPES {
            assert!(t.elements.windows(2).all(|w| w[0].path < w[1].path));
        }
    }
}
