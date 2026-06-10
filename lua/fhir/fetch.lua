local M = {}

-- Indirection for tests.
M._uname = vim.uv.os_uname

local OS = { Linux = "linux", Darwin = "macos" }
local ARCH = { x86_64 = "x86_64", aarch64 = "aarch64", arm64 = "aarch64" }

-- The release asset name for this machine, or nil when no prebuilt exists.
function M.asset()
  local u = M._uname()
  local os_name, arch = OS[u.sysname], ARCH[u.machine]
  if not (os_name and arch) then
    return nil
  end
  return ("fhir_core-%s-%s.so"):format(os_name, arch)
end

-- Where a given engine release lives locally.
function M.dir(tag)
  return vim.fn.stdpath("data") .. "/fhir.nvim/" .. tag
end

return M
