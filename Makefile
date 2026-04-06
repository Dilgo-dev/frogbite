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

# Records every .tape file in tests/tui/tapes/ as an animated gif under
# tests/tui/tapes/output/. The autorunner uses these as Discord
# attachments. Requires vhs (provided via nix-shell on NixOS).
tui-record: build
	mkdir -p tests/tui/tapes/output
	nix-shell -p vhs ttyd ffmpeg --run 'for tape in tests/tui/tapes/*.tape; do echo "▶ $$tape"; vhs "$$tape" || exit 1; done'

# Records a single tape by name (without extension or path):
#   make tui-record-one TAPE=00_smoke
tui-record-one: build
	mkdir -p tests/tui/tapes/output
	nix-shell -p vhs ttyd ffmpeg --run 'vhs tests/tui/tapes/$(TAPE).tape'
