local ui = require("fhir.ui")

describe("ui.float", function()
  it("opens a scratch float with the lines and close mappings", function()
    local win, buf = ui.float({ "alpha", "beta" }, { ft = "json" })
    assert.is_true(vim.api.nvim_win_is_valid(win))
    assert.are.same({ "alpha", "beta" }, vim.api.nvim_buf_get_lines(buf, 0, -1, false))
    assert.are.equal("json", vim.bo[buf].filetype)
    local has_q = false
    for _, map in ipairs(vim.api.nvim_buf_get_keymap(buf, "n")) do
      if map.lhs == "q" then
        has_q = true
      end
    end
    assert.is_true(has_q)
    vim.api.nvim_win_close(win, true)
  end)

  it("closes itself when focus leaves the window", function()
    local prev = vim.api.nvim_get_current_win()
    local win = ui.float({ "x" }, {})
    assert.is_true(vim.api.nvim_win_is_valid(win))
    vim.api.nvim_set_current_win(prev)
    assert.is_false(vim.api.nvim_win_is_valid(win))
  end)

  it("renders a title and widens to fit it", function()
    local win = ui.float({ "x" }, { title = "Observation/o1" })
    local cfg = vim.api.nvim_win_get_config(win)
    assert.are.equal(" Observation/o1 ", cfg.title[1][1])
    assert.is_true(cfg.width >= #" Observation/o1 ")
    vim.api.nvim_win_close(win, true)
  end)

  it("middle-truncates an overlong title, keeping type and id tail", function()
    local long = "MedicationAdministration/8c95253e-8ee8-45e6-8ccd-9af713d7a4be"
    local win = ui.float({ "x" }, { title = long })
    local rendered = vim.api.nvim_win_get_config(win).title[1][1]
    assert.is_true(#rendered <= 42) -- 40 cap + the padding spaces
    assert.is_not_nil(rendered:match("^ Medication"))
    assert.is_not_nil(rendered:match("%.%.%."))
    assert.is_not_nil(rendered:match("af713d7a4be $"))
    vim.api.nvim_win_close(win, true)
  end)

  it("omits the title when none is given", function()
    local win = ui.float({ "x" }, {})
    assert.is_nil(vim.api.nvim_win_get_config(win).title)
    vim.api.nvim_win_close(win, true)
  end)
end)
