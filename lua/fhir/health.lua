local M = {}

function M.check()
  vim.health.start("fhir.nvim")

  if vim.fn.has("nvim-0.10") == 1 then
    vim.health.ok("Neovim >= 0.10")
  else
    vim.health.error("Neovim >= 0.10 is required")
  end

  if pcall(vim.treesitter.language.add, "json") then
    vim.health.ok("Treesitter `json` parser found")
  else
    vim.health.warn("Treesitter `json` parser not found", {
      "Install it (e.g. `:TSInstall json` via nvim-treesitter).",
      "Navigation is unavailable for a buffer without it; the plugin will not crash.",
    })
  end

  vim.health.ok("picker: vim.ui.select (dressing.nvim or telescope-ui-select add fuzzy filtering)")

  if require("fhir.native").available() then
    vim.health.ok("FHIRPath engine loaded")
  else
    vim.health.warn("FHIRPath engine not available", {
      ":FhirFetchEngine downloads a prebuilt engine (linux, apple-silicon macos).",
      "Or build it with `make build` from the plugin directory.",
      ":FhirEval is unavailable without it; navigation works regardless.",
    })
  end

  local workspace = require("fhir.workspace")
  local file = vim.api.nvim_buf_get_name(0)
  local root = workspace.root(file ~= "" and file or vim.uv.cwd())
  local files, clipped = workspace.files(root)
  vim.health.info(
    ("workspace root %s: %d json candidates%s"):format(
      root,
      #files,
      clipped and " (clipped - raise workspace.max_files)" or ""
    )
  )
end

return M
