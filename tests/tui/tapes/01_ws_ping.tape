# tests/tui/tapes/01_ws_ping.tape
#
# Demos WebSocket manual ping: connects to a ws:// URL, shows the WS
# panel with the ping hint, and sends a ping frame via the 'p' key.

Output tests/tui/tapes/output/01_ws_ping.gif

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

# Focus URL bar and enter edit mode
Type "	"
Sleep 400ms
Type "e"
Sleep 300ms

# Clear existing URL and type a ws:// address
Home
Sleep 200ms
Backspace 60
Sleep 200ms
Type "ws://localhost:9999"
Sleep 600ms
Enter
Sleep 1200ms

# Send a manual ping
Type "p"
Sleep 800ms

Type "q"
Sleep 500ms
