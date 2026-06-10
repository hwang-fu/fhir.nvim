local config = require("fhir.config")
local ui = require("fhir.ui")

local M = {}

-- Indirection for tests.
M._uname = vim.uv.os_uname
M._system = vim.system
M._data_root = nil

-- Platforms a release ships prebuilts for (intel macs build from source).
local PLATFORMS = {
  ["Linux/x86_64"] = "linux-x86_64",
  ["Linux/aarch64"] = "linux-aarch64",
  ["Darwin/arm64"] = "macos-aarch64",
}

-- The release asset name for this machine, or nil when no prebuilt exists.
function M.asset()
  local u = M._uname()
  local platform = PLATFORMS[u.sysname .. "/" .. u.machine]
  return platform and ("fhir_core-%s.so"):format(platform)
end

-- Where a given engine release lives locally.
function M.dir(tag)
  return (M._data_root or vim.fn.stdpath("data")) .. "/fhir.nvim/" .. tag
end

local function get(url, path)
  return M._system({ "curl", "-fsSL", "--retry", "2", "-o", path, url }):wait().code == 0
end

local function read_all(path)
  local f = io.open(path, "rb")
  if not f then
    return nil
  end
  local s = f:read("*a")
  f:close()
  return s
end

-- Download the engine for this platform from the given release tag (defaults
-- to the pinned one), verify it against the release's SHA256SUMS, and install
-- it atomically into the versioned data directory.
function M.run(tag)
  tag = (tag and tag ~= "") and tag or config.get().native.tag
  local asset = M.asset()
  if not asset then
    ui.notify(
      "no prebuilt engine for this platform; build from source with `make build`",
      vim.log.levels.WARN
    )
    return
  end

  local base = ("https://github.com/hwang-fu/fhir.nvim/releases/download/%s/"):format(tag)
  local dest = M.dir(tag)
  vim.fn.mkdir(dest, "p")
  local sums_partial = dest .. "/SHA256SUMS.partial"
  local so_partial = dest .. "/fhir_core.so.partial"

  if not get(base .. "SHA256SUMS", sums_partial) or not get(base .. asset, so_partial) then
    os.remove(sums_partial)
    os.remove(so_partial)
    ui.notify(
      ("engine download failed for %s (is the release published?)"):format(tag),
      vim.log.levels.ERROR
    )
    return
  end

  local sums = read_all(sums_partial) or ""
  os.remove(sums_partial)
  local want = sums:match("(%x+)%s+" .. vim.pesc(asset))
  local got = vim.fn.sha256(read_all(so_partial) or "")
  if not want or want ~= got then
    os.remove(so_partial)
    ui.notify("engine checksum mismatch; nothing installed", vim.log.levels.ERROR)
    return
  end

  -- same directory, same filesystem: the rename is atomic
  os.rename(so_partial, dest .. "/fhir_core.so")
  require("fhir.native")._reset()
  ui.notify(("engine %s installed"):format(tag))
end

return M
