local workspace = require("fhir.workspace")

local FIX = vim.fn.getcwd() .. "/tests/fixtures/workspace"

describe("workspace", function()
  after_each(function()
    require("fhir.config").setup({})
    workspace._reset()
  end)

  it("finds the root by marker, falling back to the cwd", function()
    local tmp = vim.fn.tempname()
    vim.fn.mkdir(tmp .. "/.git", "p")
    vim.fn.mkdir(tmp .. "/a/b", "p")
    vim.fn.writefile({ "{}" }, tmp .. "/a/b/x.json")
    assert.are.equal(tmp, workspace.root(tmp .. "/a/b/x.json"))

    local lone = vim.fn.tempname()
    vim.fn.mkdir(lone .. "/c", "p")
    vim.fn.writefile({ "{}" }, lone .. "/c/x.json")
    assert.are.equal(vim.uv.cwd(), workspace.root(lone .. "/c/x.json"))
  end)

  it("discovers json files minus the ignore set", function()
    local files = workspace.files(FIX)
    local names = {}
    for _, f in ipairs(files) do
      names[f:sub(#FIX + 2)] = true
    end
    assert.is_true(names["patients/alice.json"])
    assert.is_true(names["observations/hr.json"])
    assert.is_true(names["bundle.json"])
    assert.is_true(names["notes.json"]) -- discovery is name-based; records decide
    assert.is_nil(names["node_modules/dep.json"])
  end)

  it("clips at max_files and says so", function()
    require("fhir.config").setup({ workspace = { max_files = 2 } })
    local ui = require("fhir.ui")
    local orig, notified = ui.notify, nil
    ui.notify = function(msg)
      notified = msg
    end
    local files, clipped = workspace.files(FIX)
    ui.notify = orig
    assert.are.equal(2, #files)
    assert.is_true(clipped)
    assert.is_not_nil(notified:match("max_files"))
  end)

  it("replaces the ignore list wholesale when configured", function()
    require("fhir.config").setup({ workspace = { ignore = { "patients" } } })
    local ws = require("fhir.config").get().workspace
    assert.are.same({ "patients" }, ws.ignore) -- no merge with the defaults
    local files = workspace.files(FIX)
    local seen = {}
    for _, f in ipairs(files) do
      seen[f:sub(#FIX + 2)] = true
    end
    assert.is_nil(seen["patients/alice.json"])
    assert.is_true(seen["node_modules/dep.json"]) -- default set replaced
  end)
end)
