local h = require("tests.helpers")

describe("workspace end to end (real engine)", function()
  it("validates, fixes, and re-validates across a workspace", function()
    if not require("fhir.native").available() then
      print("SKIP: native module not built (run `make build`)")
      return
    end
    local root = h.workspace_clone()
    vim.fn.writefile({
      '{"resourceType":"Observation","id":"bad",',
      '"code":{"text":"x"},"favouriteColor":"blue"}',
    }, root .. "/bad.json")
    local buf = h.open_file(root .. "/bad.json")
    vim.api.nvim_win_set_buf(0, buf)

    local validate = require("fhir.features.validate")
    validate.run()
    local diags = vim.diagnostic.get(buf)
    local function has(pattern)
      for _, d in ipairs(diags) do
        if d.message:match(pattern) then
          return d
        end
      end
    end
    assert.is_not_nil(has("favouriteColor")) -- unknown element
    assert.is_not_nil(has('"status" is missing')) -- required

    -- fix the unknown element through the real flow
    local d = has("favouriteColor")
    vim.api.nvim_win_set_cursor(0, { d.lnum + 1, d.col })
    require("fhir.features.fix").run()

    diags = vim.diagnostic.get(buf)
    assert.is_nil(has("favouriteColor")) -- gone and re-validated
    assert.is_not_nil(has('"status" is missing')) -- still honest

    require("fhir.config").setup({})
    require("fhir.workspace")._reset()
  end)
end)
