local h = require("tests.helpers")
local usages = require("fhir.features.usages")
local index = require("fhir.index")

local function cursor_in_resource(buf, identity)
  vim.api.nvim_win_set_buf(0, buf)
  local res = index.get(buf).by_identity[identity]
  vim.api.nvim_win_set_cursor(0, { res.location.range[1] + 1, res.location.range[2] + 1 })
  return res
end

describe("usages", function()
  it("lists usages of the resource under the cursor and jumps on selection", function()
    local buf = h.fixture_buf("bundle_urn.json")
    local patient = cursor_in_resource(buf, "Patient/p1")
    local orig = vim.ui.select
    vim.ui.select = function(list, _, cb)
      cb(list[1])
    end
    usages.run()
    vim.ui.select = orig
    local occ = index.get(buf).reverse[patient][1]
    assert.are.same(
      { occ.location.range[1] + 1, occ.location.range[2] },
      vim.api.nvim_win_get_cursor(0)
    )
  end)

  it("notifies when the resource has no usages", function()
    local buf = h.fixture_buf("bundle_urn.json")
    cursor_in_resource(buf, "Observation/o1") -- nothing references o1
    local msg
    local orig = vim.notify
    vim.notify = function(m)
      msg = m
    end
    pcall(usages.run)
    vim.notify = orig
    assert.is_not_nil(msg)
    assert.is_not_nil(msg:match("[Nn]o "))
  end)

  it("notifies when the cursor is not on a resource", function()
    local buf = h.fixture_buf("bundle_urn.json")
    vim.api.nvim_win_set_buf(0, buf)
    vim.api.nvim_win_set_cursor(0, { 1, 0 }) -- on "{" / Bundle, not an indexed resource
    local msg
    local orig = vim.notify
    vim.notify = function(m)
      msg = m
    end
    pcall(usages.run)
    vim.notify = orig
    assert.is_not_nil(msg)
  end)
end)
