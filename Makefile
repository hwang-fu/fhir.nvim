PLENARY := .tests/plenary.nvim

$(PLENARY):
	git clone --depth 1 https://github.com/nvim-lua/plenary.nvim $@

.PHONY: test lint
test: $(PLENARY)
	nvim --headless --noplugin -u tests/minimal_init.lua -c "PlenaryBustedDirectory tests/fhir/ {minimal_init='tests/minimal_init.lua'}"

lint:
	stylua --check lua tests && luacheck lua
