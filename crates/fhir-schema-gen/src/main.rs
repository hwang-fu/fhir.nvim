//! Compiles FHIR R4 definition bundles into the static schema tables
//! consumed by fhir-core. Run via `make schema`.

mod model;
mod render;

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [types_path, resources_path, out_path, provenance] = &args[..] else {
        eprintln!(
            "usage: fhir-schema-gen <profiles-types.json> <profiles-resources.json> <out.rs> <provenance>"
        );
        return ExitCode::from(2);
    };
    match run(types_path, resources_path, out_path, provenance) {
        Ok(n) => {
            eprintln!("fhir-schema-gen: wrote {n} types to {out_path}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("fhir-schema-gen: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(
    types_path: &str,
    resources_path: &str,
    out_path: &str,
    provenance: &str,
) -> Result<usize, String> {
    let mut types = Vec::new();
    for path in [types_path, resources_path] {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
        let bundle: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))?;
        let entries = bundle
            .get("entry")
            .and_then(serde_json::Value::as_array)
            .ok_or(format!("{path}: not a definition bundle"))?;
        for entry in entries {
            let Some(res) = entry.get("resource") else {
                continue;
            };
            if res.get("resourceType").and_then(serde_json::Value::as_str)
                != Some("StructureDefinition")
            {
                continue;
            }
            types.extend(model::parse_structure_definition(res));
        }
    }
    std::fs::write(out_path, render::render(&types, provenance))
        .map_err(|e| format!("{out_path}: {e}"))?;
    Ok(types.len())
}
