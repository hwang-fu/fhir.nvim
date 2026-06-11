local M = {}

local defaults = {
  detection = "auto",
  keymaps = {},
  -- `tag` pins which engine release :FhirFetchEngine installs and the loader
  -- looks for; `dir` (unset by default) overrides the search path entirely.
  native = { tag = "v3.0.0" },
  -- on_save re-validates attached buffers after every write (when the
  -- engine is present); :FhirValidate always works manually.
  validate = { on_save = true },
  -- Cross-file features look for json under the buffer's git root (cwd
  -- when there is none), skipping `ignore` directories; `max_files`
  -- bounds the scan and clipping is reported.
  workspace = {
    ignore = { ".git", "node_modules", "target", ".tests" },
    max_files = 2000,
  },
}

-- Active config; initialized from defaults so get() works before setup() is called.
local current = vim.deepcopy(defaults)

local function validate(opts)
  if opts.detection ~= nil and opts.detection ~= "auto" and opts.detection ~= "manual" then
    error("fhir: invalid detection mode: " .. tostring(opts.detection))
  end
  if opts.native ~= nil and opts.native.dir ~= nil and type(opts.native.dir) ~= "string" then
    error("fhir: native.dir must be a string")
  end
  if opts.native ~= nil and opts.native.tag ~= nil and type(opts.native.tag) ~= "string" then
    error("fhir: native.tag must be a string")
  end
  if
    opts.validate ~= nil
    and opts.validate.on_save ~= nil
    and type(opts.validate.on_save) ~= "boolean"
  then
    error("fhir: validate.on_save must be a boolean")
  end
  if opts.workspace ~= nil then
    if opts.workspace.ignore ~= nil and type(opts.workspace.ignore) ~= "table" then
      error("fhir: workspace.ignore must be a list of directory names")
    end
    if opts.workspace.max_files ~= nil and type(opts.workspace.max_files) ~= "number" then
      error("fhir: workspace.max_files must be a number")
    end
  end
end

function M.setup(opts)
  opts = opts or {}
  validate(opts)
  -- "force": user values win at the leaf, untouched defaults survive.
  current = vim.tbl_deep_extend("force", defaults, opts)
  -- list options replace wholesale: index-wise merging would leave default
  -- tail entries behind a shorter user list
  if opts.workspace ~= nil and opts.workspace.ignore ~= nil then
    current.workspace.ignore = opts.workspace.ignore
  end
end

function M.get()
  return current
end

return M
