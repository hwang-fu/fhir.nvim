local parse = require("fhir.parse")
local pathmap = require("fhir.pathmap")
local ui = require("fhir.ui")

local M = {}

-- Delete a key/value pair together with one neighboring comma (the next
-- when one follows, else the previous).
local function remove_element(buf, key)
  local pair = key:parent()
  local sr, sc, er, ec = pair:range()
  local nxt = pair:next_sibling()
  if nxt and nxt:type() == "," then
    er, ec = select(3, nxt:range())
  else
    local prev = pair:prev_sibling()
    if prev and prev:type() == "," then
      sr, sc = prev:range()
    end
  end
  vim.api.nvim_buf_set_text(buf, sr, sc, er, ec, {})
end

-- Insert `"name": ""` into the parent object (before its closing brace)
-- and park the cursor on the skeleton, ready to fill in.
local function insert_skeleton(buf, parent, name)
  local close = parent:child(parent:child_count() - 1) -- the "}"
  local sr, sc = close:range()
  local text = ('"%s": ""'):format(name)
  if parent:named_child_count() > 0 then
    text = ", " .. text
  end
  vim.api.nvim_buf_set_text(buf, sr, sc, sr, sc, { text })
  vim.api.nvim_win_set_cursor(0, { sr + 1, sc + #text - 1 })
end

local function rename_key(buf, key, to)
  local sr, sc, er, ec = key:range()
  vim.api.nvim_buf_set_text(buf, sr, sc, er, ec, { ('"%s"'):format(to) })
end

local function wrap_in_array(buf, node)
  local sr, sc, er, ec = node:range()
  -- the later edit first, so the earlier offset stays valid
  vim.api.nvim_buf_set_text(buf, er, ec, er, ec, { "]" })
  vim.api.nvim_buf_set_text(buf, sr, sc, sr, sc, { "[" })
end

local function unwrap_array(buf, node)
  local element = node:named_child(0)
  local text = vim.treesitter.get_node_text(element, buf)
  local sr, sc, er, ec = node:range()
  vim.api.nvim_buf_set_text(buf, sr, sc, er, ec, vim.split(text, "\n"))
end

-- The repairs applicable to one finding: zero or one of the structural
-- fixes, dispatched on category + message. Inapplicable shapes (an
-- unresolved path, a multi-element unwrap, an unparseable suggestion)
-- yield nothing - never a broken apply.
function M.fixes_for(buf, issue)
  local root = parse.root(buf)
  if not root then
    return {}
  end
  local node, key, exact = pathmap.node(root, buf, issue.path)
  local fixes = {}
  local function add(label, apply)
    fixes[#fixes + 1] = { label = label, apply = apply }
  end

  if issue.category == "unknown" then
    if exact and key then
      add("remove the element", function()
        remove_element(buf, key)
      end)
    end
  elseif issue.category == "cardinality" then
    if issue.message:find("is missing", 1, true) then
      local parent_path = issue.path:match("^(.*)%.[^%.]+$")
      local name = issue.path:match("([^%.]+)$")
      if parent_path and name then
        local parent, _, pexact = pathmap.node(root, buf, parent_path)
        if pexact and parent:type() == "object" then
          add(('insert "%s"'):format(name), function()
            insert_skeleton(buf, parent, name)
          end)
        end
      end
    elseif issue.message:find("expected an array", 1, true) then
      if exact and node then
        add("wrap in an array", function()
          wrap_in_array(buf, node)
        end)
      end
    elseif issue.message:find("did not expect an array", 1, true) then
      if exact and node and node:type() == "array" and node:named_child_count() == 1 then
        add("unwrap the array", function()
          unwrap_array(buf, node)
        end)
      end
    elseif issue.message:find("must not be empty", 1, true) then
      if exact and key then
        add("remove the empty element", function()
          remove_element(buf, key)
        end)
      end
    end
  elseif issue.category == "choice" then
    local hint = issue.message:match('%(e%.g%. "([%w_]+)"%)')
    if hint and exact and key then
      add(('rename to "%s"'):format(hint), function()
        rename_key(buf, key, hint)
      end)
    end
  end
  return fixes
end

-- Offer the repairs for the findings on the cursor line: a single fix
-- applies directly, several go through the picker. The re-validation
-- afterwards is the confirmation.
function M.run()
  local buf = vim.api.nvim_get_current_buf()
  local validate = require("fhir.features.validate")
  local row = vim.api.nvim_win_get_cursor(0)[1] - 1
  local diags = vim.diagnostic.get(buf, { namespace = validate.namespace, lnum = row })

  local fixes = {}
  for _, d in ipairs(diags) do
    if d.user_data then
      for _, f in ipairs(M.fixes_for(buf, d.user_data)) do
        fixes[#fixes + 1] = f
      end
    end
  end
  if #fixes == 0 then
    ui.notify("no applicable fix here", vim.log.levels.INFO)
    return
  end

  local function apply(f)
    f.apply()
    validate.run()
  end
  if #fixes == 1 then
    apply(fixes[1])
    return
  end
  ui.select(fixes, { prompt = "FHIR fixes" }, function(choice)
    if choice then
      apply(choice)
    end
  end)
end

return M
