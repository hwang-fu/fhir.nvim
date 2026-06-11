local h = require("tests.helpers")

local FIXTURE = [[{
  "resourceType": "Patient",
  "name": [
    { "family": "Chalmers", "given": ["Peter", "James"] },
    { "given": ["Jim"] }
  ],
  "contact": [ { "gender": "male" } ]
}]]

describe("validate feature", function()
  local fake_native, notified, feature, ui, buf

  local ISSUES = vim.json.encode({
    {
      path = "Patient.contact[0].gender",
      severity = "error",
      category = "type",
      message = "expected true or false",
    },
    {
      path = "Patient.name[1].given",
      severity = "warning",
      category = "invariant",
      message = "x-1: be nice",
    },
  })

  before_each(function()
    fake_native = {
      available = function()
        return true
      end,
      validate = function(json, resolver)
        fake_native.seen = { json = json, resolver = resolver }
        return ISSUES, nil
      end,
    }
    package.loaded["fhir.native"] = fake_native
    package.loaded["fhir.features.validate"] = nil
    feature = require("fhir.features.validate")
    ui = require("fhir.ui")
    notified = nil
    ui.notify = function(msg)
      notified = msg
    end
    buf = h.buf(FIXTURE)
    vim.api.nvim_win_set_buf(0, buf)
  end)

  after_each(function()
    package.loaded["fhir.native"] = nil
    package.loaded["fhir.features.validate"] = nil
  end)

  it("lands issues in the diagnostics namespace at mapped ranges", function()
    feature.run()
    local diags = vim.diagnostic.get(buf)
    table.sort(diags, function(a, b)
      return a.lnum < b.lnum
    end)
    assert.are.equal(2, #diags)
    assert.are.equal(4, diags[1].lnum) -- name[1].given
    assert.are.equal(vim.diagnostic.severity.WARN, diags[1].severity)
    assert.are.equal("x-1: be nice", diags[1].message)
    assert.are.equal("fhir", diags[1].source)
    assert.are.equal(6, diags[2].lnum) -- contact[0].gender
    assert.are.equal(vim.diagnostic.severity.ERROR, diags[2].severity)
    -- the document went across with a resolver attached
    assert.is_not_nil(fake_native.seen.json:match('"Patient"'))
    assert.are.equal("function", type(fake_native.seen.resolver))
  end)

  it("clear() empties the namespace", function()
    feature.run()
    assert.are.equal(2, #vim.diagnostic.get(buf))
    feature.clear()
    assert.are.same({}, vim.diagnostic.get(buf))
  end)

  it("notifies on engine errors", function()
    fake_native.validate = function()
      return nil, "validate error: bad json"
    end
    feature.run()
    assert.is_not_nil(notified:match("validate error"))
  end)

  it("on_save is silent without the engine, run() explains", function()
    fake_native.available = function()
      return false
    end
    fake_native.validate = function()
      return nil, "FHIRPath engine not available (run `make build`)"
    end
    feature.on_save()
    assert.is_nil(notified)
    feature.run()
    assert.is_not_nil(notified:match("not available"))
  end)

  it("validates the whole document, not the first indexed resource", function()
    local BUNDLE = [[{
  "resourceType": "Bundle",
  "type": "collection",
  "entry": [
    { "resource": { "resourceType": "Patient", "id": "p1" } }
  ]
}]]
    fake_native.validate = function(json)
      fake_native.seen = { json = json }
      return vim.json.encode({
        {
          path = "Bundle.entry[0].resource",
          severity = "warning",
          category = "invariant",
          message = "dom-6: A resource should have narrative for robust management",
        },
      }),
        nil
    end
    local bbuf = h.buf(BUNDLE)
    vim.api.nvim_win_set_buf(0, bbuf)
    feature.run()
    -- the Bundle itself crossed the seam, not its first entry
    assert.is_not_nil(fake_native.seen.json:find('"Bundle"', 1, true))
    local diags = vim.diagnostic.get(bbuf)
    assert.are.equal(1, #diags)
    assert.are.equal(4, diags[1].lnum) -- the entry's "resource" key
  end)

  it("revalidates on write when attached and enabled", function()
    require("fhir.config").setup({})
    require("fhir.detect").attach(buf)
    vim.api.nvim_exec_autocmds("BufWritePost", { buffer = buf })
    assert.are.equal(2, #vim.diagnostic.get(buf))
  end)
end)
