//! Runs the vendored official FHIRPath test suite and enforces a pass-rate
//! floor. See conformance/README.md for provenance.

use fhir_core::{Value, evaluate};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// Raise to the measured rate (rounded down) whenever it improves.
const RATE_FLOOR: f64 = 0.54;

fn expected_value(ty: &str, text: &str) -> Value {
    match ty {
        "boolean" => Value::Boolean(text == "true"),
        "integer" => Value::Integer(text.parse().expect("integer output")),
        "decimal" => Value::Decimal(text.parse().expect("decimal output")),
        "date" => Value::Date(text.trim_start_matches('@').to_string()),
        "dateTime" => Value::DateTime(text.trim_start_matches('@').to_string()),
        // string, code, uri, quantity-as-text, ...: compare textually
        _ => Value::String(text.to_string()),
    }
}

#[test]
fn conformance_suite() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/conformance");
    let xml = fs::read_to_string(dir.join("tests-fhir-r4.xml"))
        .expect("vendored suite missing - see tests/conformance/README.md");
    let doc = roxmltree::Document::parse(&xml).expect("suite xml parses");

    let mut inputs: BTreeMap<String, Option<String>> = BTreeMap::new();
    let mut stats: BTreeMap<String, (u32, u32)> = BTreeMap::new(); // group -> (pass, total)
    let mut failures: Vec<String> = Vec::new();

    for group in doc.descendants().filter(|n| n.has_tag_name("group")) {
        let gname = group.attribute("name").unwrap_or("?").to_string();
        for test in group.children().filter(|n| n.has_tag_name("test")) {
            let tname = test.attribute("name").unwrap_or("?");
            let entry = stats.entry(gname.clone()).or_insert((0, 0));
            entry.1 += 1;

            let Some(expr_node) = test.children().find(|n| n.has_tag_name("expression")) else {
                failures.push(format!("{gname}/{tname}: no expression"));
                continue;
            };
            let expr = expr_node.text().unwrap_or("");
            let expect_invalid = expr_node.attribute("invalid").is_some();

            // the suite references .xml input names; we vendor .json twins
            let json = match test.attribute("inputfile") {
                Some(f) => {
                    let f = f.replace(".xml", ".json");
                    let content = inputs
                        .entry(f.clone())
                        .or_insert_with(|| fs::read_to_string(dir.join("input").join(&f)).ok());
                    match content {
                        Some(s) => s.clone(),
                        None => {
                            failures.push(format!("{gname}/{tname}: missing input {f}"));
                            continue;
                        }
                    }
                }
                None => "{}".to_string(),
            };

            // predicate="true" means: coerce the result to "does it exist"
            let predicate = test.attribute("predicate") == Some("true");
            let pass = match (expect_invalid, evaluate(&json, expr)) {
                (true, result) => result.is_err(),
                (false, Err(_)) => false,
                (false, Ok(actual)) => {
                    let expected: Vec<Value> = test
                        .children()
                        .filter(|n| n.has_tag_name("output"))
                        .map(|o| {
                            expected_value(o.attribute("type").unwrap_or(""), o.text().unwrap_or(""))
                        })
                        .collect();
                    if predicate {
                        expected == [Value::Boolean(!actual.is_empty())]
                    } else {
                        actual.len() == expected.len()
                            && actual.iter().zip(&expected).all(|(a, e)| a == e)
                    }
                }
            };
            if pass {
                entry.0 += 1;
            } else {
                failures.push(format!("{gname}/{tname}: {expr}"));
            }
        }
    }

    let (pass, total) = stats
        .values()
        .fold((0u32, 0u32), |(p, t), (gp, gt)| (p + gp, t + gt));
    println!("\n== FHIRPath conformance ==");
    for (g, (p, t)) in &stats {
        println!("{p:>4}/{t:<4} {g}");
    }
    let rate = f64::from(pass) / f64::from(total.max(1));
    println!(
        "== overall: {pass}/{total} = {:.1}% (floor {:.1}%) ==",
        rate * 100.0,
        RATE_FLOOR * 100.0
    );
    println!("== first failures ==");
    for f in failures.iter().take(20) {
        println!("  {f}");
    }

    assert!(total > 0, "no tests found - is the suite vendored?");
    assert!(
        rate >= RATE_FLOOR,
        "conformance regressed: {rate:.3} < floor {RATE_FLOOR:.3}"
    );
}
