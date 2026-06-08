local h = require("tests.helpers")
local ui = require("fhir.ui")

describe("ui.jump_to", function()
  it("moves the cursor and pushes the origin onto the jumplist", function()
    local buf = h.buf('{\n  "a": "x",\n  "b": "y"\n}')
    vim.api.nvim_win_set_buf(0, buf)
    vim.api.nvim_win_set_cursor(0, { 1, 0 }) -- origin: line 1
    ui.jump_to({ bufnr = buf, range = { 2, 7, 2, 10 } }) -- target: line 3 (0-indexed row 2)
    assert.are.same({ 3, 7 }, vim.api.nvim_win_get_cursor(0))
    local jl = vim.fn.getjumplist()[1]
    assert.is_true(#jl >= 1)
    assert.are.equal(1, jl[#jl].lnum) -- origin line recorded
  end)
end)

describe("ui.notify", function()
  it("does not throw", function()
    assert.has_no.errors(function()
      ui.notify("hello", vim.log.levels.INFO)
    end)
  end)
end)
