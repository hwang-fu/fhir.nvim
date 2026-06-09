local h = require("tests.helpers")
local index = require("fhir.index")
local labeler = require("fhir.labeler")

local function label_first(text)
  local buf = h.buf(text)
  return labeler.label(index.get(buf).resources[1], buf)
end

describe("labeler", function()
  it("prefers code.text", function()
    assert.are.equal(
      "[Observation] BP (o)",
      label_first('{ "resourceType": "Observation", "id": "o", "code": { "text": "BP" } }')
    )
  end)

  it("then code.coding[].display", function()
    assert.are.equal(
      "[Observation] Heart rate (o)",
      label_first(
        '{ "resourceType": "Observation", "id": "o", "code": { "coding": [ { "display": "Heart rate" } ] } }'
      )
    )
  end)

  it("then a formatted given + family name", function()
    assert.are.equal(
      "[Patient] Jane Doe (p)",
      label_first(
        '{ "resourceType": "Patient", "id": "p", "name": [ { "family": "Doe", "given": [ "Jane" ] } ] }'
      )
    )
  end)

  it("uses name.text when present", function()
    assert.are.equal(
      "[Patient] Dr Jane Doe (p)",
      label_first('{ "resourceType": "Patient", "id": "p", "name": [ { "text": "Dr Jane Doe" } ] }')
    )
  end)

  it("falls back to Type/id when no human field is found", function()
    assert.are.equal("Device/d", label_first('{ "resourceType": "Device", "id": "d" }'))
  end)
end)
