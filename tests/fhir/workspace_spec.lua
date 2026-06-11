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

  it("indexes identities across files", function()
    local idx = workspace.index(FIX)
    assert.are.same({ FIX .. "/patients/alice.json" }, idx.by_identity["Patient/p1"])
    assert.are.same({ FIX .. "/bundle.json" }, idx.by_identity["urn:uuid:m1"])
    assert.are.same({ FIX .. "/bundle.json" }, idx.by_identity["Medication/m1"])
    assert.is_not_nil(idx.by_identity["Observation/o1"])
  end)

  it("collects outgoing references per file", function()
    local idx = workspace.index(FIX)
    local hr = idx.references[FIX .. "/observations/hr.json"]
    assert.are.same({ "Patient/p1" }, hr)
    local bundle = idx.references[FIX .. "/bundle.json"]
    table.sort(bundle)
    assert.are.same({ "Patient/p1", "urn:uuid:m1" }, bundle)
  end)

  it("skips non-resources and unparseable files quietly", function()
    local idx = workspace.index(FIX)
    for _, files in pairs(idx.by_identity) do
      for _, f in ipairs(files) do
        assert.is_nil(f:match("notes%.json"))
        assert.is_nil(f:match("broken%.json"))
      end
    end
    assert.are.equal(2, idx.skipped) -- counted, never silent
  end)

  it("re-decodes only files whose mtime changed", function()
    local tmp = vim.fn.tempname()
    vim.fn.mkdir(tmp, "p")
    local file = tmp .. "/x.json"
    vim.fn.writefile({ '{"resourceType":"Patient","id":"a"}' }, file)
    assert.is_not_nil(workspace.index(tmp).by_identity["Patient/a"])
    vim.fn.writefile({ '{"resourceType":"Patient","id":"b"}' }, file)
    local stat = vim.uv.fs_stat(file)
    vim.uv.fs_utime(file, stat.mtime.sec + 5, stat.mtime.sec + 5)
    local idx = workspace.index(tmp)
    assert.is_not_nil(idx.by_identity["Patient/b"])
    assert.is_nil(idx.by_identity["Patient/a"])
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
