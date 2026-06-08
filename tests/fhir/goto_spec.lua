local h = require("tests.helpers")
local goto_ref = require("fhir.features.goto")

-- Put the cursor inside the first reference occurrence of `flavor`; returns buf, occ.
local function on_reference(fixture, flavor)
  local index = require("fhir.index")
  local buf = h.fixture_buf(fixture)
  vim.api.nvim_win_set_buf(0, buf)
  local occ
  for _, r in ipairs(index.get(buf).references) do
    if r.flavor == flavor then
      occ = r
      break
    end
  end
  vim.api.nvim_win_set_cursor(0, { occ.location.range[1] + 1, occ.location.range[2] + 1 })
  return buf, occ
end

describe("goto", function()
  it("jumps to the target resource when on a resolvable reference", function()
    local buf = on_reference("bundle_urn.json", "urn-uuid")
    goto_ref.run()
    local target = require("fhir.index").get(buf).by_identity["urn:uuid:p1"].location
    assert.are.same({ target.range[1] + 1, target.range[2] }, vim.api.nvim_win_get_cursor(0))
  end)

  it("notifies (no move, no throw) on an unresolvable reference", function()
    on_reference("unresolvable.json", "relative")
    local before = vim.api.nvim_win_get_cursor(0)
    assert.has_no.errors(function()
      goto_ref.run()
    end)
    assert.are.same(before, vim.api.nvim_win_get_cursor(0))
  end)

  it("is a quiet no-op when the cursor is not on a reference", function()
    local buf = h.fixture_buf("bundle_urn.json")
    vim.api.nvim_win_set_buf(0, buf)
    vim.api.nvim_win_set_cursor(0, { 1, 0 }) -- not on a reference
    local before = vim.api.nvim_win_get_cursor(0)
    assert.has_no.errors(function()
      goto_ref.run()
    end)
    assert.are.same(before, vim.api.nvim_win_get_cursor(0))
  end)
end)
