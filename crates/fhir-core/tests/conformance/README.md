# FHIRPath conformance suite (vendored)

- `tests-fhir-r4.xml`: the official FHIRPath test suite for FHIR R4, from the
  FHIR test-cases repository.
  - Source: https://raw.githubusercontent.com/FHIR/fhir-test-cases/master/r4/fhirpath/tests-fhir-r4.xml
  - Vendored 2026-06-09 at upstream commit `64e28a4a39e2a7b5aba94291a40f5be5cf659018`.
- `input/*.json`: the example resources the suite evaluates against, from the
  FHIR R4 specification (https://hl7.org/fhir/R4/<name>.json).
  - Five inputs the suite references are not published at either source
    (`appointment-examplereq`, `patient-container-example`,
    `patient-name-extensions`, `parameters-example-types`,
    `patient-example-period`); the 12 tests using them count as failures.
- License: FHIR test material is CC0 / public domain.
- Do not edit these files; re-vendor to update, then re-check `RATE_FLOOR`
  in `../conformance.rs`.
