local h = require("tests.helpers")

describe("eval feature", function()
  local fake_native, floated, notified, eval_feature, ui

  before_each(function()
    fake_native = {
      available = function()
        return true
      end,
      eval = function(json, expr, resolver)
        fake_native.seen = { json = json, expr = expr, resolver = resolver }
        return '["Peter","James"]', nil
      end,
    }
    package.loaded["fhir.native"] = fake_native
    package.loaded["fhir.features.eval"] = nil
    eval_feature = require("fhir.features.eval")
    ui = require("fhir.ui")
    floated, notified = nil, nil
    ui.float = function(lines)
      floated = lines
    end
    ui.notify = function(msg)
      notified = msg
    end
  end)

  after_each(function()
    package.loaded["fhir.native"] = nil
    package.loaded["fhir.features.eval"] = nil
  end)

  it("evaluates against the resource under the cursor and floats the result", function()
    local buf = h.fixture_buf("bundle_urn.json")
    vim.api.nvim_win_set_buf(0, buf)
    local idx = require("fhir.index").get(buf)
    local obs = idx.by_identity["Observation/o1"]
    vim.api.nvim_win_set_cursor(0, { obs.location.range[1] + 1, obs.location.range[2] })

    eval_feature.run("status")

    assert.is_not_nil(fake_native.seen.json:match('"Observation"'))
    assert.are.equal("status", fake_native.seen.expr)
    assert.are.equal("function", type(fake_native.seen.resolver))
    assert.are.same({ '"Peter"', '"James"' }, floated)
  end)

  it("the resolver callback returns the referenced resource's json", function()
    local buf = h.fixture_buf("bundle_urn.json")
    vim.api.nvim_win_set_buf(0, buf)
    eval_feature.run("id")
    local resolved = fake_native.seen.resolver("Patient/p1")
    assert.is_not_nil(resolved)
    assert.is_not_nil(vim.json.decode(resolved).id)
  end)

  it("notifies when the native engine is unavailable", function()
    fake_native.available = function()
      return false
    end
    fake_native.eval = function()
      return nil, "FHIRPath engine not available (run `make build`)"
    end
    local buf = h.fixture_buf("bundle_urn.json")
    vim.api.nvim_win_set_buf(0, buf)
    eval_feature.run("id")
    assert.is_nil(floated)
    assert.is_not_nil(notified:match("not available"))
  end)

  it("notifies an evaluation error", function()
    fake_native.eval = function()
      return nil, "parse error: unexpected token"
    end
    local buf = h.fixture_buf("bundle_urn.json")
    vim.api.nvim_win_set_buf(0, buf)
    eval_feature.run("1 +")
    assert.is_not_nil(notified:match("parse error"))
  end)

  it("prompts when called without an expression", function()
    local buf = h.fixture_buf("bundle_urn.json")
    vim.api.nvim_win_set_buf(0, buf)
    local orig = vim.ui.input
    vim.ui.input = function(_, cb)
      cb("id")
    end
    eval_feature.run()
    vim.ui.input = orig
    assert.are.equal("id", fake_native.seen.expr)
  end)
end)
