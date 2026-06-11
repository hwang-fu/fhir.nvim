local index = require("fhir.index")
local native = require("fhir.native")
local pathmap = require("fhir.pathmap")
local resolver = require("fhir.resolver")
local ui = require("fhir.ui")

local M = {}

local ns = vim.api.nvim_create_namespace("fhir.validate")

local severities = {
  error = vim.diagnostic.severity.ERROR,
  warning = vim.diagnostic.severity.WARN,
  information = vim.diagnostic.severity.INFO,
}

-- Validate the buffer's top-level document and publish the findings.
function M.run()
  local buf = vim.api.nvim_get_current_buf()
  local idx = index.get(buf)
  local res = idx.resources[1]
  if not res then
    ui.notify("no resource in this buffer", vim.log.levels.INFO)
    return
  end

  local json = vim.treesitter.get_node_text(res.node, buf)
  local result, err = native.validate(json, resolver.callback(idx, res))
  if err then
    ui.notify(err, vim.log.levels.ERROR)
    return
  end

  local diags = {}
  for _, issue in ipairs(vim.json.decode(result)) do
    local r = pathmap.range(res.node, buf, issue.path)
    diags[#diags + 1] = {
      lnum = r[1],
      col = r[2],
      end_lnum = r[3],
      end_col = r[4],
      severity = severities[issue.severity] or vim.diagnostic.severity.ERROR,
      message = issue.message,
      source = "fhir",
    }
  end
  vim.diagnostic.set(ns, buf, diags)
end

-- Drop a buffer's findings (the current buffer when unspecified).
function M.clear(buf)
  vim.diagnostic.reset(ns, buf or vim.api.nvim_get_current_buf())
end

-- The write hook: quietly does nothing when the engine is absent.
function M.on_save()
  if not native.available() then
    return
  end
  M.run()
end

return M
