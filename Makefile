VERSION ?= dev

.PHONY: build install clean fmt lint check

build:
	cargo build --release

install: build
	cp target/release/frogbite ~/.local/bin/frogbite

clean:
	cargo clean

fmt:
	cargo fmt

lint:
	cargo clippy -- -D warnings

check: fmt lint build
