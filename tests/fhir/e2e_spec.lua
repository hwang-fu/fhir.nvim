local h = require("tests.helpers")

describe("end-to-end go-to-reference", function()
  it("setup + attach + :FhirGoto jumps across a urn:uuid reference", function()
    require("fhir").setup({})
    local buf = h.fixture_buf("bundle_urn.json")
    vim.api.nvim_win_set_buf(0, buf)
    require("fhir.detect").attach(buf)

    local refs = require("fhir.index").get(buf).references
    vim.api.nvim_win_set_cursor(0, { refs[1].location.range[1] + 1, refs[1].location.range[2] + 1 })

    vim.cmd("FhirGoto")

    local target = require("fhir.index").get(buf).by_identity["urn:uuid:p1"].location
    assert.are.same({ target.range[1] + 1, target.range[2] }, vim.api.nvim_win_get_cursor(0))
  end)

  it("setup is idempotent and defines the global :FhirEnable", function()
    require("fhir").setup({})
    require("fhir").setup({})
    assert.is_not_nil(vim.api.nvim_get_commands({}).FhirEnable)
  end)
end)

describe("end-to-end find-usages", function()
  it("setup + attach + :FhirUsages jumps to a referencing location", function()
    require("fhir").setup({})
    local buf = h.fixture_buf("bundle_urn.json")
    vim.api.nvim_win_set_buf(0, buf)
    require("fhir.detect").attach(buf)

    local patient = require("fhir.index").get(buf).by_identity["Patient/p1"]
    vim.api.nvim_win_set_cursor(0, { patient.location.range[1] + 1, patient.location.range[2] + 1 })

    local orig = vim.ui.select
    vim.ui.select = function(list, _, cb)
      cb(list[1])
    end
    vim.cmd("FhirUsages")
    vim.ui.select = orig

    local occ = require("fhir.index").get(buf).reverse[patient][1]
    assert.are.same(
      { occ.location.range[1] + 1, occ.location.range[2] },
      vim.api.nvim_win_get_cursor(0)
    )
  end)
end)

describe("end-to-end outline", function()
  it("setup + attach + :FhirOutline jumps to the chosen resource", function()
    require("fhir").setup({})
    local buf = h.fixture_buf("bundle_urn.json")
    vim.api.nvim_win_set_buf(0, buf)
    require("fhir.detect").attach(buf)

    local orig = vim.ui.select
    vim.ui.select = function(list, _, cb)
      cb(list[2]) -- the Observation
    end
    vim.cmd("FhirOutline")
    vim.ui.select = orig

    local obs = require("fhir.index").get(buf).by_identity["Observation/o1"]
    assert.are.same(
      { obs.location.range[1] + 1, obs.location.range[2] },
      vim.api.nvim_win_get_cursor(0)
    )
  end)
end)

describe("end-to-end eval", function()
  it("setup + attach + :FhirEval floats the result", function()
    package.loaded["fhir.native"] = {
      available = function()
        return true
      end,
      eval = function()
        return '["final"]', nil
      end,
    }
    package.loaded["fhir.features.eval"] = nil
    require("fhir").setup({})
    local buf = h.fixture_buf("bundle_urn.json")
    vim.api.nvim_win_set_buf(0, buf)
    require("fhir.detect").attach(buf)

    local ui = require("fhir.ui")
    local floated
    local orig = ui.float
    ui.float = function(lines)
      floated = lines
    end
    vim.cmd("FhirEval status")
    ui.float = orig
    package.loaded["fhir.native"] = nil
    package.loaded["fhir.features.eval"] = nil

    assert.are.same({ '"final"' }, floated)
  end)
end)

describe("engine fetch command", function()
  it("setup registers :FhirFetchEngine and it delegates the tag", function()
    require("fhir").setup({})
    local fetch = require("fhir.fetch")
    local orig = fetch.run
    local seen
    fetch.run = function(tag)
      seen = tag or "default"
    end
    vim.cmd("FhirFetchEngine v1.2.3")
    assert.are.equal("v1.2.3", seen)
    vim.cmd("FhirFetchEngine")
    assert.are.equal("default", seen)
    fetch.run = orig
  end)
end)
