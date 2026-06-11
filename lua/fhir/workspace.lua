local config = require("fhir.config")
local ui = require("fhir.ui")

local M = {}

-- The workspace root for a path: the enclosing git repository, else the cwd.
function M.root(path)
  return vim.fs.root(path, ".git") or vim.uv.cwd()
end

-- Does the directory `dir` (under `root`) contain an ignored component?
local function ignored(dir, root, ignore)
  local rel = dir:sub(#root + 2)
  for part in rel:gmatch("[^/]+") do
    for _, name in ipairs(ignore) do
      if part == name then
        return true
      end
    end
  end
  return false
end

-- Candidate json files under `root`, ignore-filtered and bounded by
-- workspace.max_files (clipping is reported, never silent).
-- Returns the sorted list and whether it was clipped.
function M.files(root)
  local cfg = config.get().workspace
  local found = vim.fs.find(function(name, dir)
    return name:match("%.json$") ~= nil and not ignored(dir, root, cfg.ignore)
  end, { path = root, type = "file", limit = cfg.max_files + 1 })
  local clipped = #found > cfg.max_files
  if clipped then
    found[#found] = nil
    ui.notify(
      ("workspace clipped to %d files; raise workspace.max_files to cover more"):format(
        cfg.max_files
      ),
      vim.log.levels.WARN
    )
  end
  table.sort(found)
  return found, clipped
end

-- Drop cached state (the test seam).
function M._reset() end

return M
