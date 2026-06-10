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

describe("fhir.fetch run", function()
  local tmp, notified

  local function stub_downloads(files)
    -- files: url suffix -> content; the stub "curl" writes content to -o path
    fetch._system = function(cmd)
      local out, url
      for i, a in ipairs(cmd) do
        if a == "-o" then
          out = cmd[i + 1]
        end
        url = a
      end
      local content
      for suffix, c in pairs(files) do
        if url:sub(-#suffix) == suffix then
          content = c
        end
      end
      return {
        wait = function()
          if content then
            local f = assert(io.open(out, "wb"))
            f:write(content)
            f:close()
            return { code = 0 }
          end
          return { code = 22 }
        end,
      }
    end
  end

  before_each(function()
    tmp = vim.fn.tempname()
    fetch._data_root = tmp
    fetch._uname = function()
      return { sysname = "Linux", machine = "x86_64" }
    end
    notified = {}
    require("fhir.ui").notify = function(msg)
      notified[#notified + 1] = msg
    end
  end)

  after_each(function()
    fetch._uname = vim.uv.os_uname
    fetch._system = vim.system
    fetch._data_root = nil
  end)

  it("downloads, verifies, and installs the engine", function()
    local payload = "fake-cdylib-bytes"
    local sum = vim.fn.sha256(payload)
    stub_downloads({
      ["SHA256SUMS"] = sum .. "  fhir_core-linux-x86_64.so\n",
      ["fhir_core-linux-x86_64.so"] = payload,
    })
    fetch.run("v9.9.9")
    local installed = tmp .. "/fhir.nvim/v9.9.9/fhir_core.so"
    local f = assert(io.open(installed, "rb"))
    assert.are.equal(payload, f:read("*a"))
    f:close()
    assert.is_not_nil(table.concat(notified, " "):match("installed"))
  end)

  it("refuses a checksum mismatch and installs nothing", function()
    stub_downloads({
      ["SHA256SUMS"] = string.rep("0", 64) .. "  fhir_core-linux-x86_64.so\n",
      ["fhir_core-linux-x86_64.so"] = "tampered-bytes",
    })
    fetch.run("v9.9.9")
    assert.are.equal(0, vim.fn.filereadable(tmp .. "/fhir.nvim/v9.9.9/fhir_core.so"))
    assert.is_not_nil(table.concat(notified, " "):match("[Cc]hecksum"))
  end)

  it("reports unsupported platforms", function()
    fetch._uname = function()
      return { sysname = "Windows_NT", machine = "x86_64" }
    end
    fetch.run("v9.9.9")
    assert.is_not_nil(table.concat(notified, " "):match("build"))
  end)

  it("reports a missing release cleanly", function()
    stub_downloads({})
    fetch.run("v9.9.9")
    assert.is_not_nil(table.concat(notified, " "):match("[Dd]ownload"))
  end)
end)
