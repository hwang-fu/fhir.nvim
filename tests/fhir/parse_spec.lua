local h = require("tests.helpers")
local parse = require("fhir.parse")

describe("parse", function()
  it("returns the root object node of a buffer", function()
    local buf = h.buf('{ "resourceType": "Patient", "id": "p1" }')
    local root = parse.root(buf)
    assert.is_not_nil(root)
    assert.are.equal("object", root:type())
  end)

  it("reads a string value by key, unescaped", function()
    local buf = h.buf('{ "a": "x\\ty", "n": 3 }')
    local root = parse.root(buf)
    assert.are.equal("x\ty", parse.string_value(root, "a", buf))
    assert.is_nil(parse.string_value(root, "missing", buf))
  end)

  it("gives the value node's range for positioning", function()
    local buf = h.buf('{ "id": "p1" }')
    local root = parse.root(buf)
    local node = parse.value_node(root, "id", buf)
    local srow, scol = node:range()
    assert.are.equal(0, srow)
    assert.is_true(scol > 0)
  end)

  it("iterates an array's element nodes", function()
    local buf = h.buf('{ "xs": [ {"k":1}, {"k":2} ] }')
    local root = parse.root(buf)
    local arr = parse.value_node(root, "xs", buf)
    local n = 0
    for _ in parse.iter_array(arr) do
      n = n + 1
    end
    assert.are.equal(2, n)
  end)
end)
