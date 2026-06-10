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
    assert.are.same({}, config.get().native) -- no dir set by default
    config.setup({ native = { dir = "/tmp/x" } })
    assert.are.equal("/tmp/x", config.get().native.dir)
    assert.has_error(function()
      config.setup({ native = { dir = 42 } })
    end)
  end)
end)
