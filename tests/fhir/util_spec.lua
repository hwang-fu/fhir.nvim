local util = require("fhir.util")

describe("util.range_contains", function()
  local range = { 2, 5, 2, 10 } -- row 2, cols 5..10 (end exclusive)

  it("includes the start, excludes the end column", function()
    assert.is_true(util.range_contains(range, 2, 5))
    assert.is_true(util.range_contains(range, 2, 9))
    assert.is_false(util.range_contains(range, 2, 10))
    assert.is_false(util.range_contains(range, 2, 4))
  end)

  it("rejects other rows", function()
    assert.is_false(util.range_contains(range, 1, 7))
    assert.is_false(util.range_contains(range, 3, 7))
  end)

  it("handles a multi-line range", function()
    local r = { 1, 8, 3, 2 } -- starts row 1 col 8, ends row 3 col 2
    assert.is_true(util.range_contains(r, 2, 0)) -- whole middle row
    assert.is_true(util.range_contains(r, 1, 8))
    assert.is_false(util.range_contains(r, 1, 7))
    assert.is_false(util.range_contains(r, 3, 2))
  end)
end)
