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
    assert.are.equal("v2.2.0", config.get().native.tag)
    config.setup({ native = { tag = "v9.9.9" } })
    assert.are.equal("v9.9.9", config.get().native.tag)
    assert.has_error(function()
      config.setup({ native = { tag = 42 } })
    end)
  end)
end)
