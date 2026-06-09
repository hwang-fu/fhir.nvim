local index = require("fhir.index")
local labeler = require("fhir.labeler")
local ui = require("fhir.ui")

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

return M
