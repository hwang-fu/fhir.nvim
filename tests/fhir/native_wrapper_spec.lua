local native = require("fhir.native")

describe("fhir.native", function()
  after_each(function()
    native._reset()
    package.loaded["fhir_core"] = nil
  end)

  it("delegates eval to the loaded module", function()
    local seen
    package.loaded["fhir_core"] = {
      eval = function(json, expr, resolver)
        seen = { json = json, expr = expr, resolver = resolver }
        return '["ok"]', nil
      end,
    }
    local result, err = native.eval("{}", "id", print)
    assert.is_nil(err)
    assert.are.equal('["ok"]', result)
    assert.are.equal("id", seen.expr)
    assert.are.equal(print, seen.resolver)
    assert.is_true(native.available())
  end)

  it("degrades when the module cannot load", function()
    native._require = function()
      error("not found")
    end
    local result, err = native.eval("{}", "id")
    assert.is_nil(result)
    assert.is_not_nil(err:match("make build"))
    assert.is_false(native.available())
    native._require = require
  end)
end)
