#!/usr/bin/env bash
# End-to-end TUI tests for frogbite, driven through tmux.
#
# Each test spawns frogbite in a detached tmux session, sends a sequence
# of keys, captures the pane as plain text, and asserts the presence (or
# absence) of expected strings. Tests run with an isolated $HOME so they
# never touch the user's real collection.
#
# Run with:
#   make tui-test
# or directly:
#   nix-shell -p tmux --run ./tests/tui/run.sh

cd "$(dirname "$0")/../.."

# shellcheck source=lib.sh
. tests/tui/lib.sh

# ────────────────────────────────────────────────────────────────────
# 01 / smoke: open and close the app
# ────────────────────────────────────────────────────────────────────
test_smoke_boot() {
  local s
  s=$(frog_start smoke) || return 1
  assert_contains "$s" "frogbite" || return 1
  assert_contains "$s" "Request" || return 1
  assert_contains "$s" "Response" || return 1
  frog_stop "$s"
}
run_test "smoke / boots and shows main panes" test_smoke_boot

# ────────────────────────────────────────────────────────────────────
# 02 / settings view opens with the new theme entry
# ────────────────────────────────────────────────────────────────────
test_settings_open() {
  local s
  s=$(frog_start settings) || return 1
  frog_send "$s" s
  assert_contains "$s" "Settings" || return 1
  assert_contains "$s" "Splash animation" || return 1
  assert_contains "$s" "Theme" || return 1
  assert_contains "$s" "scooby" || return 1
  frog_send "$s" Escape
  frog_stop "$s"
}
run_test "settings / opens with theme entry" test_settings_open

# ────────────────────────────────────────────────────────────────────
# 03 / theme cycle persists in settings.json
# ────────────────────────────────────────────────────────────────────
test_theme_cycle() {
  local s
  s=$(frog_start theme) || return 1
  frog_send "$s" s
  # navigate to the Theme entry (4th item, index 3)
  frog_send "$s" j j j
  # cycle once: scooby -> mono
  frog_send "$s" Enter
  assert_contains "$s" "mono" || return 1
  # cycle again: mono -> dracula
  frog_send "$s" Enter
  assert_contains "$s" "dracula" || return 1
  frog_send "$s" Escape
  frog_stop "$s"
  # verify it landed in settings.json
  if grep -q '"theme": "dracula"' "$HOME/.config/frogbite/settings.json"; then
    printf '    \033[32mok\033[0m  settings.json persisted dracula\n'
  else
    printf '    \033[31mFAIL\033[0m  settings.json did not persist dracula\n'
    cat "$HOME/.config/frogbite/settings.json" | sed 's/^/    /'
    return 1
  fi
}
run_test "theme / cycles and persists" test_theme_cycle

# ────────────────────────────────────────────────────────────────────
# 04 / method popup cycles through methods including new ones
# ────────────────────────────────────────────────────────────────────
test_method_popup() {
  local s
  s=$(frog_start method) || return 1
  # focus url bar
  frog_send "$s" Tab
  # open method popup
  frog_send "$s" m
  assert_contains "$s" "GET" || return 1
  assert_contains "$s" "POST" || return 1
  assert_contains "$s" "GRPC" || return 1
  assert_contains "$s" "GQL" || return 1
  frog_send "$s" Escape
  frog_stop "$s"
}
run_test "method popup / lists HTTP + GRPC + GQL" test_method_popup

# ────────────────────────────────────────────────────────────────────
# 05 / switching to GRPC tags the Request title
# ────────────────────────────────────────────────────────────────────
test_grpc_indicator() {
  local s
  s=$(frog_start grpc) || return 1
  frog_send "$s" Tab
  frog_send "$s" m
  # GRPC is the 8th method (index 7), navigate down 7 times then Enter
  frog_send "$s" j j j j j j j Enter
  assert_contains "$s" "[grpc:" || return 1
  frog_stop "$s"
}
run_test "grpc / Request title shows [grpc: no method]" test_grpc_indicator

# ────────────────────────────────────────────────────────────────────
# 06 / switching to GQL tags the Request title
# ────────────────────────────────────────────────────────────────────
test_graphql_indicator() {
  local s
  s=$(frog_start gql) || return 1
  frog_send "$s" Tab
  frog_send "$s" m
  # GQL is the 9th method (index 8)
  frog_send "$s" j j j j j j j j Enter
  assert_contains "$s" "[graphql]" || return 1
  frog_stop "$s"
}
run_test "graphql / Request title shows [graphql]" test_graphql_indicator

# ────────────────────────────────────────────────────────────────────
# 07 / proxy popup saves a value and shows it in the title
# ────────────────────────────────────────────────────────────────────
test_proxy_popup() {
  local s
  s=$(frog_start proxy) || return 1
  frog_send "$s" Tab
  frog_send "$s" Y
  assert_contains "$s" "Proxy URL" || return 1
  frog_type "$s" "http://localhost:9999"
  frog_send "$s" Enter
  assert_contains "$s" "[proxy: http://localhost:9999]" || return 1
  frog_stop "$s"
}
run_test "proxy / popup saves and shows in title" test_proxy_popup

# ────────────────────────────────────────────────────────────────────
# 08 / plugins popup saves a list and shows the count
# ────────────────────────────────────────────────────────────────────
test_plugins_popup() {
  local s
  s=$(frog_start plugins) || return 1
  frog_send "$s" Tab
  frog_send "$s" Z
  assert_contains "$s" "Plugins" || return 1
  frog_type "$s" "stamp, auth"
  frog_send "$s" Enter
  assert_contains "$s" "[plugins: 2]" || return 1
  frog_stop "$s"
}
run_test "plugins / popup saves list and shows count" test_plugins_popup

# ────────────────────────────────────────────────────────────────────
# 09 / sidebar navigation does not crash and shows requests
# ────────────────────────────────────────────────────────────────────
test_sidebar_nav() {
  local s
  s=$(frog_start sidebar) || return 1
  assert_contains "$s" "Examples" || return 1
  frog_send "$s" j j Enter
  assert_contains "$s" "https://" || return 1
  frog_stop "$s"
}
run_test "sidebar / navigates default Examples folder" test_sidebar_nav

# ────────────────────────────────────────────────────────────────────
# 10 / ws panel activates for ws:// URL and shows stream
# ────────────────────────────────────────────────────────────────────
test_ws_panel() {
  local s
  s=$(frog_start ws_panel) || return 1
  # focus UrlBar and enter edit mode
  frog_send "$s" Tab
  frog_send "$s" e
  # clear existing URL: End to be sure we're at the end, then BSpace many times
  frog_send "$s" End
  for _ in $(seq 1 60); do tmux send-keys -t "$s" BSpace; done
  sleep 0.4
  frog_type "$s" "ws://localhost:9999"
  frog_send "$s" Enter
  sleep 0.8
  assert_contains "$s" "WS" || return 1
  assert_contains "$s" "connecting to ws://localhost:9999" || return 1
  assert_contains "$s" "message" || return 1
  frog_send "$s" x
  frog_stop "$s"
}
run_test "ws / panel activates for ws:// URL" test_ws_panel

summary
