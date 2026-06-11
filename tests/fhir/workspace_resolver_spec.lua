local h = require("tests.helpers")
local index = require("fhir.index")
local resolver = require("fhir.resolver")
local workspace = require("fhir.workspace")

describe("workspace resolver", function()
  local root

  before_each(function()
    root = h.workspace_clone()
  end)

  after_each(function()
    require("fhir.config").setup({})
    workspace._reset()
  end)

  local function occ_in(file, raw)
    local buf = h.open_file(root .. "/" .. file)
    vim.api.nvim_win_set_buf(0, buf)
    local idx = index.get(buf)
    return { raw = raw, flavor = index.flavor(raw), owner = idx.resources[1] }, idx
  end

  it("resolves a relative reference into another file", function()
    local occ, idx = occ_in("observations/hr.json", "Patient/p1")
    local loc = resolver.resolve(occ, idx)
    assert.is_not_nil(loc)
    assert.are.equal(root .. "/patients/alice.json", vim.api.nvim_buf_get_name(loc.bufnr))
    local target = index.get(loc.bufnr).resources[1]
    assert.are.same(target.location.range, loc.range)
  end)

  it("resolves a urn fullUrl into a bundle in another file", function()
    vim.fn.writefile({
      '{"resourceType":"Observation","id":"o9","status":"final",',
      '"code":{"text":"x"},"subject":{"reference":"urn:uuid:m1"}}',
    }, root .. "/obs2.json")
    local occ, idx = occ_in("obs2.json", "urn:uuid:m1")
    local loc = resolver.resolve(occ, idx)
    assert.is_not_nil(loc)
    assert.are.equal(root .. "/bundle.json", vim.api.nvim_buf_get_name(loc.bufnr))
  end)

  it("prefers a local resolution over the workspace", function()
    local occ, idx = occ_in("bundle.json", "urn:uuid:m1")
    local loc = resolver.resolve(occ, idx)
    assert.are.equal(vim.api.nvim_get_current_buf(), loc.bufnr)
  end)

  it("returns nil for the unresolvable", function()
    local occ, idx = occ_in("observations/hr.json", "Patient/nobody")
    assert.is_nil(resolver.resolve(occ, idx))
  end)

  it("breaks identity ties deterministically and says so", function()
    vim.fn.writefile({ '{"resourceType":"Patient","id":"p9"}' }, root .. "/a.json")
    vim.fn.writefile({ '{"resourceType":"Patient","id":"p9"}' }, root .. "/b.json")
    vim.fn.writefile({
      '{"resourceType":"Observation","id":"o8","status":"final",'
        .. '"code":{"text":"x"},"subject":{"reference":"Patient/p9"}}',
    }, root .. "/obs3.json")
    local ui = require("fhir.ui")
    local orig, notified = ui.notify, nil
    ui.notify = function(msg)
      notified = msg
    end
    local occ, idx = occ_in("obs3.json", "Patient/p9")
    local loc = resolver.resolve(occ, idx)
    ui.notify = orig
    assert.are.equal(root .. "/a.json", vim.api.nvim_buf_get_name(loc.bufnr))
    assert.is_not_nil(notified:match("match"))
  end)

  it("the engine callback reads targets across files", function()
    local occ, idx = occ_in("observations/hr.json", "Patient/p1")
    local text = resolver.callback(idx, occ.owner)("Patient/p1")
    assert.are.equal("p1", vim.json.decode(text).id)
  end)
end)
