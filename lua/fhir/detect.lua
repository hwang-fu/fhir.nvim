local config = require("fhir.config")
local parse = require("fhir.parse")

local M = {}

-- Cheap, pcall-guarded check: does the buffer's top-level object have a resourceType?
function M.is_fhir(bufnr)
  local ok, result = pcall(function()
    local root = parse.root(bufnr)
    return root ~= nil and parse.string_value(root, "resourceType", bufnr) ~= nil
  end)
  return ok and result or false
end

-- Attach fhir.nvim to a buffer: buffer-local state, commands, and opt-in keymaps.
function M.attach(bufnr)
  if vim.b[bufnr].fhir_attached then
    return
  end
  vim.b[bufnr].fhir_attached = true
  vim.api.nvim_buf_create_user_command(bufnr, "FhirGoto", function()
    require("fhir.features.goto").run()
  end, { desc = "Go to the FHIR reference under the cursor" })
  vim.api.nvim_buf_create_user_command(bufnr, "FhirUsages", function()
    require("fhir.features.usages").run()
  end, { desc = "List references to the FHIR resource under the cursor" })
  vim.api.nvim_buf_create_user_command(bufnr, "FhirDisable", function()
    M.detach(bufnr)
  end, { desc = "Disable fhir.nvim for this buffer" })
  local goto_key = config.get().keymaps.goto_reference
  if goto_key then
    vim.keymap.set("n", goto_key, function()
      require("fhir.features.goto").run()
    end, { buffer = bufnr, desc = "fhir: go to reference" })
  end
  local usages_key = config.get().keymaps.find_usages
  if usages_key then
    vim.keymap.set("n", usages_key, function()
      require("fhir.features.usages").run()
    end, { buffer = bufnr, desc = "fhir: find usages" })
  end
end

-- Tear down: remove commands/keymaps, clear state, drop the cached index.
function M.detach(bufnr)
  vim.b[bufnr].fhir_attached = nil
  pcall(vim.api.nvim_buf_del_user_command, bufnr, "FhirGoto")
  pcall(vim.api.nvim_buf_del_user_command, bufnr, "FhirUsages")
  pcall(vim.api.nvim_buf_del_user_command, bufnr, "FhirDisable")
  local goto_key = config.get().keymaps.goto_reference
  if goto_key then
    pcall(vim.keymap.del, "n", goto_key, { buffer = bufnr })
  end
  local usages_key = config.get().keymaps.find_usages
  if usages_key then
    pcall(vim.keymap.del, "n", usages_key, { buffer = bufnr })
  end
  require("fhir.index").clear(bufnr)
end

-- Detection autocmds: auto-attach FHIR json buffers, rebuild on save, tear down on unload.
function M.setup_autocmds()
  local group = vim.api.nvim_create_augroup("fhir", { clear = true })
  vim.api.nvim_create_autocmd({ "BufReadPost", "BufNewFile" }, {
    group = group,
    pattern = { "*.json", "*.fhir.json" },
    callback = function(args)
      if config.get().detection == "auto" and M.is_fhir(args.buf) then
        M.attach(args.buf)
      end
    end,
  })
  vim.api.nvim_create_autocmd("BufWritePost", {
    group = group,
    pattern = { "*.json", "*.fhir.json" },
    callback = function(args)
      if vim.b[args.buf].fhir_attached then
        require("fhir.index").clear(args.buf)
      end
    end,
  })
  vim.api.nvim_create_autocmd("BufUnload", {
    group = group,
    pattern = { "*.json", "*.fhir.json" },
    callback = function(args)
      if vim.b[args.buf].fhir_attached then
        M.detach(args.buf)
      end
    end,
  })
end

return M
