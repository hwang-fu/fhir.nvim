local h = require("tests.helpers")
local outline = require("fhir.features.outline")
local index = require("fhir.index")

describe("outline", function()
  it("lists every resource with labels and jumps to the chosen one", function()
    local buf = h.fixture_buf("bundle_urn.json")
    vim.api.nvim_win_set_buf(0, buf)
    local seen
    local orig = vim.ui.select
    vim.ui.select = function(list, _, cb)
      seen = list
      cb(list[2]) -- pick the Observation (document order: Patient, Observation)
    end
    outline.run()
    vim.ui.select = orig

    assert.are.equal(2, #seen)
    assert.are.equal("[Patient] Jane Doe (p1)", seen[1].label)
    assert.are.equal("[Observation] Heart rate (o1)", seen[2].label)

    local obs = index.get(buf).by_identity["Observation/o1"]
    assert.are.same(
      { obs.location.range[1] + 1, obs.location.range[2] },
      vim.api.nvim_win_get_cursor(0)
    )
  end)

  it("lists the whole workspace and jumps on selection", function()
    local root = h.workspace_clone()
    local buf = h.open_file(root .. "/patients/alice.json")
    vim.api.nvim_win_set_buf(0, buf)
    local items
    local orig = vim.ui.select
    vim.ui.select = function(list, _, cb)
      items = list
      for _, item in ipairs(list) do
        if item.label:match("Heart rate") then
          cb(item)
          return
        end
      end
    end
    outline.run_workspace()
    vim.ui.select = orig
    assert.are.equal(4, #items) -- alice, hr, and the two bundle entries
    assert.are.equal(root .. "/observations/hr.json", vim.api.nvim_buf_get_name(0))
    require("fhir.config").setup({})
    require("fhir.workspace")._reset()
  end)

  it("notifies when the buffer has no resources", function()
    local buf = h.buf("")
    vim.api.nvim_win_set_buf(0, buf)
    local msg
    local orig = vim.notify
    vim.notify = function(m)
      msg = m
    end
    pcall(outline.run)
    vim.notify = orig
    assert.is_not_nil(msg)
    assert.is_not_nil(msg:match("[Nn]o "))
  end)
end)
