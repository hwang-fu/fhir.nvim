NVIM     ?= nvim
STYLUA   ?= stylua
LUACHECK ?= luacheck

LUA_SRC      := lua
LUA_TEST     := tests
SPEC_DIR     := $(LUA_TEST)/fhir/
MINIMAL_INIT := $(LUA_TEST)/minimal_init.lua

SCRATCH      := .tests
PLENARY      := $(SCRATCH)/plenary.nvim
PLENARY_URL  := https://github.com/nvim-lua/plenary.nvim

CARGO        ?= cargo
CDYLIB       := target/release/libfhir_core.so
NATIVE       := $(SCRATCH)/fhir_core.so

.PHONY: all test lint clean build

all: lint test

$(PLENARY):
	git clone --depth 1 $(PLENARY_URL) $@

test: $(PLENARY)
	$(NVIM) --headless --noplugin -u $(MINIMAL_INIT) -c "PlenaryBustedDirectory $(SPEC_DIR) {minimal_init='$(MINIMAL_INIT)'}"

lint:
	$(STYLUA) --check $(LUA_SRC) $(LUA_TEST)
	$(LUACHECK) $(LUA_SRC)

build:
	$(CARGO) build --release
	mkdir -p $(SCRATCH)
	cp $(CDYLIB) $(NATIVE)

clean:
	rm -rf $(SCRATCH)
