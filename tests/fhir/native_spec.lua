-- Built by `make build` into .tests/fhir_core.so. Skip if absent.
package.cpath = vim.fn.getcwd() .. "/.tests/?.so;" .. package.cpath
local ok, fhir_core = pcall(require, "fhir_core")

local PATIENT = '{"resourceType":"Patient","name":[{"given":["Peter","James"]}]}'

describe("fhir_core native module", function()
  it("evaluates fhirpath and returns json", function()
    if not ok then
      print("SKIP: native module not built (run `make build`)")
      return
    end
    local result, err = fhir_core.eval(PATIENT, "name.given")
    assert.is_nil(err)
    assert.are.same({ "Peter", "James" }, vim.json.decode(result))
  end)

  it("returns nil and a message on a bad expression", function()
    if not ok then
      print("SKIP: native module not built (run `make build`)")
      return
    end
    local result, err = fhir_core.eval("{}", "1 +")
    assert.is_nil(result)
    assert.is_string(err)
  end)

  it("resolve() reaches back into lua", function()
    if not ok then
      print("SKIP: native module not built (run `make build`)")
      return
    end
    local obs = '{"resourceType":"Observation","id":"o1","subject":{"reference":"Patient/p1"}}'
    local result, err = fhir_core.eval(obs, "subject.resolve().id", function(ref)
      if ref == "Patient/p1" then
        return '{"resourceType":"Patient","id":"p1"}'
      end
    end)
    assert.is_nil(err)
    assert.are.same({ "p1" }, vim.json.decode(result))
  end)
end)
