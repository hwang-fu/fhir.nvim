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

describe("fix flow", function()
  it("fixes the finding under the cursor and re-validates", function()
    package.loaded["fhir.native"] = {
      available = function()
        return true
      end,
      validate = function(json)
        if json:find("favouriteColor", 1, true) then
          return vim.json.encode({
            {
              path = "Patient.favouriteColor",
              category = "unknown",
              severity = "error",
              message = '"favouriteColor" is not an element of Patient',
            },
          }),
            nil
        end
        return "[]", nil
      end,
    }
    package.loaded["fhir.features.validate"] = nil
    package.loaded["fhir.features.fix"] = nil
    local validate = require("fhir.features.validate")
    local flow_fix = require("fhir.features.fix")

    local buf = h.buf('{"resourceType":"Patient","favouriteColor":"blue"}')
    vim.api.nvim_win_set_buf(0, buf)
    validate.run()
    local d = vim.diagnostic.get(buf)[1]
    vim.api.nvim_win_set_cursor(0, { d.lnum + 1, d.col })

    flow_fix.run() -- a single fix applies without a picker

    local doc = vim.json.decode(table.concat(vim.api.nvim_buf_get_lines(buf, 0, -1, false), "\n"))
    assert.is_nil(doc.favouriteColor)
    assert.are.same({}, vim.diagnostic.get(buf)) -- re-validated clean

    package.loaded["fhir.native"] = nil
    package.loaded["fhir.features.validate"] = nil
    package.loaded["fhir.features.fix"] = nil
  end)

  it("notifies when nothing on the line is fixable", function()
    local buf = h.buf('{"resourceType":"Patient"}')
    vim.api.nvim_win_set_buf(0, buf)
    local ui = require("fhir.ui")
    local orig, msg = ui.notify, nil
    ui.notify = function(m)
      msg = m
    end
    fix.run()
    ui.notify = orig
    assert.is_not_nil(msg:match("[Nn]o"))
  end)
end)
