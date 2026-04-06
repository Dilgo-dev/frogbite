#!/usr/bin/env bash
# Shared helpers for TUI end-to-end tests driven through tmux.
#
# Each test gets an isolated $HOME (so it cannot see or stomp on the
# user's real ~/.config/frogbite/) and its own tmux session. The session
# pane can be inspected as plain text via `frog_capture` and walked with
# `assert_contains` / `assert_missing`.

set -uo pipefail

FROGBITE_BIN="${FROGBITE_BIN:-./target/release/frogbite}"
SESSION_PREFIX="fb-test-$$"
PASS_COUNT=0
FAIL_COUNT=0
FAILED_TESTS=()

if ! command -v tmux >/dev/null 2>&1; then
  echo "error: tmux not found in PATH (run via nix-shell -p tmux or install tmux)" >&2
  exit 2
fi
if [ ! -x "$FROGBITE_BIN" ]; then
  echo "error: $FROGBITE_BIN not found, run 'cargo build --release' first" >&2
  exit 2
fi

# Per-run sandbox so tests never touch the user's real collection.
TEST_HOME="$(mktemp -d -t frogbite-tui-test.XXXXXX)"
mkdir -p "$TEST_HOME/.config/frogbite"
cat > "$TEST_HOME/.config/frogbite/settings.json" <<'JSON'
{
  "splash_animation": false,
  "vim_keys": true,
  "update_check": false,
  "theme": "scooby"
}
JSON
export HOME="$TEST_HOME"

cleanup() {
  for s in $(tmux ls 2>/dev/null | awk -F: '/^'"$SESSION_PREFIX"'/ {print $1}'); do
    tmux kill-session -t "$s" 2>/dev/null || true
  done
  rm -rf "$TEST_HOME"
}
trap cleanup EXIT

frog_start() {
  local name="${1:-default}"
  local session="${SESSION_PREFIX}-${name}"
  tmux kill-session -t "$session" 2>/dev/null || true
  tmux new-session -d -s "$session" -x 140 -y 40 "$FROGBITE_BIN"
  sleep 0.6
  printf '%s' "$session"
}

# Send keys to the session. All trailing args are passed to send-keys.
# Use tmux key names for specials: Enter, Escape, BSpace, Up, Down, Tab, BTab, C-c.
frog_send() {
  local session=$1
  shift
  tmux send-keys -t "$session" "$@"
  sleep 0.18
}

# Send literal text (no key-name interpretation).
frog_type() {
  local session=$1
  local text=$2
  tmux send-keys -t "$session" -l "$text"
  sleep 0.15
}

frog_capture() {
  local session=$1
  tmux capture-pane -t "$session" -p
}

frog_stop() {
  local session=$1
  tmux send-keys -t "$session" Escape 2>/dev/null || true
  sleep 0.05
  tmux send-keys -t "$session" q 2>/dev/null || true
  sleep 0.1
  tmux kill-session -t "$session" 2>/dev/null || true
}

assert_contains() {
  local session=$1
  local needle=$2
  local pane
  pane=$(frog_capture "$session")
  if printf '%s\n' "$pane" | grep -qF -- "$needle"; then
    printf '    \033[32mok\033[0m  contains: %s\n' "$needle"
  else
    printf '    \033[31mFAIL\033[0m  missing: %s\n' "$needle"
    echo '    ----- pane -----'
    printf '%s\n' "$pane" | sed 's/^/    /'
    echo '    ---- /pane -----'
    return 1
  fi
}

assert_missing() {
  local session=$1
  local needle=$2
  local pane
  pane=$(frog_capture "$session")
  if printf '%s\n' "$pane" | grep -qF -- "$needle"; then
    printf '    \033[31mFAIL\033[0m  unexpected: %s\n' "$needle"
    return 1
  else
    printf '    \033[32mok\033[0m  absent: %s\n' "$needle"
  fi
}

run_test() {
  local name=$1
  local fn=$2
  printf '\n\033[1m▶\033[0m %s\n' "$name"
  if "$fn"; then
    printf '  \033[32mPASS\033[0m\n'
    PASS_COUNT=$((PASS_COUNT + 1))
  else
    printf '  \033[31mFAIL\033[0m\n'
    FAIL_COUNT=$((FAIL_COUNT + 1))
    FAILED_TESTS+=("$name")
  fi
}

summary() {
  printf '\n═══════════════════════════════════════\n'
  if [ "$FAIL_COUNT" -eq 0 ]; then
    printf '  \033[32m%s passed\033[0m, %s failed\n' "$PASS_COUNT" "$FAIL_COUNT"
  else
    printf '  %s passed, \033[31m%s failed\033[0m\n' "$PASS_COUNT" "$FAIL_COUNT"
    for t in "${FAILED_TESTS[@]}"; do
      printf '    - %s\n' "$t"
    done
  fi
  printf '═══════════════════════════════════════\n'
  if [ "$FAIL_COUNT" -gt 0 ]; then
    exit 1
  fi
}
