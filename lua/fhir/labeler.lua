local parse = require("fhir.parse")

local M = {}

-- First named element of an array node, or nil. iter_array returns an iterator,
-- so calling it once yields the first element (or nil for an empty/absent array).
local function first(array_node)
  return parse.iter_array(array_node)()
end

-- code.text, else code.coding[0].display.
local function code_label(node, bufnr)
  local code = parse.value_node(node, "code", bufnr)
  if not code then
    return nil
  end
  local text = parse.string_value(code, "text", bufnr)
  if text then
    return text
  end
  local coding = first(parse.value_node(code, "coding", bufnr))
  return coding and parse.string_value(coding, "display", bufnr) or nil
end

-- name[0].text, else "<given> <family>".
local function name_label(node, bufnr)
  local name = first(parse.value_node(node, "name", bufnr))
  if not name then
    return nil
  end
  local text = parse.string_value(name, "text", bufnr)
  if text then
    return text
  end
  local family = parse.string_value(name, "family", bufnr)
  local given = parse.node_string(first(parse.value_node(name, "given", bufnr)), bufnr)
  if given and family then
    return given .. " " .. family
  end
  return given or family
end

-- Human-readable label for a resource, with a Type/id fallback.
function M.label(res, bufnr)
  local human = code_label(res.node, bufnr) or name_label(res.node, bufnr)
  if human then
    return string.format("[%s] %s (%s)", res.resource_type, human, res.id or "?")
  end
  return string.format("%s/%s", res.resource_type or "?", res.id or "?")
end

return M
