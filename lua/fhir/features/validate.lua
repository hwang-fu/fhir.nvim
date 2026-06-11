local index = require("fhir.index")
local native = require("fhir.native")
local parse = require("fhir.parse")
local pathmap = require("fhir.pathmap")
local resolver = require("fhir.resolver")
local resolver_ws = require("fhir.resolver.workspace")
local ui = require("fhir.ui")
local workspace = require("fhir.workspace")

local M = {}

local ns = vim.api.nvim_create_namespace("fhir.validate")

local severities = {
  error = vim.diagnostic.severity.ERROR,
  warning = vim.diagnostic.severity.WARN,
  information = vim.diagnostic.severity.INFO,
}

-- Validate a buffer's top-level document and publish the findings.
-- The whole document, not the resource under the cursor: in a Bundle the
-- index lists the entries, but findings anchor at the document root.
-- `quiet` suppresses the notifications (the workspace sweep's refresh).
local function publish(buf, quiet)
  local idx = index.get(buf)
  local root = parse.root(buf)
  if not root or #idx.resources == 0 then
    if not quiet then
      ui.notify("no resource in this buffer", vim.log.levels.INFO)
    end
    return
  end
  -- when the document is itself an indexed resource, passing it as the
  -- resolver's owner makes its contained (#id) references resolvable
  local owner
  for _, r in ipairs(idx.resources) do
    if r.node == root then
      owner = r
    end
  end

  local json = vim.treesitter.get_node_text(root, buf)
  local result, err = native.validate(json, resolver.callback(idx, owner))
  if err then
    if not quiet then
      ui.notify(err, vim.log.levels.ERROR)
    end
    return
  end

  local diags = {}
  for _, issue in ipairs(vim.json.decode(result)) do
    local r = pathmap.range(root, buf, issue.path)
    diags[#diags + 1] = {
      lnum = r[1],
      col = r[2],
      end_lnum = r[3],
      end_col = r[4],
      severity = severities[issue.severity] or vim.diagnostic.severity.ERROR,
      message = issue.message,
      source = "fhir",
    }
  end
  vim.diagnostic.set(ns, buf, diags)
end

-- Validate the current buffer's document and publish the findings.
function M.run()
  publish(vim.api.nvim_get_current_buf())
end

-- Drop a buffer's findings (the current buffer when unspecified).
function M.clear(buf)
  vim.diagnostic.reset(ns, buf or vim.api.nvim_get_current_buf())
end

local qf_types = { error = "E", warning = "W", information = "I" }

-- Validate every workspace document (raw disk text, no resolver - the
-- lenient verdict rules cover unresolvable references) and collect the
-- findings into the quickfix list. `scope == "all"` includes advisory
-- findings; the default keeps errors only. The summary counts everything
-- either way.
function M.run_workspace(scope)
  if not native.available() then
    ui.notify("FHIRPath engine not available (run `make build`)", vim.log.levels.ERROR)
    return
  end
  local file = vim.api.nvim_buf_get_name(0)
  local root = workspace.root(file ~= "" and file or vim.uv.cwd())
  local files = workspace.files(root)
  ui.notify(("validating %d files..."):format(#files))

  local counts = { error = 0, warning = 0, information = 0 }
  local skipped = 0
  local entries = {}
  for _, f in ipairs(files) do
    local ok, lines = pcall(vim.fn.readfile, f)
    local result, err
    if ok then
      result, err = native.validate(table.concat(lines, "\n"))
    end
    if not ok or err then
      skipped = skipped + 1 -- unreadable, unparseable, or not a resource
    else
      local kept = {}
      for _, issue in ipairs(vim.json.decode(result)) do
        counts[issue.severity] = (counts[issue.severity] or 0) + 1
        if scope == "all" or issue.severity == "error" then
          kept[#kept + 1] = issue
        end
      end
      if #kept > 0 then
        -- only files with findings pay for a buffer: pathmap places each
        -- entry at its exact element
        local buf = resolver_ws.open_file(f)
        local doc = parse.root(buf)
        for _, issue in ipairs(kept) do
          local r = doc and pathmap.range(doc, buf, issue.path) or { 0, 0, 0, 0 }
          entries[#entries + 1] = {
            filename = f,
            lnum = r[1] + 1,
            col = r[2] + 1,
            type = qf_types[issue.severity] or "E",
            text = ("[%s] %s"):format(issue.severity, issue.message),
          }
        end
      end
    end
  end

  vim.fn.setqflist({}, " ", { title = "FHIR workspace validation", items = entries })

  -- buffers already open get their live view refreshed too (their own
  -- text, full resolver fidelity - the editor's view of a file wins)
  for _, f in ipairs(files) do
    local buf = vim.fn.bufnr(f)
    if buf ~= -1 and vim.fn.bufloaded(buf) == 1 and vim.b[buf].fhir_attached then
      publish(buf, true)
    end
  end

  local summary = ("%d files validated, %d skipped: %d errors, %d warnings, %d informational"):format(
    #files - skipped,
    skipped,
    counts.error,
    counts.warning,
    counts.information
  )
  if #entries > 0 then
    summary = summary .. " - :copen to browse"
  end
  ui.notify(summary, counts.error > 0 and vim.log.levels.WARN or vim.log.levels.INFO)
end

-- The write hook: quietly does nothing when the engine is absent.
function M.on_save()
  if not native.available() then
    return
  end
  M.run()
end

return M
