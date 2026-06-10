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
end)
