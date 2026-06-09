vim.opt.runtimepath:prepend(vim.fn.getcwd())
vim.opt.runtimepath:prepend(vim.fn.getcwd() .. "/.tests/plenary.nvim")
vim.opt.runtimepath:prepend(vim.fn.getcwd() .. "/.tests/nvim-treesitter") -- CI-installed json parser (no-op locally)
vim.opt.runtimepath:append(vim.fn.stdpath("data") .. "/site") -- your installed TS json parser
vim.cmd("runtime plugin/plenary.vim")
