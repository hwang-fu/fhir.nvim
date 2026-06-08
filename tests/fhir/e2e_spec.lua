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
