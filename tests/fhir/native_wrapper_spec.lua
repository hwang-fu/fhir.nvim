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

  it("delegates validate and guards against engines that lack it", function()
    package.loaded["fhir_core"] = {
      eval = function() end,
      validate = function(json, resolver)
        return '[{"path":"' .. json .. '"}]', tostring(resolver ~= nil)
      end,
    }
    local result, err = native.validate("Patient", print)
    assert.are.equal('[{"path":"Patient"}]', result)
    assert.are.equal("true", err)

    native._reset()
    package.loaded["fhir_core"] = { eval = function() end } -- an older engine
    local r2, e2 = native.validate("{}")
    assert.is_nil(r2)
    assert.is_not_nil(e2:match("FhirFetchEngine"))
  end)

  it("validate degrades when the module cannot load", function()
    native._require = function()
      error("not found")
    end
    local result, err = native.validate("{}")
    assert.is_nil(result)
    assert.is_not_nil(err:match("make build"))
    native._require = require
  end)

  it("searches config dir, then the pinned data dir, then the dev build", function()
    local dirs = native._dirs()
    assert.is_not_nil(dirs[1]:match("fhir%.nvim/v"))
    assert.is_not_nil(dirs[2]:match("%.tests$"))
    require("fhir.config").setup({ native = { dir = "/explicit" } })
    dirs = native._dirs()
    assert.are.equal("/explicit", dirs[1])
    assert.are.equal(3, #dirs)
    require("fhir.config").setup({})
  end)
end)
