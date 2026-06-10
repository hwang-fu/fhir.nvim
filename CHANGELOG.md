# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

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
