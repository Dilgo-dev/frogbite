VERSION ?= dev

.PHONY: build install clean fmt lint check tui-test

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

# End-to-end TUI tests driven through tmux. Builds the release binary
# first so the test runner can spawn it. Requires tmux (provided via
# nix-shell on NixOS).
tui-test: build
	nix-shell -p tmux --run ./tests/tui/run.sh
