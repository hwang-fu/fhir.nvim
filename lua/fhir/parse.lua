local M = {}

-- Decode a `string` node into its unescaped Lua string value, or nil.
-- Decision 2 forbids decoding the whole document; unescaping a single scalar
-- string via vim.json.decode is idiomatic and fine.
local function decode_string(node, bufnr)
  if not node or node:type() ~= "string" then
    return nil
  end
  local ok, val = pcall(vim.json.decode, vim.treesitter.get_node_text(node, bufnr))
  if ok and type(val) == "string" then
    return val
  end
  return nil
end

-- Top-level value node (object/array) of the buffer's JSON, or nil when the
-- parser is unavailable or the buffer holds no value (soft-dep degradation).
function M.root(bufnr)
  local ok, parser = pcall(vim.treesitter.get_parser, bufnr, "json")
  if not ok or not parser then
    return nil
  end
  local tree = parser:parse()[1]
  if not tree then
    return nil
  end
  return tree:root():named_child(0)
end

-- The value node for `key` within an object node, or nil if absent.
function M.value_node(obj, key, bufnr)
  if not obj then
    return nil
  end
  for child in obj:iter_children() do
    if child:type() == "pair" and decode_string(child:field("key")[1], bufnr) == key then
      return child:field("value")[1]
    end
  end
  return nil
end

-- The unescaped string value for `key`, or nil if absent / not a string.
function M.string_value(obj, key, bufnr)
  return decode_string(M.value_node(obj, key, bufnr), bufnr)
end

-- Iterator over the named element nodes of an array node.
function M.iter_array(node)
  local i = 0
  return function()
    if not node then
      return nil
    end
    local child = node:named_child(i)
    i = i + 1
    return child
  end
end

return M
