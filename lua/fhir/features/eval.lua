local index = require("fhir.index")
local native = require("fhir.native")
local resolver = require("fhir.resolver")
local ui = require("fhir.ui")
local util = require("fhir.util")

local M = {}

-- The indexed resource whose range contains (row, col), or nil.
-- Top-level resources don't overlap, so the first match is the answer.
local function resource_at(idx, row, col)
  for _, res in ipairs(idx.resources) do
    if util.range_contains(res.location.range, row, col) then
      return res
    end
  end
  return nil
end

-- Evaluate `expr` against the resource under the cursor; prompt when absent.
function M.run(expr)
  if not expr or expr == "" then
    vim.ui.input({ prompt = "FHIRPath: " }, function(input)
      if input and input ~= "" then
        M.run(input)
      end
    end)
    return
  end

  local buf = vim.api.nvim_get_current_buf()
  local pos = vim.api.nvim_win_get_cursor(0)
  local idx = index.get(buf)
  local res = resource_at(idx, pos[1] - 1, pos[2]) or idx.resources[1]
  if not res then
    ui.notify("no resource in this buffer", vim.log.levels.INFO)
    return
  end

  local json = vim.treesitter.get_node_text(res.node, buf)
  local result, err = native.eval(json, expr, resolver.callback(idx, res))
  if err then
    ui.notify(err, vim.log.levels.ERROR)
    return
  end

  local lines = {}
  for _, v in ipairs(vim.json.decode(result)) do
    lines[#lines + 1] = vim.json.encode(v)
  end
  if #lines == 0 then
    lines = { "(empty)" }
  end
  local title = res.resource_type or "resource"
  if res.id then
    title = title .. "/" .. res.id
  end
  ui.float(lines, { ft = "json", title = title })
end

return M
