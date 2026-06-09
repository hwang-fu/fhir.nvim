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
