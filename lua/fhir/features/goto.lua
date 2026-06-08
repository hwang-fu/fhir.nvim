local index = require("fhir.index")
local resolver = require("fhir.resolver")
local ui = require("fhir.ui")
local util = require("fhir.util")

local M = {}

-- Resolve the reference under the cursor and jump to it, or notify if unresolved.
function M.run()
  local buf = vim.api.nvim_get_current_buf()
  local pos = vim.api.nvim_win_get_cursor(0)
  local row, col = pos[1] - 1, pos[2] -- to 0-indexed (TS) coordinates
  local idx = index.get(buf)
  for _, occ in ipairs(idx.references) do
    if util.range_contains(occ.location.range, row, col) then
      local target = resolver.resolve(occ, idx)
      if target then
        ui.jump_to(target)
      else
        ui.notify("unresolved reference: " .. occ.raw, vim.log.levels.WARN)
      end
      return
    end
  end
  ui.notify("no reference under the cursor", vim.log.levels.INFO)
end

return M
