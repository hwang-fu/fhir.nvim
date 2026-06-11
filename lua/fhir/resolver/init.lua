local M = {}

-- The active resolver, behind the same resolve(occ, idx) -> Location?
-- interface: v1 shipped LocalBufferResolver, v4 wraps it with workspace
-- fall-through; v5 (server) swaps in the same way.
local active = require("fhir.resolver.workspace")

function M.resolve(occ, idx)
  return active.resolve(occ, idx)
end

function M.resolve_resource(occ, idx)
  return active.resolve_resource(occ, idx)
end

-- The resolve() callback handed across to the engine: a raw reference string
-- in, the target resource's JSON text (or nil) back. `owner` makes contained
-- (#id) references resolvable.
function M.callback(idx, owner)
  return function(ref)
    local occ = { raw = ref, flavor = require("fhir.index").flavor(ref), owner = owner }
    local loc = M.resolve(occ, idx)
    if not loc then
      return nil
    end
    local r = loc.range
    local text = vim.api.nvim_buf_get_text(loc.bufnr, r[1], r[2], r[3], r[4], {})
    return table.concat(text, "\n")
  end
end

return M
