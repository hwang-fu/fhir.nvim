local index = require("fhir.index")
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

-- List references to the resource under the cursor and jump to a chosen one.
function M.run()
  local buf = vim.api.nvim_get_current_buf()
  local pos = vim.api.nvim_win_get_cursor(0)
  local row, col = pos[1] - 1, pos[2] -- to 0-indexed (TS) coordinates
  local idx = index.get(buf)

  local res = resource_at(idx, row, col)
  if not res then
    ui.notify("no resource under the cursor", vim.log.levels.INFO)
    return
  end

  local refs = idx.reverse[res]
  if not refs or #refs == 0 then
    ui.notify("no references to this resource", vim.log.levels.INFO)
    return
  end

  local items = {}
  for _, occ in ipairs(refs) do
    items[#items + 1] = {
      location = occ.location,
      label = string.format(
        "%s/%s (line %d)",
        occ.owner.resource_type,
        occ.owner.id or "?",
        occ.location.range[1] + 1
      ),
    }
  end

  ui.select(items, { prompt = "FHIR usages" }, function(item)
    ui.jump_to(item.location)
  end)
end

return M
