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
  vim.api.nvim_buf_create_user_command(bufnr, "FhirOutline", function()
    require("fhir.features.outline").run()
  end, { desc = "Outline: pick a FHIR resource and jump to it" })
  vim.api.nvim_buf_create_user_command(
    bufnr,
    "FhirEval",
    function(opts)
      require("fhir.features.eval").run(opts.args ~= "" and opts.args or nil)
    end,
    { nargs = "*", desc = "Evaluate a FHIRPath expression against the resource under the cursor" }
  )
  vim.api.nvim_buf_create_user_command(bufnr, "FhirValidate", function(opts)
    local v = require("fhir.features.validate")
    if opts.bang then
      v.clear()
    else
      v.run()
    end
  end, { bang = true, desc = "Validate the buffer's FHIR document (! clears the findings)" })
  vim.api.nvim_buf_create_user_command(bufnr, "FhirDisable", function()
    M.detach(bufnr)
  end, { desc = "Disable fhir.nvim for this buffer" })
  if config.get().validate.on_save then
    vim.api.nvim_create_autocmd("BufWritePost", {
      buffer = bufnr,
      group = vim.api.nvim_create_augroup("fhir.validate." .. bufnr, { clear = true }),
      callback = function()
        require("fhir.features.validate").on_save()
      end,
    })
  end
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
  local outline_key = config.get().keymaps.outline
  if outline_key then
    vim.keymap.set("n", outline_key, function()
      require("fhir.features.outline").run()
    end, { buffer = bufnr, desc = "fhir: outline" })
  end
  local eval_key = config.get().keymaps.eval
  if eval_key then
    vim.keymap.set("n", eval_key, function()
      require("fhir.features.eval").run()
    end, { buffer = bufnr, desc = "fhir: evaluate fhirpath" })
  end
  local diag_key = config.get().keymaps.diagnostics
  if diag_key then
    vim.keymap.set("n", diag_key, function()
      vim.diagnostic.open_float()
    end, { buffer = bufnr, desc = "fhir: finding details" })
  end
end

-- Tear down: remove commands/keymaps, clear state, drop the cached index.
function M.detach(bufnr)
  vim.b[bufnr].fhir_attached = nil
  pcall(vim.api.nvim_buf_del_user_command, bufnr, "FhirGoto")
  pcall(vim.api.nvim_buf_del_user_command, bufnr, "FhirUsages")
  pcall(vim.api.nvim_buf_del_user_command, bufnr, "FhirOutline")
  pcall(vim.api.nvim_buf_del_user_command, bufnr, "FhirEval")
  pcall(vim.api.nvim_buf_del_user_command, bufnr, "FhirValidate")
  pcall(vim.api.nvim_buf_del_user_command, bufnr, "FhirDisable")
  pcall(vim.api.nvim_del_augroup_by_name, "fhir.validate." .. bufnr)
  require("fhir.features.validate").clear(bufnr)
  local goto_key = config.get().keymaps.goto_reference
  if goto_key then
    pcall(vim.keymap.del, "n", goto_key, { buffer = bufnr })
  end
  local usages_key = config.get().keymaps.find_usages
  if usages_key then
    pcall(vim.keymap.del, "n", usages_key, { buffer = bufnr })
  end
  local outline_key = config.get().keymaps.outline
  if outline_key then
    pcall(vim.keymap.del, "n", outline_key, { buffer = bufnr })
  end
  local eval_key = config.get().keymaps.eval
  if eval_key then
    pcall(vim.keymap.del, "n", eval_key, { buffer = bufnr })
  end
  local diag_key = config.get().keymaps.diagnostics
  if diag_key then
    pcall(vim.keymap.del, "n", diag_key, { buffer = bufnr })
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
