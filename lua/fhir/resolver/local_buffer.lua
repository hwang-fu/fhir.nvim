local M = {}

-- Resolve a reference occurrence to its target resource, or nil if it points
-- outside the buffer / is a contained or unsupported flavor.
function M.resolve_resource(occ, idx)
  if not occ then
    return nil
  end
  local raw = occ.raw
  if occ.flavor == "urn-uuid" or occ.flavor == "relative" then
    return idx.by_identity[raw]
  elseif occ.flavor == "absolute" then
    local res = idx.by_identity[raw]
    if not res then
      local tail = raw:match("(%a[%w_]*/[^/]+)$")
      res = tail and idx.by_identity[tail] or nil
    end
    return res
  end
  return nil
end

-- Resolve a reference occurrence to a target Location within the same buffer,
-- or nil if it points outside the buffer / is an unsupported flavor.
function M.resolve(occ, idx)
  if occ and occ.flavor == "contained" then
    return occ.owner and occ.owner.contained[occ.raw:sub(2)] or nil
  end
  local res = M.resolve_resource(occ, idx)
  return res and res.location or nil
end

return M
