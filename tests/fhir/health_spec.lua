describe("health", function()
  it("runs without error and reports nvim version, json parser, and picker", function()
    assert.has_no.errors(function()
      require("fhir.health").check()
    end)
  end)

  it("reports the workspace root and candidate count", function()
    local lines = {}
    local orig = vim.health.info
    vim.health.info = function(msg)
      lines[#lines + 1] = msg
    end
    require("fhir.health").check()
    vim.health.info = orig
    local found
    for _, l in ipairs(lines) do
      if l:match("workspace root") and l:match("%d+ json") then
        found = l
      end
    end
    assert.is_not_nil(found)
  end)
end)
