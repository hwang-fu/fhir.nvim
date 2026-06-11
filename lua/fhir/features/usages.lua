local index = require("fhir.index")
local resolver_ws = require("fhir.resolver.workspace")
local ui = require("fhir.ui")
local util = require("fhir.util")
local workspace = require("fhir.workspace")

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

-- Every identity key the buffer index knows for this resource
-- (Type/id plus any bundle fullUrl).
local function aliases_of(idx, res)
  local set = {}
  for key, r in pairs(idx.by_identity) do
    if r == res then
      set[key] = true
    end
  end
  return set
end

-- Occurrences in OTHER workspace files whose reference matches an alias,
-- with exact positions from each file's own buffer index.
local function workspace_items(buf, aliases)
  local file = vim.api.nvim_buf_get_name(buf)
  if file == "" then
    return {} -- an unnamed buffer belongs to no workspace
  end
  local ws = workspace.index(workspace.root(file))
  local referrers = {}
  for path, refs in pairs(ws.references) do
    if path ~= file then
      for _, raw in ipairs(refs) do
        if aliases[raw] then
          referrers[#referrers + 1] = path
          break
        end
      end
    end
  end
  table.sort(referrers)
  local items = {}
  for _, path in ipairs(referrers) do
    local tidx = index.get(resolver_ws.open_file(path))
    for _, occ in ipairs(tidx.references) do
      if aliases[occ.raw] then
        items[#items + 1] = {
          location = occ.location,
          label = string.format(
            "%s/%s (%s:%d)",
            occ.owner.resource_type,
            occ.owner.id or "?",
            vim.fs.basename(path),
            occ.location.range[1] + 1
          ),
        }
      end
    end
  end
  return items
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

  local items = {}
  for _, occ in ipairs(idx.reverse[res] or {}) do
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
  for _, item in ipairs(workspace_items(buf, aliases_of(idx, res))) do
    items[#items + 1] = item
  end
  if #items == 0 then
    ui.notify("no references to this resource", vim.log.levels.INFO)
    return
  end

  ui.select(items, { prompt = "FHIR usages" }, function(item)
    ui.jump_to(item.location)
  end)
end

return M
