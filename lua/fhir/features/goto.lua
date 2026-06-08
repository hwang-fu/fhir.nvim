local index = require("fhir.index")
local resolver = require("fhir.resolver")
local ui = require("fhir.ui")

local M = {}

-- Is (row, col) within `range` {srow, scol, erow, ecol}? Start-inclusive, end-col-exclusive.
local function range_contains(range, row, col)
  local srow, scol, erow, ecol = range[1], range[2], range[3], range[4]
  if row < srow or row > erow then
    return false
  end
  if row == srow and col < scol then
    return false
  end
  if row == erow and col >= ecol then
    return false
  end
  return true
end

-- Resolve the reference under the cursor and jump to it, or notify if unresolved.
function M.run()
  local buf = vim.api.nvim_get_current_buf()
  local pos = vim.api.nvim_win_get_cursor(0)
  local row, col = pos[1] - 1, pos[2] -- to 0-indexed (TS) coordinates
  local idx = index.get(buf)
  for _, occ in ipairs(idx.references) do
    if range_contains(occ.location.range, row, col) then
      local target = resolver.resolve(occ, idx)
      if target then
        ui.jump_to(target)
      else
        ui.notify("unresolved reference: " .. occ.raw, vim.log.levels.WARN)
      end
      return
    end
  end
end

return M
