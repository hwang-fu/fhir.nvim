local M = {}

local did_setup = false

-- Configure fhir.nvim, install detection autocmds, and register the global :FhirEnable.
function M.setup(opts)
  require("fhir.config").setup(opts)
  require("fhir.detect").setup_autocmds()
  if not did_setup then
    vim.api.nvim_create_user_command("FhirEnable", function()
      require("fhir.detect").attach(0)
    end, { desc = "Enable fhir.nvim for the current buffer" })
    did_setup = true
  end
end

-- Public API: jump to the reference under the cursor.
function M.goto_reference()
  require("fhir.features.goto").run()
end

-- Public API: list references to the resource under the cursor.
function M.find_usages()
  require("fhir.features.usages").run()
end

-- Public API: pick a resource from the buffer's outline and jump to it.
function M.outline()
  require("fhir.features.outline").run()
end

-- Public API: evaluate a FHIRPath expression (prompts when expr is nil).
function M.eval(expr)
  require("fhir.features.eval").run(expr)
end

return M
