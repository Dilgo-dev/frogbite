# tests/tui/tapes/02_ws_reconnect.tape
#
# Demos WebSocket auto-reconnect: connects to a ws:// URL, enables
# auto-reconnect with R, shows the reconnect:on indicator, then
# toggles it back off.

Output tests/tui/tapes/output/02_ws_reconnect.gif

Set Shell "bash"
Set FontSize 14
Set Width 1280
Set Height 720
Set Theme "Catppuccin Mocha"
Set TypingSpeed 60ms

Hide
Type "export HOME=$(mktemp -d) && mkdir -p $HOME/.config/frogbite && cat > $HOME/.config/frogbite/settings.json <<JSON"
Enter
Type '{"splash_animation": false, "vim_keys": true, "update_check": false, "theme": "scooby"}'
Enter
Type "JSON"
Enter
Show

Type "./target/release/frogbite"
Enter
Sleep 1500ms

# Focus URL bar, enter edit mode, type a ws:// URL
Type "	"
Sleep 300ms
Type "e"
Sleep 200ms
# Clear existing URL
Type@0ms ""
Backspace 60
Sleep 200ms
Type "ws://localhost:9999"
Sleep 400ms
Enter
Sleep 1200ms

# Enable auto-reconnect
Type "R"
Sleep 800ms

# Show the reconnect:on indicator for a moment
Sleep 2000ms

# Disable auto-reconnect
Type "R"
Sleep 800ms

# Close WS and quit
Type "x"
Sleep 400ms
Type "q"
Sleep 500ms
