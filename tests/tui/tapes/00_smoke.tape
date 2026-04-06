# tests/tui/tapes/00_smoke.tape
#
# Reference VHS tape: boots frogbite, opens settings, cycles the theme,
# and quits. Used as the canonical demo of a feature for the autorunner
# Discord upload. Copy this file as a starting point when adding a new
# tape for a new feature.

Output tests/tui/tapes/output/00_smoke.gif

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

# Open settings, cycle theme to dracula, exit
Type "s"
Sleep 600ms
Type "jjj"
Sleep 400ms
Enter
Sleep 400ms
Enter
Sleep 600ms
Escape
Sleep 400ms
Type "q"
Sleep 500ms
