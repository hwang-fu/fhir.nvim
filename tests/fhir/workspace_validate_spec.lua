local h = require("tests.helpers")

describe("workspace validation", function()
  local feature, root, notifications

  -- canned verdicts keyed off the document text: the Observation file has
  -- an error + a warning, Patients a warning, everything else is clean
  local function issues_for(json)
    if json:find('"Observation"', 1, true) then
      return vim.json.encode({
        {
          path = "Observation.status",
          severity = "error",
          category = "cardinality",
          message = 'required element "status" is missing',
        },
        {
          path = "Observation",
          severity = "warning",
          category = "invariant",
          message = "dom-6: A resource should have narrative for robust management",
        },
      })
    end
    if json:find('"Patient"', 1, true) then
      return vim.json.encode({
        {
          path = "Patient",
          severity = "warning",
          category = "invariant",
          message = "dom-6: A resource should have narrative for robust management",
        },
      })
    end
    return "[]"
  end

  before_each(function()
    package.loaded["fhir.native"] = {
      available = function()
        return true
      end,
      validate = function(json)
        return issues_for(json), nil
      end,
    }
    package.loaded["fhir.features.validate"] = nil
    feature = require("fhir.features.validate")
    notifications = {}
    require("fhir.ui").notify = function(msg)
      notifications[#notifications + 1] = msg
    end
    root = h.workspace_clone()
    vim.api.nvim_win_set_buf(0, h.open_file(root .. "/patients/alice.json"))
  end)

  after_each(function()
    package.loaded["fhir.native"] = nil
    package.loaded["fhir.features.validate"] = nil
    require("fhir.config").setup({})
    require("fhir.workspace")._reset()
    vim.fn.setqflist({}, "f")
  end)

  it("collects error findings into the quickfix at exact positions", function()
    feature.run_workspace()
    local qf = vim.fn.getqflist()
    assert.are.equal(1, #qf) -- warnings gated out by default
    assert.are.equal(root .. "/observations/hr.json", vim.api.nvim_buf_get_name(qf[1].bufnr))
    assert.are.equal(4, qf[1].lnum) -- the "status" key's line
    assert.are.equal("E", qf[1].type)
    assert.is_not_nil(qf[1].text:match("status"))
    assert.are.equal("FHIR workspace validation", vim.fn.getqflist({ title = 1 }).title)
  end)

  it("includes advisory findings with the all scope", function()
    feature.run_workspace("all")
    local qf = vim.fn.getqflist()
    assert.are.equal(3, #qf) -- hr error+warning, alice warning
  end)

  it("summarizes counts and points at the quickfix", function()
    feature.run_workspace()
    local last = notifications[#notifications]
    assert.is_not_nil(last:match("1 error"))
    assert.is_not_nil(last:match("2 warning"))
    assert.is_not_nil(last:match("copen"))
  end)

  it("refreshes diagnostics of open attached buffers", function()
    local buf = h.open_file(root .. "/patients/alice.json")
    require("fhir.detect").attach(buf)
    feature.run_workspace()
    local diags = vim.diagnostic.get(buf)
    assert.are.equal(1, #diags) -- the buffer path keeps every severity
    assert.are.equal(vim.diagnostic.severity.WARN, diags[1].severity)
    require("fhir.detect").detach(buf)
  end)

  it("aborts with a notice when the engine is absent", function()
    package.loaded["fhir.native"].available = function()
      return false
    end
    feature.run_workspace()
    assert.are.same({}, vim.fn.getqflist())
    assert.is_not_nil(notifications[#notifications]:match("engine"))
  end)
end)
