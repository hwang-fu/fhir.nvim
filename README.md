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
| `:FhirEnable` / `:FhirDisable` | attach / detach the current buffer |

No keymaps are set by default. Opt in through `setup()`:

```lua
require("fhir").setup({
  keymaps = {
    goto_reference = "gd",
    find_usages    = "gr",
    outline        = "<leader>fo",
  },
})
```

## Configuration

| Option | Default | Description |
|---|---|---|
| `detection` | `"auto"` | `"auto"` attaches FHIR JSON buffers automatically; `"manual"` requires `:FhirEnable`. |
| `keymaps` | `{}` | Opt-in buffer-local maps: `goto_reference`, `find_usages`, `outline`. |

See `:help fhir` for the full reference.

## Scope & limitations

- Targets **FHIR R4** (`4.0.1`).
- Resolves relative, absolute-URL, `urn:uuid:`, and `contained` references. References by `identifier` and conditional references are **not** resolved.
- Resolution is **single-buffer**; cross-file and live-server resolution are future work.

## Roadmap

v1 (this release) is navigation, in pure Lua. Planned, not yet built:

- **FHIRPath evaluation** in the editor, powered by a standalone, conformance-tested **Rust** engine.
- **Validation & diagnostics** against R4 structure rules and constraints.

## Development

```sh
make test   # plenary specs (clones plenary into .tests/ on first run)
make lint   # stylua + luacheck
```

## License

[MIT](./LICENSE)

[lazy.nvim]: https://github.com/folke/lazy.nvim
[nvim-treesitter]: https://github.com/nvim-treesitter/nvim-treesitter
[dressing.nvim]: https://github.com/stevearc/dressing.nvim
[telescope-ui-select]: https://github.com/nvim-telescope/telescope-ui-select.nvim
