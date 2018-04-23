CARGO = cargo

all: build

prebuild:
	@mkdir -p build

build: prebuild
	@cd rust && \
	$(CARGO) build --release --target wasm32-unknown-unknown --verbose && \
	cp target/wasm32-unknown-unknown/release/wasm_gb.wasm ../build

test:
	@cd rust && $(CARGO) test

clean:
	@cd rust && $(CARGO) clean

.PHONY: all prebuild build test clean