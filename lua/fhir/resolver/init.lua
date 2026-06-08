local M = {}

-- The active resolver. v1 ships LocalBufferResolver; v4 (workspace) / v5 (server)
-- swap this out behind the same resolve(occ, idx) -> Location? interface.
local active = require("fhir.resolver.local_buffer")

function M.resolve(occ, idx)
  return active.resolve(occ, idx)
end

return M
