local parse = require("fhir.parse")

local M = {}

-- Per-bufnr cache of built indexes.
local cache = {}

local function classify_flavor(raw)
  if raw:sub(1, 1) == "#" then
    return "contained"
  elseif raw:match("^urn:uuid:") then
    return "urn-uuid"
  elseif raw:match("^https?://") then
    return "absolute"
  elseif raw:match("^%a[%w_]*/.+") then
    return "relative"
  end
  return "unknown"
end

local function location(node, bufnr)
  local srow, scol, erow, ecol = node:range()
  return { bufnr = bufnr, range = { srow, scol, erow, ecol } }
end

-- Collect every {"reference": "..."} under `node`, tagging each with `owner`.
local function collect_references(node, bufnr, owner, out)
  if not node then
    return
  end
  local t = node:type()
  if t == "object" then
    local refval = parse.value_node(node, "reference", bufnr)
    if refval and refval:type() == "string" then
      local raw = parse.string_value(node, "reference", bufnr)
      if raw then
        out[#out + 1] = {
          raw = raw,
          location = location(refval, bufnr),
          flavor = classify_flavor(raw),
          owner = owner,
        }
      end
    end
    for child in node:iter_children() do
      if child:type() == "pair" then
        collect_references(child:field("value")[1], bufnr, owner, out)
      end
    end
  elseif t == "array" then
    for el in parse.iter_array(node) do
      collect_references(el, bufnr, owner, out)
    end
  end
end

-- Index one resource object: register its identities, contained children, and references.
local function index_resource(obj, full_url, bufnr, idx)
  local res = {
    resource_type = parse.string_value(obj, "resourceType", bufnr),
    id = parse.string_value(obj, "id", bufnr),
    full_url = full_url,
    location = location(obj, bufnr),
    contained = {},
  }
  idx.resources[#idx.resources + 1] = res
  if res.resource_type and res.id then
    idx.by_identity[res.resource_type .. "/" .. res.id] = res
  end
  if full_url then
    idx.by_identity[full_url] = res
  end

  local contained = parse.value_node(obj, "contained", bufnr)
  for el in parse.iter_array(contained) do
    local cid = parse.string_value(el, "id", bufnr)
    if cid then
      res.contained[cid] = location(el, bufnr)
    end
  end

  collect_references(obj, bufnr, res, idx.references)
end

local function empty_index(bufnr)
  return { bufnr = bufnr, resources = {}, by_identity = {}, references = {} }
end

local function build(bufnr)
  local idx = empty_index(bufnr)
  local root = parse.root(bufnr)
  if not root then
    return idx
  end
  if parse.string_value(root, "resourceType", bufnr) == "Bundle" then
    for entry in parse.iter_array(parse.value_node(root, "entry", bufnr)) do
      local resource = parse.value_node(entry, "resource", bufnr)
      if resource then
        index_resource(resource, parse.string_value(entry, "fullUrl", bufnr), bufnr, idx)
      end
    end
  else
    index_resource(root, nil, bufnr, idx)
  end
  return idx
end

-- Return the index for `bufnr`, building (and caching) it on first use or after an edit.
function M.get(bufnr)
  local tick = vim.api.nvim_buf_get_changedtick(bufnr)
  local cached = cache[bufnr]
  if cached and cached.changedtick == tick then
    return cached
  end
  local ok, idx = pcall(build, bufnr)
  if not ok then
    idx = empty_index(bufnr)
  end
  idx.changedtick = tick
  cache[bufnr] = idx
  return idx
end

return M
