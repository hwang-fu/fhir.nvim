local config = require("fhir.config")

describe("config", function()
  before_each(function()
    config.setup({})
  end)

  it("exposes documented defaults", function()
    local c = config.get()
    assert.are.equal("auto", c.detection)
    assert.are.same({}, c.keymaps) -- no keymaps by default
  end)

  it("deep-merges user opts over defaults", function()
    config.setup({ keymaps = { goto_reference = "gd" } })
    assert.are.equal("gd", config.get().keymaps.goto_reference)
    assert.are.equal("auto", config.get().detection) -- untouched default preserved
  end)

  it("rejects an invalid detection mode", function()
    assert.has_error(function()
      config.setup({ detection = "nonsense" })
    end)
  end)

  it("exposes native defaults and merges overrides", function()
    config.setup({})
    assert.is_nil(config.get().native.dir) -- no dir set by default
    config.setup({ native = { dir = "/tmp/x" } })
    assert.are.equal("/tmp/x", config.get().native.dir)
    assert.has_error(function()
      config.setup({ native = { dir = 42 } })
    end)
  end)

  it("exposes the pinned engine tag", function()
    config.setup({})
    assert.are.equal("v4.0.0", config.get().native.tag)
    config.setup({ native = { tag = "v9.9.9" } })
    assert.are.equal("v9.9.9", config.get().native.tag)
    assert.has_error(function()
      config.setup({ native = { tag = 42 } })
    end)
  end)

  it("exposes workspace defaults and validates overrides", function()
    config.setup({})
    local ws = config.get().workspace
    assert.are.equal(2000, ws.max_files)
    assert.is_true(vim.tbl_contains(ws.ignore, "node_modules"))
    assert.has_error(function()
      config.setup({ workspace = { max_files = "many" } })
    end)
    assert.has_error(function()
      config.setup({ workspace = { ignore = "node_modules" } })
    end)
  end)
end)
