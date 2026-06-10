local config = require("fhir.config")

local M = {}

-- Indirection so tests can simulate a missing module.
M._require = require

local mod, failed

-- The directory this plugin lives in, derived from this file's own path.
local function plugin_root()
  return debug.getinfo(1, "S").source:match("^@(.*)lua/fhir/native%.lua$")
end

-- Candidate engine directories, highest priority first: an explicit config
-- dir, the pinned release in the data dir, then a source build in the repo.
function M._dirs()
  local dirs = {}
  local cfg = config.get().native
  if cfg.dir then
    dirs[#dirs + 1] = cfg.dir
  end
  dirs[#dirs + 1] = require("fhir.fetch").dir(cfg.tag)
  local root = plugin_root()
  if root then
    dirs[#dirs + 1] = root .. ".tests"
  end
  return dirs
end

local function load()
  if mod or failed then
    return mod
  end
  -- build the prefix in one piece so the priority order survives prepending
  local entries = {}
  for _, dir in ipairs(M._dirs()) do
    if not package.cpath:find(dir, 1, true) then
      entries[#entries + 1] = dir .. "/?.so"
    end
  end
  if #entries > 0 then
    package.cpath = table.concat(entries, ";") .. ";" .. package.cpath
  end
  local ok, m = pcall(M._require, "fhir_core")
  if ok then
    mod = m
  else
    failed = true
  end
  return mod
end

function M.available()
  return load() ~= nil
end

--- Evaluate a FHIRPath expression against a resource's JSON text.
--- Returns the result as a JSON array string, or nil + message.
function M.eval(json, expr, resolver)
  local m = load()
  if not m then
    return nil, "FHIRPath engine not available (run `make build`)"
  end
  return m.eval(json, expr, resolver)
end

function M._reset()
  mod, failed = nil, nil
end

return M
