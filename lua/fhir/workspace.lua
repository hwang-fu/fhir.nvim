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

-- Per-file records cached by mtime: only changed files re-decode.
local records = {}

-- Identities a decoded document exposes: the root Type/id, plus each
-- Bundle entry's fullUrl and entry resource Type/id.
local function identities_of(doc)
  local ids = {}
  if type(doc.id) == "string" then
    ids[#ids + 1] = doc.resourceType .. "/" .. doc.id
  end
  if doc.resourceType == "Bundle" and type(doc.entry) == "table" then
    for _, e in ipairs(doc.entry) do
      if type(e) == "table" then
        if type(e.fullUrl) == "string" then
          ids[#ids + 1] = e.fullUrl
        end
        local r = e.resource
        if type(r) == "table" and type(r.resourceType) == "string" and type(r.id) == "string" then
          ids[#ids + 1] = r.resourceType .. "/" .. r.id
        end
      end
    end
  end
  return ids
end

-- Every reference string in the document, except contained (#) ones -
-- those can never point outside their own file.
local function references_of(value, out)
  if type(value) ~= "table" then
    return out
  end
  for k, v in pairs(value) do
    if k == "reference" and type(v) == "string" then
      if not vim.startswith(v, "#") then
        out[#out + 1] = v
      end
    else
      references_of(v, out)
    end
  end
  return out
end

-- nil when the file is unreadable, unparseable, or not a FHIR resource.
local function record(path)
  local ok_read, lines = pcall(vim.fn.readfile, path)
  if not ok_read then
    return nil
  end
  local ok, doc = pcall(vim.json.decode, table.concat(lines, "\n"))
  if not ok or type(doc) ~= "table" or type(doc.resourceType) ~= "string" then
    return nil
  end
  return { identities = identities_of(doc), references = references_of(doc, {}) }
end

-- The identity/reference index for a root. Candidates are listed fresh
-- each call; per-file records are reused unless the file's mtime moved.
function M.index(root)
  local files, clipped = M.files(root)
  local by_identity, references = {}, {}
  local skipped = 0
  for _, f in ipairs(files) do
    local stat = vim.uv.fs_stat(f)
    local mtime = stat and stat.mtime.sec or -1
    local cached = records[f]
    if not cached or cached.mtime ~= mtime then
      cached = { mtime = mtime, data = record(f) }
      records[f] = cached
    end
    if cached.data then
      for _, id in ipairs(cached.data.identities) do
        local list = by_identity[id] or {}
        list[#list + 1] = f
        by_identity[id] = list
      end
      if #cached.data.references > 0 then
        references[f] = cached.data.references
      end
    else
      skipped = skipped + 1
    end
  end
  for _, list in pairs(by_identity) do
    table.sort(list) -- deterministic collision order
  end
  return {
    by_identity = by_identity,
    references = references,
    skipped = skipped,
    clipped = clipped,
  }
end

-- Drop cached state (the test seam).
function M._reset()
  records = {}
end

return M
