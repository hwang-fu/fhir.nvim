local h = require("tests.helpers")
local index = require("fhir.index")

describe("index", function()
  it("indexes bundle resources by Type/id and by fullUrl", function()
    local buf = h.fixture_buf("bundle_urn.json")
    local idx = index.get(buf)
    assert.are.equal(2, #idx.resources)
    assert.is_not_nil(idx.by_identity["Patient/p1"])
    assert.is_not_nil(idx.by_identity["urn:uuid:p1"])
    assert.are.equal("Observation", idx.by_identity["Observation/o1"].resource_type)
  end)

  it("collects reference occurrences with flavor and position", function()
    local refs = index.get(h.fixture_buf("bundle_urn.json")).references
    assert.are.equal(1, #refs)
    assert.are.equal("urn:uuid:p1", refs[1].raw)
    assert.are.equal("urn-uuid", refs[1].flavor)
    assert.is_not_nil(refs[1].owner)
    assert.are.equal("Observation", refs[1].owner.resource_type)
    assert.is_table(refs[1].location.range)
  end)

  it("classifies all four reference flavors", function()
    local refs = index.get(h.fixture_buf("relative_absolute.json")).references
    local seen = {}
    for _, r in ipairs(refs) do
      seen[r.flavor] = true
    end
    assert.is_true(seen["relative"])
    assert.is_true(seen["absolute"])
    local cref = index.get(h.fixture_buf("contained.json")).references[1]
    assert.are.equal("contained", cref.flavor)
  end)

  it("records contained resources scoped to their owner", function()
    local idx = index.get(h.fixture_buf("contained.json"))
    local owner = idx.references[1].owner
    assert.is_not_nil(owner.contained["pr1"])
  end)

  it("never throws on malformed JSON - returns an empty-ish index", function()
    local ok, idx = pcall(index.get, h.fixture_buf("malformed.json"))
    assert.is_true(ok)
    assert.is_table(idx.references)
  end)

  it("caches per buffer and rebuilds when changedtick advances", function()
    local buf = h.fixture_buf("bundle_urn.json")
    local a = index.get(buf)
    assert.are.equal(a, index.get(buf))
    vim.api.nvim_buf_set_lines(buf, 0, 0, false, { "" })
    assert.are_not.equal(a, index.get(buf))
  end)
end)

describe("index reverse map", function()
  it("maps a resource to the occurrences that reference it", function()
    local idx = index.get(h.fixture_buf("bundle_urn.json"))
    local patient = idx.by_identity["Patient/p1"]
    local usages = idx.reverse[patient]
    assert.is_not_nil(usages)
    assert.are.equal(1, #usages)
    assert.are.equal("Observation", usages[1].owner.resource_type)
    assert.is_table(usages[1].location.range)
  end)

  it("leaves unreferenced resources out of the reverse map", function()
    local idx = index.get(h.fixture_buf("bundle_urn.json"))
    local obs = idx.by_identity["Observation/o1"]
    assert.is_nil(idx.reverse[obs])
  end)
end)

describe("index resource node", function()
  it("stores each resource's object node", function()
    local idx = index.get(h.fixture_buf("bundle_urn.json"))
    local patient = idx.by_identity["Patient/p1"]
    assert.is_not_nil(patient.node)
    assert.are.equal("object", patient.node:type())
  end)
end)
