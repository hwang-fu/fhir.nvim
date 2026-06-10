local fetch = require("fhir.fetch")

describe("fhir.fetch platform mapping", function()
  local function with_uname(sysname, machine, fn)
    local orig = fetch._uname
    fetch._uname = function()
      return { sysname = sysname, machine = machine }
    end
    fn()
    fetch._uname = orig
  end

  it("maps supported platforms to asset names", function()
    with_uname("Linux", "x86_64", function()
      assert.are.equal("fhir_core-linux-x86_64.so", fetch.asset())
    end)
    with_uname("Linux", "aarch64", function()
      assert.are.equal("fhir_core-linux-aarch64.so", fetch.asset())
    end)
    with_uname("Darwin", "arm64", function()
      assert.are.equal("fhir_core-macos-aarch64.so", fetch.asset())
    end)
    with_uname("Darwin", "x86_64", function()
      assert.are.equal("fhir_core-macos-x86_64.so", fetch.asset())
    end)
  end)

  it("returns nil on unsupported platforms", function()
    with_uname("Windows_NT", "x86_64", function()
      assert.is_nil(fetch.asset())
    end)
  end)

  it("computes the versioned install dir", function()
    assert.is_not_nil(fetch.dir("v2.0.0"):match("fhir%.nvim/v2%.0%.0$"))
  end)
end)
