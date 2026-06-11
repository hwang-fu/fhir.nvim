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
end

function M.setup(opts)
  opts = opts or {}
  validate(opts)
  -- "force": user values win at the leaf, untouched defaults survive.
  current = vim.tbl_deep_extend("force", defaults, opts)
end

function M.get()
  return current
end

return M
