# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [4.0.0] - 2026-06-11

Workspace awareness: the plugin stops ending at the buffer edge.

### Added

- Cross-file navigation: `:FhirGoto` follows references into other files
  under the workspace root (the enclosing git repository, or the cwd),
  `:FhirUsages` lists referrers from the whole project, and the global
  `:FhirWorkspaceOutline` picks from every resource - files need not be
  open. Ambiguous identities pick the first match and say so.
- FHIRPath `resolve()` and validation invariants reach workspace targets
  through the same resolver.
- `:FhirWorkspaceValidate [all]` validates every document under the root
  into the quickfix list at exact element positions - error findings by
  default, `all` includes advisories; open buffers refresh their
  diagnostics; the summary reports every count.
- `:FhirFix` repairs the structural finding under the cursor: remove an
  unknown element, insert a missing required element, rewrite a bare
  choice name to its typed form, fix array shape, drop an empty array.
  Single undo step per repair; the buffer re-validates immediately.
- `workspace` configuration (`ignore`, `max_files` - discovery is lazy,
  bounded, and reports clipping) and an opt-in `keymaps.fix` mapping.
- `:checkhealth fhir` reports the workspace root and candidate count.

### Changed

- The default engine release pin (`native.tag`) is `v4.0.0`. The engine
  itself is unchanged since v3.0.0 - this release is pure Lua.

## [3.0.0] - 2026-06-11

Validation: open a FHIR document and see what is wrong with it, inline.

### Added

- `:FhirValidate` validates the buffer's document against the R4
  definitions - structure (unknown elements, cardinality, value types,
  primitive formats, choice elements) and constraint invariants evaluated
  as FHIRPath - and publishes every finding as a `vim.diagnostic` entry at
  the offending element, with the spec's severities. `:FhirValidate!`
  clears the findings.
- Automatic re-validation on save for attached buffers (`validate.on_save`,
  default `true`; quietly skipped when the engine is absent).
- Invariants resolve references through the buffer, like `:FhirEval`;
  expressions beyond the engine's measured subset are skipped, never
  guessed.
- The schema behind the checks is generated from the official R4
  definitions; the validator is measured against the official R4 example
  corpus (2911 documents, 100% clean or documented) as a ratcheting CI
  floor.
- `validate(json)` in the native module - an issue array as JSON, with the
  same nil+message error convention as `eval`.

### Changed

- The default engine release pin (`native.tag`) is `v3.0.0`.

## [2.2.0] - 2026-06-11

Date and quantity arithmetic; conformance reaches 57.2% of the official
suite.

### Added

- Quantity literals (`1 year`, `5 'mg'`) and same-unit quantity arithmetic
  and comparison - including against FHIR Quantity values from documents
  (`Observation.value > 100 'lbs'`). Mismatched units yield empty; no unit
  conversion.
- Date/dateTime arithmetic with calendar durations: precision preserved
  (`@2014 + 1 year = @2015`), end-of-month clamping, time carry across
  days, timezone suffixes untouched.
- `today()` and `now()`.

### Changed

- The default engine release pin (`native.tag`) is `v2.2.0`.

## [2.1.0] - 2026-06-10

Expanded FHIRPath support; the conformance rate more than doubles
(26.5% -> 54.9% of the official suite).

### Added

- Arithmetic (`+ - * / div mod`, unary minus), `xor`/`implies`, `~`/`!~`
  equivalence, and the `contains` membership operator.
- String functions: `length`, `upper`, `lower`, `trim`, `startsWith`,
  `endsWith`, `contains`, `substring`, `indexOf`, `replace`, `split`,
  `join`, `toChars`, `matches`, `replaceMatches`.
- Math functions: `abs`, `ceiling`, `floor`, `round`, `truncate`, `sqrt`,
  `exp`, `ln`, `log`, `power`.
- Conversions: `toString`, `toInteger`, `toDecimal`, `toBoolean`, and the
  `convertsToX` family.
- `iif` (lazily evaluated), `children`, `descendants`, `repeat`.

### Changed

- The default engine release pin (`native.tag`) is `v2.1.0`.

Date and quantity arithmetic remain out; see the README for the coverage
list.

## [2.0.0] - 2026-06-10

FHIRPath evaluation in the editor, powered by a standalone Rust engine.

### Added

- `:FhirEval [expr]` (and `keymaps.eval`): evaluate a FHIRPath expression
  against the resource under the cursor; results in a floating window titled
  with the target resource. Bare `:FhirEval` prompts for an expression.
- A standalone Rust FHIRPath interpreter (`crates/fhir-core`): hand-written
  lexer and Pratt parser, tree-walking evaluator over a JSON model with exact
  decimals. Measured against the official FHIRPath conformance suite (rate
  published in the README and enforced as a ratcheting CI floor).
- `resolve()` inside expressions follows references through the open buffer -
  `subject.resolve().name.given` works in a Bundle.
- `:FhirFetchEngine [tag]`: download a prebuilt, checksum-verified engine for
  linux (x86_64/aarch64) or apple-silicon macos into Neovim's data directory -
  no Rust toolchain needed. `make build` remains for other platforms;
  `native.dir` and `native.tag` config options control the lookup and the pin.
- `:checkhealth fhir` reports engine availability.

## [1.0.0] - 2026-06-09

First release: navigate and comprehend FHIR R4 JSON like a codebase.
Pure Lua, offline, single-buffer, zero-config.

### Added

- Auto-detection of FHIR R4 buffers via the top-level `resourceType`; manual
  `:FhirEnable` / `:FhirDisable` and a `detection` config option.
- Go-to-reference (`:FhirGoto`): jump from a `reference` to the resource it
  points at - relative, absolute-URL, `urn:uuid:`, and `contained` flavors;
  jumplist-aware.
- Find-usages (`:FhirUsages`): list everything that references the resource
  under the cursor.
- Outline (`:FhirOutline`): a labeled, navigable list of every resource.
- Opt-in, buffer-local keymaps (`goto_reference`, `find_usages`, `outline`).
- `:checkhealth fhir`: Neovim version, `json` Treesitter parser, and picker.
- `:help fhir` documentation.

[2.2.0]: https://github.com/hwang-fu/fhir.nvim/releases/tag/v2.2.0
[2.1.0]: https://github.com/hwang-fu/fhir.nvim/releases/tag/v2.1.0
[2.0.0]: https://github.com/hwang-fu/fhir.nvim/releases/tag/v2.0.0
[1.0.0]: https://github.com/hwang-fu/fhir.nvim/releases/tag/v1.0.0
