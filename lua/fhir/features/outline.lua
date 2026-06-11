local index = require("fhir.index")
local labeler = require("fhir.labeler")
local resolver_ws = require("fhir.resolver.workspace")
local ui = require("fhir.ui")
local workspace = require("fhir.workspace")

local M = {}

-- List every resource in the buffer and jump to the chosen one.
function M.run()
  local buf = vim.api.nvim_get_current_buf()
  local idx = index.get(buf)

  local items = {}
  for _, res in ipairs(idx.resources) do
    items[#items + 1] = {
      location = res.location,
      label = labeler.label(res, buf),
    }
  end

  if #items == 0 then
    ui.notify("no resources in this buffer", vim.log.levels.INFO)
    return
  end

  ui.select(items, { prompt = "FHIR outline" }, function(item)
    ui.jump_to(item.location)
  end)
end

-- List every resource under the workspace root and jump to the chosen one.
function M.run_workspace()
  local file = vim.api.nvim_buf_get_name(0)
  local root = workspace.root(file ~= "" and file or vim.uv.cwd())
  local ws = workspace.index(root)

  if #ws.resources == 0 then
    ui.notify("no resources in the workspace", vim.log.levels.INFO)
    return
  end

  ui.select(ws.resources, { prompt = "FHIR workspace outline" }, function(item)
    local buf = resolver_ws.open_file(item.file)
    local res = index.get(buf).by_identity[item.identity]
    -- the identity came from this file's record, but fall back to its top
    -- if the file changed between indexing and the jump
    ui.jump_to(res and res.location or { bufnr = buf, range = { 0, 0 } })
  end)
end

return M
