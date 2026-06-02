default: build

all: test

test: build
	cargo test

build:
	stellar contract build
	@ls -l ../../target/wasm32v1-none/release/*.wasm

size:
	@stat -f%z ../../target/wasm32v1-none/release/boundless_profile.wasm 2>/dev/null || stat -c%s ../../target/wasm32v1-none/release/boundless_profile.wasm

fmt:
	cargo fmt --all

clean:
	cargo clean
