#![allow(dead_code)]

mod model;

fn main() {
    eprintln!(
        "usage: fhir-schema-gen <profiles-types.json> <profiles-resources.json> <out.rs> <provenance>"
    );
    std::process::exit(2);
}
