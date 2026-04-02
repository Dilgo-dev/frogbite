VERSION ?= dev

.PHONY: build install clean fmt lint check app app-dev

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

app:
	cd src/app && cargo tauri build

app-dev:
	cd src/app && cargo tauri dev
