//! Verdict harness over the official R4 example corpus. The published
//! examples are valid, so every error-severity issue raised on them is a
//! false positive unless documented in the lists below. The clean rate is
//! a ratchet: it may only go up.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

/// Fraction of examples that must validate without error-severity issues.
const CLEAN_FLOOR: f64 = 1.0;

/// Invariant keys reported wrongly because of engine limits, with reasons.
const SUPPRESSED_INVARIANTS: &[(&str, &str)] = &[
    (
        "ext-1",
        "primitive values carried only by an _element companion are invisible to the engine's value.exists()",
    ),
    (
        "tst-5",
        "schema-less choice navigation also counts responseCode as the Code form of response",
    ),
];

/// Example files (by stem) that are known-imperfect upstream. The middle
/// field is the message fragment naming the ONE finding each file is
/// forgiven; anything else in these files still fails the ratchet.
const KNOWN_EXCEPTIONS: &[(&str, &str, &str)] = &[
    // logical-model examples: abstract is false yet baseDefinition is absent
    ("definition", "sdf-4:", "violates sdf-4 upstream"),
    ("event", "sdf-4:", "violates sdf-4 upstream"),
    ("fivews", "sdf-4:", "violates sdf-4 upstream"),
    ("request", "sdf-4:", "violates sdf-4 upstream"),
    // definitional search parameter examples lack the required base element
    ("codesystem-extensions-CodeSystem-author", "\"base\" is missing", "no base"),
    ("codesystem-extensions-CodeSystem-effective", "\"base\" is missing", "no base"),
    ("codesystem-extensions-CodeSystem-end", "\"base\" is missing", "no base"),
    ("codesystem-extensions-CodeSystem-keyword", "\"base\" is missing", "no base"),
    ("codesystem-extensions-CodeSystem-workflow", "\"base\" is missing", "no base"),
    ("valueset-extensions-ValueSet-author", "\"base\" is missing", "no base"),
    ("valueset-extensions-ValueSet-effective", "\"base\" is missing", "no base"),
    ("valueset-extensions-ValueSet-end", "\"base\" is missing", "no base"),
    ("valueset-extensions-ValueSet-keyword", "\"base\" is missing", "no base"),
    ("valueset-extensions-ValueSet-workflow", "\"base\" is missing", "no base"),
];

/// Whole families of known-imperfect generated examples, by stem suffix,
/// each forgiven exactly the finding its fragment names.
const KNOWN_EXCEPTION_SUFFIXES: &[(&str, &str, &str)] = &[(
    "-questionnaire",
    "\"linkId\" is missing",
    "the spec's generated questionnaire forms omit linkId on display items",
)];

fn corpus_dir() -> Option<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.tests/r4-examples");
    dir.is_dir().then_some(dir)
}

#[test]
fn r4_examples_validate_cleanly() {
    let Some(dir) = corpus_dir() else {
        eprintln!("r4 example corpus not present; run `make corpus` to enable");
        return; // a download, not vendored: absent locally is fine, CI fetches
    };

    let (mut total, mut clean, mut skipped) = (0usize, 0usize, 0usize);
    let mut by_category: BTreeMap<String, usize> = BTreeMap::new();
    let mut by_invariant: BTreeMap<String, usize> = BTreeMap::new();
    let mut samples: BTreeMap<String, String> = BTreeMap::new();
    let mut forgiven: BTreeMap<&str, usize> = BTreeMap::new();

    let mut files: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    files.sort();

    for path in &files {
        let stem = path.file_stem().unwrap().to_string_lossy().to_string();
        let json = fs::read_to_string(path).unwrap();
        let Ok(issues) = fhir_core::validate(&json) else {
            skipped += 1; // not a resource; counted so nothing hides
            continue;
        };
        total += 1;
        let mut failed = false;
        for i in &issues {
            if i.severity != fhir_core::Severity::Error {
                continue;
            }
            let key = i.message.split(':').next().unwrap_or("").to_string();
            if i.category == fhir_core::Category::Invariant
                && SUPPRESSED_INVARIANTS.iter().any(|(k, _)| *k == key)
            {
                continue;
            }
            // a documented upstream imperfection: forgiven by finding, not
            // by file - anything else in these files still counts
            let pardon = KNOWN_EXCEPTIONS
                .iter()
                .find(|(s, frag, _)| *s == stem && i.message.contains(frag))
                .or_else(|| {
                    KNOWN_EXCEPTION_SUFFIXES
                        .iter()
                        .find(|(s, frag, _)| stem.ends_with(s) && i.message.contains(frag))
                });
            if let Some((_, frag, _)) = pardon {
                *forgiven.entry(frag).or_insert(0) += 1;
                continue;
            }
            failed = true;
            let bucket = if i.category == fhir_core::Category::Invariant {
                by_invariant.entry(key.clone()).or_insert(0)
            } else {
                by_category.entry(format!("{:?}", i.category)).or_insert(0)
            };
            *bucket += 1;
            let label = format!("{:?}/{key}", i.category);
            samples
                .entry(label)
                .or_insert_with(|| format!("{stem}: {} - {}", i.path, i.message));
        }
        if !failed {
            clean += 1;
        }
    }

    let rate = clean as f64 / total as f64;
    eprintln!(
        "r4 examples: {clean}/{total} clean ({:.1}%), {skipped} skipped",
        rate * 100.0
    );
    eprintln!("error issues by category: {by_category:?}");
    eprintln!("error issues by invariant: {by_invariant:?}");
    eprintln!("forgiven findings: {forgiven:?}");
    for (label, sample) in &samples {
        eprintln!("  sample [{label}] {sample}");
    }
    if !SUPPRESSED_INVARIANTS.is_empty() {
        eprintln!("suppressed invariants: {SUPPRESSED_INVARIANTS:?}");
    }
    if !KNOWN_EXCEPTIONS.is_empty() {
        eprintln!("known exceptions: {KNOWN_EXCEPTIONS:?}");
    }
    if !KNOWN_EXCEPTION_SUFFIXES.is_empty() {
        eprintln!("known exception families: {KNOWN_EXCEPTION_SUFFIXES:?}");
    }
    assert!(
        rate >= CLEAN_FLOOR,
        "clean rate {rate:.4} fell below the floor {CLEAN_FLOOR}"
    );
}
