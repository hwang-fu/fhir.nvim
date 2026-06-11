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

-- Clone the fixture workspace into a temp dir with its own .git marker;
-- returns the root. Tests may write extra files into it.
function M.workspace_clone()
  local src = vim.fn.getcwd() .. "/tests/fixtures/workspace"
  local root = vim.fn.tempname()
  vim.fn.mkdir(root .. "/.git", "p")
  vim.fn.system({ "cp", "-r", src .. "/.", root })
  return root
end

-- Open a real file into a loaded buffer (filetype set for treesitter).
function M.open_file(path)
  local buf = vim.fn.bufadd(path)
  vim.fn.bufload(buf)
  vim.bo[buf].filetype = "json"
  return buf
end

return M
