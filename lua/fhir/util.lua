local M = {}

-- Is (row, col) within `range` {srow, scol, erow, ecol}? Start-inclusive, end-col-exclusive.
function M.range_contains(range, row, col)
  local srow, scol, erow, ecol = range[1], range[2], range[3], range[4]
  if row < srow or row > erow then
    return false
  end
  if row == srow and col < scol then
    return false
  end
  if row == erow and col >= ecol then
    return false
  end
  return true
end

return M
