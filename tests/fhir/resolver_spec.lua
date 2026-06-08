local h = require("tests.helpers")
local index = require("fhir.index")
local resolver = require("fhir.resolver")

local function ref_named(idx, flavor)
  for _, r in ipairs(idx.references) do
    if r.flavor == flavor then
      return r
    end
  end
end

describe("LocalBufferResolver", function()
  it("resolves urn:uuid to the entry it names", function()
    local idx = index.get(h.fixture_buf("bundle_urn.json"))
    local loc = resolver.resolve(ref_named(idx, "urn-uuid"), idx)
    assert.are.same(idx.by_identity["urn:uuid:p1"].location, loc)
  end)

  it("resolves a relative Type/id reference", function()
    local idx = index.get(h.fixture_buf("relative_absolute.json"))
    local loc = resolver.resolve(ref_named(idx, "relative"), idx)
    assert.are.same(idx.by_identity["Patient/123"].location, loc)
  end)

  it("resolves an absolute URL via exact fullUrl or trailing Type/id", function()
    local idx = index.get(h.fixture_buf("relative_absolute.json"))
    local loc = resolver.resolve(ref_named(idx, "absolute"), idx)
    assert.is_not_nil(loc)
  end)

  it("resolves a contained #localId within its owner", function()
    local idx = index.get(h.fixture_buf("contained.json"))
    local loc = resolver.resolve(ref_named(idx, "contained"), idx)
    assert.are.same(idx.references[1].owner.contained["pr1"], loc)
  end)

  it("returns nil for an unresolvable external reference", function()
    local idx = index.get(h.fixture_buf("unresolvable.json"))
    assert.is_nil(resolver.resolve(idx.references[1], idx))
  end)
end)
