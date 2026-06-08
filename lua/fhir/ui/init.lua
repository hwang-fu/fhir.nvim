local M = {}

-- Jump to a Location, pushing the current position onto the jumplist first so
-- <C-o> returns. (nvim_win_set_cursor alone does NOT touch the jumplist.)
function M.jump_to(location)
  vim.cmd("normal! m'")
  if location.bufnr ~= vim.api.nvim_get_current_buf() then
    vim.api.nvim_set_current_buf(location.bufnr)
  end
  vim.api.nvim_win_set_cursor(0, { location.range[1] + 1, location.range[2] })
end

-- Notify the user with a consistent "fhir:" prefix.
function M.notify(msg, level)
  vim.notify("fhir: " .. msg, level or vim.log.levels.INFO)
end

return M
