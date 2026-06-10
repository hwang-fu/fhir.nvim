local config = require("fhir.config")

local M = {}

-- Indirection so tests can simulate a missing module.
M._require = require

local mod, failed

-- The directory this plugin lives in, derived from this file's own path.
local function plugin_root()
  return debug.getinfo(1, "S").source:match("^@(.*)lua/fhir/native%.lua$")
end

local function load()
  if mod or failed then
    return mod
  end
  local dir = config.get().native.dir
  if not dir then
    local root = plugin_root()
    dir = root and (root .. ".tests")
  end
  if dir and not package.cpath:find(dir, 1, true) then
    package.cpath = dir .. "/?.so;" .. package.cpath
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
