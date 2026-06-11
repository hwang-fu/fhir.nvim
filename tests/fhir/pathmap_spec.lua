local h = require("tests.helpers")
local index = require("fhir.index")
local pathmap = require("fhir.pathmap")

local FIXTURE = [[{
  "resourceType": "Patient",
  "name": [
    { "family": "Chalmers", "given": ["Peter", "James"] },
    { "given": ["Jim"] }
  ],
  "contact": [ { "gender": "male" } ]
}]]

local function fixture()
  local buf = h.buf(FIXTURE)
  return buf, index.get(buf).resources[1]
end

describe("pathmap", function()
  it("resolves dotted paths to the element's key", function()
    local buf, res = fixture()
    local r = pathmap.range(res.node, buf, "Patient.contact[0].gender")
    assert.are.same(6, r[1]) -- the "gender" key's row
    local r2 = pathmap.range(res.node, buf, "Patient.name[1].given")
    assert.are.same(4, r2[1])
  end)

  it("resolves indices to the array element itself", function()
    local buf, res = fixture()
    local r = pathmap.range(res.node, buf, "Patient.name[1]")
    assert.are.same(4, r[1])
    local r2 = pathmap.range(res.node, buf, "Patient.name[0].given[1]")
    assert.are.same(3, r2[1]) -- "James"
  end)

  it("falls back to the deepest resolvable ancestor", function()
    local buf, res = fixture()
    -- a missing required child lands on its parent
    local r = pathmap.range(res.node, buf, "Patient.contact[0].nothing")
    assert.are.same(6, r[1]) -- the contact object's row
    -- an out-of-range index lands on the element's key
    local r2 = pathmap.range(res.node, buf, "Patient.name[7].family")
    assert.are.same(2, r2[1])
    -- nothing resolvable at all: the resource's first row
    local r3 = pathmap.range(res.node, buf, "Patient.nope.deeper")
    assert.are.same(0, r3[1])
    -- the resource root itself
    local r4 = pathmap.range(res.node, buf, "Patient")
    assert.are.same(0, r4[1])
  end)
end)
