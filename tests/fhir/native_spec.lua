-- Built by `make build` into .tests/fhir_core.so. Skip if absent.
package.cpath = vim.fn.getcwd() .. "/.tests/?.so;" .. package.cpath
local ok, fhir_core = pcall(require, "fhir_core")

describe("fhir_core native module", function()
  it("loads and pings", function()
    if not ok then
      print("SKIP: native module not built (run `make build`)")
      return
    end
    assert.are.equal("hello from fhir-core", fhir_core.ping())
  end)
end)
