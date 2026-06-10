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

-- The resolve() callback handed across to the engine: a raw reference string
-- in, the target resource's JSON text (or nil) back. `owner` makes contained
-- (#id) references resolvable.
local function make_resolver(idx, owner)
  return function(ref)
    local occ = { raw = ref, flavor = index.flavor(ref), owner = owner }
    local loc = resolver.resolve(occ, idx)
    if not loc then
      return nil
    end
    local r = loc.range
    local text = vim.api.nvim_buf_get_text(loc.bufnr, r[1], r[2], r[3], r[4], {})
    return table.concat(text, "\n")
  end
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
  local result, err = native.eval(json, expr, make_resolver(idx, res))
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
  ui.float(lines, { ft = "json" })
end

return M
