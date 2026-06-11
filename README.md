# fhir.nvim

[![CI](https://github.com/hwang-fu/fhir.nvim/actions/workflows/ci.yml/badge.svg)](https://github.com/hwang-fu/fhir.nvim/actions/workflows/ci.yml)
[![Neovim](https://img.shields.io/badge/Neovim-0.10%2B-blueviolet?logo=neovim&logoColor=white)](https://neovim.io)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](./LICENSE)

> Navigate and comprehend **FHIR R4 JSON** the way you navigate a codebase - jump between resources, follow references, and outline a document, without leaving your editor.

FHIR JSON is verbose, deeply nested, and reference-heavy. `fhir.nvim` treats a FHIR document like a small codebase: jump from a reference to the resource it points at, find everything that points back, and browse a navigable outline of what's in the file. It's offline-first, works on a single buffer, and needs zero configuration.

## Features

### Go to reference

Put the cursor on a `reference` value and jump to the resource it points at - relative (`Patient/123`), absolute URL, `urn:uuid:...`, or `contained` (`#id`). Jumplist-aware, so `<C-o>` brings you back.

### Find usages

On a resource, list everything that references it - the inverse of go-to-reference.

### Outline

A searchable list of every resource in the document, each with a human-readable label like `[Observation] Heart rate (id)`.

### Evaluate FHIRPath

`:FhirEval name.given` runs a FHIRPath expression against the resource under the cursor and floats the result - one JSON value per line. `resolve()` follows references through the buffer, so `subject.resolve().name.given` works inside a Bundle. Powered by the Rust engine below; get it with `:FhirFetchEngine` (linux, apple-silicon macos) or `make build`.

### Validate

`:FhirValidate` checks the whole document against the R4 definitions - structure (unknown elements, cardinality, types, primitive formats, choice elements) and constraint invariants - and puts every finding into `vim.diagnostic`: signs, virtual text, `]d` navigation. Severities follow the spec (a missing required element is an error; "should have narrative" is a warning). Attached buffers re-validate on save; `:FhirValidate!` clears the findings. Same engine, same graceful degradation when it's absent.

## Requirements

- Neovim **>= 0.10**
- The `json` Treesitter parser (a soft dependency) - `:TSInstall json` via [nvim-treesitter]. `:checkhealth fhir` reports whether it's present.
- Optional: [dressing.nvim] or [telescope-ui-select] give `vim.ui.select` a fuzzy picker (used by the outline and find-usages lists).

## Installation

With [lazy.nvim]:

```lua
{
  "hwang-fu/fhir.nvim",
  opts = {},
}
```

`opts = {}` calls `setup()` for you. Other plugin managers are analogous. v1 is pure Lua - no build step.

## Usage

Open a FHIR `.json` / `.fhir.json` file; the plugin auto-attaches when the top-level object has a `resourceType`. Then:

| Command | Does |
|---|---|
| `:FhirGoto` | jump to the reference under the cursor |
| `:FhirUsages` | list references to the resource under the cursor |
| `:FhirOutline` | pick any resource and jump to it |
| `:FhirEval [expr]` | evaluate a FHIRPath expression (prompts without args) |
| `:FhirValidate[!]` | validate the document into diagnostics (`!` clears them) |
| `:FhirEnable` / `:FhirDisable` | attach / detach the current buffer |

No keymaps are set by default. Opt in through `setup()`:

```lua
require("fhir").setup({
  keymaps = {
    goto_reference = "gd",
    find_usages    = "gr",
    outline        = "<leader>fo",
    eval           = "<leader>fe",
    diagnostics    = "gl",   -- show the finding under the cursor in a float
  },
})
```

## Configuration

| Option | Default | Description |
|---|---|---|
| `detection` | `"auto"` | `"auto"` attaches FHIR JSON buffers automatically; `"manual"` requires `:FhirEnable`. |
| `keymaps` | `{}` | Opt-in buffer-local maps: `goto_reference`, `find_usages`, `outline`, `eval`, `diagnostics`. |
| `native.dir` | unset | Explicit directory containing the `fhir_core` module; overrides the search below. |
| `native.tag` | current release | Which engine release `:FhirFetchEngine` installs and the loader looks for. |
| `validate.on_save` | `true` | Re-validate attached buffers after each write (needs the engine). |

See `:help fhir` for the full reference.

## Scope & limitations

- Targets **FHIR R4** (`4.0.1`).
- Resolves relative, absolute-URL, `urn:uuid:`, and `contained` references. References by `identifier` and conditional references are **not** resolved.
- Resolution is **single-buffer**; cross-file and live-server resolution are future work.

## FHIRPath & validation engine (in development)

The `crates/` workspace contains **`fhir-core`**, a standalone Rust FHIRPath
interpreter - hand-written lexer and Pratt parser, tree-walking evaluator over
a JSON model with exact decimals - powering `:FhirEval`.

Getting the engine: `:FhirFetchEngine` downloads a prebuilt, SHA-256-verified
binary for linux (x86_64/aarch64) and apple-silicon macos from the pinned release into Neovim's
data directory. On other platforms (or for development), `make build` compiles
it from source - that is the only case needing a Rust toolchain.

Honesty over checklists: the engine is measured against the **official
FHIRPath conformance suite** (vendored under
`crates/fhir-core/tests/conformance/`) on every pull request, with a
ratcheting pass-rate floor. *Conformance* means how faithfully an
implementation matches the behavior the spec requires; the suite - 935
input/expression/expected-output cases published with the spec - turns that
into a measurable score. Current rate: **57.2% (535/935)**.

Covered so far: path navigation and indexing; boolean/string/integer/decimal/
date/dateTime literals; equality, comparison, and `~` equivalence; arithmetic
(`+ - * / div mod`, unary minus); three-valued `and`/`or`/`xor`/`implies`/
`not()`; `in`/`contains` membership, `|` (union), `&` (concat); existence
(`exists`, `empty`, `count`, `all`, `distinct`); `where`, `select`, `ofType`
(handles polymorphic `value[x]`); subsetting (`first`/`last`/`single`/`tail`/
`skip`/`take`); `is`/`as`; string functions (`length`, `upper`, `lower`,
`trim`, `startsWith`, `endsWith`, `contains`, `substring`, `indexOf`,
`replace`, `split`, `join`, `toChars`, `matches`, `replaceMatches`); math
functions (`abs`, `ceiling`, `floor`, `round`, `truncate`, `sqrt`, `exp`,
`ln`, `log`, `power`); conversions (`toString`/`toInteger`/`toDecimal`/
`toBoolean` and their `convertsToX` twins); `iif` (lazy), `children`,
`descendants`, `repeat`; quantity literals (`1 year`, `5 'mg'`) with
same-unit arithmetic and comparison (including against FHIR Quantity
values); date/dateTime arithmetic with calendar durations (precision
preserved, end-of-month clamped); `today()`/`now()`; `extension(url)`;
`resolve()` via a pluggable resolver trait.

Not covered yet: unit conversion (UCUM), `%variables`, type reflection,
strict choice-element typing rules, and terminology functions.

The engine also **validates** resources against the R4 definitions:
structure (unknown elements, cardinality, value types, primitive formats,
choice elements) and constraint invariants evaluated as FHIRPath, over
schema tables generated from the official definitions. Measured against
the **official R4 example corpus** (2,911 documents, `make corpus`):
**100%** validate without error-severity findings - after documenting 203
examples that are imperfect upstream and two invariant keys the engine
cannot yet evaluate faithfully; every exclusion is listed in the harness
output, and the rate is a ratcheting floor in CI.

## Roadmap

Shipped so far: navigation (pure Lua), FHIRPath evaluation, and validation
diagnostics - the Rust engine reaching the editor through a native module
with graceful degradation when it is absent. Ahead:

- **Workspace awareness** - cross-file reference resolution and
  workspace-wide validation.
- **Quick fixes** - code actions for common findings.

## Development

```sh
make test   # plenary specs (clones plenary into .tests/ on first run)
make lint   # stylua + luacheck

cargo test -p fhir-core                 # engine unit + conformance tests
cargo clippy -p fhir-core -- -D warnings
```

## License

[MIT](./LICENSE)

[lazy.nvim]: https://github.com/folke/lazy.nvim
[nvim-treesitter]: https://github.com/nvim-treesitter/nvim-treesitter
[dressing.nvim]: https://github.com/stevearc/dressing.nvim
[telescope-ui-select]: https://github.com/nvim-telescope/telescope-ui-select.nvim
