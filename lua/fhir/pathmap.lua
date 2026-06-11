local M = {}

-- "name[1]" -> "name", 1; "name" -> "name", nil
local function parse_segment(seg)
  local name, idx = seg:match("^([%w_]+)%[(%d+)%]$")
  if name then
    return name, tonumber(idx)
  end
  return seg, nil
end

-- The value node and key node of `key` within an object node, or nil.
local function field(node, key, bufnr)
  local quoted = '"' .. key .. '"'
  for pair in node:iter_children() do
    if pair:type() == "pair" then
      local k = pair:field("key")[1]
      if k and vim.treesitter.get_node_text(k, bufnr) == quoted then
        return pair:field("value")[1], k
      end
    end
  end
end

-- The i-th (0-based) element of an array node, or nil.
local function item(node, i)
  local n = 0
  for child in node:iter_children() do
    if child:named() then
      if n == i then
        return child
      end
      n = n + 1
    end
  end
end

-- Resolves an element path with indices ("Patient.name[0].given[2]") to
-- its treesitter nodes: the value node, its key node (nil for array
-- elements and the root), and whether every segment resolved. On a miss
-- the deepest resolved node is returned. `root` is the resource's own
-- object node; the leading type segment names it and is skipped.
function M.node(root, bufnr, path)
  local node, key = root, nil
  local exact = true
  local first = true
  for seg in path:gmatch("[^%.]+") do
    if first then
      first = false
    else
      local name, idx = parse_segment(seg)
      local value, k = field(node, name, bufnr)
      if not value then
        exact = false
        break
      end
      node, key = value, k
      if idx then
        local element = item(node, idx)
        if not element then
          exact = false
          break
        end
        node, key = element, nil -- an array element has no key of its own
      end
    end
  end
  return node, key, exact
end

-- The buffer range for a path: the element's key when it resolves (a
-- tight underline), the deepest existing ancestor when it does not (e.g.
-- a missing required element lands on its parent), the resource's first
-- line as a last resort. Returns { start_row, start_col, end_row, end_col }.
function M.range(root, bufnr, path)
  local node, key = M.node(root, bufnr, path)
  local sr, sc, er, ec = (key or node):range()
  return { sr, sc, er, ec }
end

return M
