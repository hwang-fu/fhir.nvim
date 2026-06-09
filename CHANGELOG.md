# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

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

[1.0.0]: https://github.com/hwang-fu/fhir.nvim/releases/tag/v1.0.0
