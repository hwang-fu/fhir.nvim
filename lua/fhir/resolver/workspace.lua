local local_buffer = require("fhir.resolver.local_buffer")
local ui = require("fhir.ui")
local workspace = require("fhir.workspace")

local M = {}

-- fhir.index requires fhir.resolver, so this module must reach index
-- lazily or the requires would cycle
local function buffer_index(buf)
  return require("fhir.index").get(buf)
end

-- Open a file into a loaded (not displayed) buffer ready for indexing.
-- Filetype is set when empty so treesitter works without ftdetect.
function M.open_file(path)
  local buf = vim.fn.bufadd(path)
  vim.fn.bufload(buf)
  if vim.bo[buf].filetype == "" then
    vim.bo[buf].filetype = "json"
  end
  return buf
end

-- The workspace files declaring the occurrence's identity, and the key
-- they were found under (absolute references may match by their tail).
local function candidates(occ, ws)
  local files = ws.by_identity[occ.raw]
  if files then
    return files, occ.raw
  end
  if occ.flavor == "absolute" then
    local tail = occ.raw:match("(%a[%w_]*/[^/]+)$")
    if tail and ws.by_identity[tail] then
      return ws.by_identity[tail], tail
    end
  end
  return nil, nil
end

-- Resolve across the workspace: local buffer first; contained references
-- never leave their file.
function M.resolve_resource(occ, idx)
  local res = local_buffer.resolve_resource(occ, idx)
  if res or not occ or occ.flavor == "contained" then
    return res
  end
  local file = vim.api.nvim_buf_get_name(idx.bufnr)
  if file == "" then
    return nil -- an unnamed buffer belongs to no workspace
  end
  local ws = workspace.index(workspace.root(file))
  local files, key = candidates(occ, ws)
  if not files or #files == 0 then
    return nil
  end
  if #files > 1 then
    ui.notify(
      ("%d workspace matches for %s; using %s"):format(#files, occ.raw, vim.fs.basename(files[1])),
      vim.log.levels.INFO
    )
  end
  local buf = M.open_file(files[1])
  return buffer_index(buf).by_identity[key]
end

function M.resolve(occ, idx)
  if occ and occ.flavor == "contained" then
    return local_buffer.resolve(occ, idx)
  end
  local res = M.resolve_resource(occ, idx)
  return res and res.location or nil
end

return M
