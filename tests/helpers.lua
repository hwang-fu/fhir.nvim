local M = {}

-- Load a fixture file into a fresh json buffer; returns bufnr.
function M.fixture_buf(name)
  local path = vim.fn.getcwd() .. "/tests/fixtures/" .. name
  local lines = vim.fn.readfile(path)
  local buf = vim.api.nvim_create_buf(false, true)
  vim.api.nvim_buf_set_lines(buf, 0, -1, false, lines)
  vim.bo[buf].filetype = "json"
  return buf
end

-- Create a json buffer from a raw string (for unit tests without a file).
function M.buf(text)
  local buf = vim.api.nvim_create_buf(false, true)
  vim.api.nvim_buf_set_lines(buf, 0, -1, false, vim.split(text, "\n"))
  vim.bo[buf].filetype = "json"
  return buf
end

return M
