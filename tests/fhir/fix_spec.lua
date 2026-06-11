local h = require("tests.helpers")
local fix = require("fhir.features.fix")

-- Apply the single applicable repair for `issue` against a fresh buffer;
-- returns the decoded result (assertions are semantic, never formatting).
local function repaired(json, issue)
  local buf = h.buf(json)
  vim.api.nvim_win_set_buf(0, buf)
  local fixes = fix.fixes_for(buf, issue)
  assert.are.equal(1, #fixes)
  fixes[1].apply()
  return vim.json.decode(table.concat(vim.api.nvim_buf_get_lines(buf, 0, -1, false), "\n")), buf
end

describe("fix catalog", function()
  it("removes an unknown element (middle and last position)", function()
    local doc = repaired(
      '{"resourceType":"Patient","favouriteColor":"blue","active":true}',
      { path = "Patient.favouriteColor", category = "unknown", message = "x" }
    )
    assert.is_nil(doc.favouriteColor)
    assert.is_true(doc.active)
    doc = repaired(
      '{"resourceType":"Patient","active":true,"favouriteColor":"blue"}',
      { path = "Patient.favouriteColor", category = "unknown", message = "x" }
    )
    assert.is_nil(doc.favouriteColor)
    assert.is_true(doc.active)
  end)

  it("inserts a required-element skeleton with the cursor inside", function()
    local doc, buf = repaired('{"resourceType":"Observation","code":{"text":"x"}}', {
      path = "Observation.status",
      category = "cardinality",
      message = 'required element "status" is missing',
    })
    assert.are.equal("", doc.status)
    -- the cursor sits on the skeleton, ready to type
    local pos = vim.api.nvim_win_get_cursor(0)
    local line = vim.api.nvim_buf_get_lines(buf, pos[1] - 1, pos[1], false)[1]
    assert.is_not_nil(line:find('"status"', 1, true))
  end)

  it("rewrites a bare choice name to the suggested form", function()
    local doc = repaired('{"resourceType":"Patient","deceased":true}', {
      path = "Patient.deceased",
      category = "choice",
      message = '"deceased" takes a typed form (e.g. "deceasedBoolean")',
    })
    assert.is_nil(doc.deceased)
    assert.is_true(doc.deceasedBoolean)
  end)

  it("wraps and unwraps array shapes", function()
    local doc = repaired('{"resourceType":"Patient","name":{"family":"X"}}', {
      path = "Patient.name",
      category = "cardinality",
      message = "expected an array (the element repeats)",
    })
    assert.are.equal("X", doc.name[1].family)
    doc = repaired('{"resourceType":"Patient","gender":["male"]}', {
      path = "Patient.gender",
      category = "cardinality",
      message = "did not expect an array (the element does not repeat)",
    })
    assert.are.equal("male", doc.gender)
  end)

  it("offers no unwrap for a multi-element array and no fix for the unfixable", function()
    local buf = h.buf('{"resourceType":"Patient","gender":["male","female"]}')
    assert.are.equal(0, #fix.fixes_for(buf, {
      path = "Patient.gender",
      category = "cardinality",
      message = "did not expect an array (the element does not repeat)",
    }))
    assert.are.equal(0, #fix.fixes_for(buf, {
      path = "Patient.gender",
      category = "format",
      message = "does not match the code format",
    }))
  end)

  it("removes an empty array element", function()
    local doc = repaired('{"resourceType":"Patient","photo":[],"active":true}', {
      path = "Patient.photo",
      category = "cardinality",
      message = "arrays must not be empty",
    })
    assert.is_nil(doc.photo)
    assert.is_true(doc.active)
  end)
end)
