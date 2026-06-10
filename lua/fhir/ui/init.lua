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

-- Show `lines` in a scratch floating window at the cursor; q/<Esc> close it.
-- Returns the window and buffer handles.
function M.float(lines, opts)
  opts = opts or {}
  local buf = vim.api.nvim_create_buf(false, true)
  vim.api.nvim_buf_set_lines(buf, 0, -1, false, lines)
  if opts.ft then
    vim.bo[buf].filetype = opts.ft
  end

  local width = 1
  for _, line in ipairs(lines) do
    width = math.max(width, #line)
  end
  width = math.min(width, vim.o.columns - 4)
  local height = math.min(#lines, 12)

  local win = vim.api.nvim_open_win(buf, true, {
    relative = "cursor",
    row = 1,
    col = 0,
    width = width,
    height = height,
    style = "minimal",
    border = "rounded",
  })
  for _, lhs in ipairs({ "q", "<Esc>" }) do
    vim.keymap.set("n", lhs, function()
      if vim.api.nvim_win_is_valid(win) then
        vim.api.nvim_win_close(win, true)
      end
    end, { buffer = buf, nowait = true })
  end
  return win, buf
end

-- Present `items` (each with a `.label`) in a picker; call on_choice(item) on a
-- pick, no-op on cancel. Wraps vim.ui.select (richer adapters can layer on later).
function M.select(items, opts, on_choice)
  vim.ui.select(items, {
    prompt = opts.prompt,
    format_item = function(item)
      return item.label
    end,
  }, function(choice)
    if choice then
      on_choice(choice)
    end
  end)
end

return M
