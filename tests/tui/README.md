# tests/tui

End-to-end TUI tests for frogbite, driven through tmux.

Each test spawns the release binary inside a detached tmux session,
sends a sequence of keys, captures the pane as plain text, and asserts
on the presence (or absence) of expected strings. Tests run with an
isolated `$HOME` (a fresh `mktemp -d`) so they never touch the user's
real `~/.config/frogbite/` collection or settings.

## Run

```bash
make tui-test
```

That target builds the release binary if needed and runs the suite
inside `nix-shell -p tmux`. To run by hand:

```bash
cargo build --release
nix-shell -p tmux --run ./tests/tui/run.sh
```

## Layout

- `lib.sh` - shared helpers (`frog_start`, `frog_send`, `frog_type`,
  `frog_capture`, `frog_stop`, `assert_contains`, `assert_missing`,
  `run_test`, `summary`).
- `run.sh` - the actual scenarios. One bash function per scenario,
  registered with `run_test`.

## Adding a new test

Append a new function to `run.sh`:

```bash
test_my_feature() {
  local s
  s=$(frog_start my-feature) || return 1
  frog_send "$s" Tab          # focus url bar
  frog_send "$s" Y            # open proxy popup
  frog_type "$s" "http://x"   # type literal text
  frog_send "$s" Enter
  assert_contains "$s" "[proxy: http://x]" || return 1
  frog_stop "$s"
}
run_test "my feature / does the thing" test_my_feature
```

Use `frog_send` for tmux key names (`Enter`, `Escape`, `BSpace`,
`Tab`, `BTab`, `Up`, `Down`, `C-c`, ...) and `frog_type` for literal
text. Insert short sleeps inside helpers, not the test, so scenarios
stay readable.
