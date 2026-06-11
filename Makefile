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

SCHEMA_CACHE := $(SCRATCH)/r4-definitions
SCHEMA_BASE  := https://hl7.org/fhir/R4
SCHEMA_OUT   := crates/fhir-core/src/schema/generated.rs

CORPUS_DIR   := $(SCRATCH)/r4-examples
CORPUS_URL   := $(SCHEMA_BASE)/examples-json.zip

.PHONY: all test lint clean build schema corpus

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

schema:
	mkdir -p $(SCHEMA_CACHE)
	curl -fsSL -o $(SCHEMA_CACHE)/profiles-types.json $(SCHEMA_BASE)/profiles-types.json
	curl -fsSL -o $(SCHEMA_CACHE)/profiles-resources.json $(SCHEMA_BASE)/profiles-resources.json
	$(CARGO) run -p fhir-schema-gen -- $(SCHEMA_CACHE)/profiles-types.json $(SCHEMA_CACHE)/profiles-resources.json $(SCHEMA_OUT) "$(SCHEMA_BASE) definitions, fetched $$(date -u +%Y-%m-%d)"

corpus:
	test -f $(CORPUS_DIR)/.complete || ( \
	  mkdir -p $(CORPUS_DIR) && \
	  curl -fsSL -o $(CORPUS_DIR)/examples.zip $(CORPUS_URL) && \
	  unzip -oq $(CORPUS_DIR)/examples.zip -d $(CORPUS_DIR) && \
	  rm $(CORPUS_DIR)/examples.zip && \
	  touch $(CORPUS_DIR)/.complete )

clean:
	rm -rf $(SCRATCH)
