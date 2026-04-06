# tests/tui/tapes/03_ws_save_replay.tape
#
# Demos WebSocket session save and replay popup. Connects to a ws://
# endpoint, saves the message stream to JSON, then opens the replay
# popup to show the workflow.

Output tests/tui/tapes/output/03_ws_save_replay.gif

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

# Focus URL bar, enter ws:// URL
Type "	"
Type "e"
Sleep 300ms
# Clear existing URL
Type@0ms "${END}"
Backspace 60
Sleep 300ms
Type "ws://echo.websocket.org"
Enter
Sleep 1500ms

# Save the stream with S
Type "S"
Sleep 800ms

# Open replay popup with L (won't open if disconnected)
# Show the help bar with S/L hints
Sleep 1500ms

# Quit
Escape
Sleep 200ms
Type "x"
Sleep 300ms
Type "q"
Sleep 500ms
