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
