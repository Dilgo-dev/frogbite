use std::io::Write;
use std::process::{Command, Stdio};

pub fn copy_to_clipboard(text: &str) -> Result<(), String> {
    let commands: &[&[&str]] = if cfg!(target_os = "macos") {
        &[&["pbcopy"]]
    } else if cfg!(target_os = "windows") {
        &[&["clip.exe"]]
    } else {
        &[
            &["wl-copy"],
            &["xclip", "-selection", "clipboard"],
            &["xsel", "--clipboard", "--input"],
        ]
    };

    for cmd in commands {
        let Ok(mut child) = Command::new(cmd[0])
            .args(&cmd[1..])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        else {
            continue;
        };
        if let Some(stdin) = child.stdin.as_mut() {
            let _ = stdin.write_all(text.as_bytes());
        }
        if child.wait().is_ok() {
            return Ok(());
        }
    }

    Err("No clipboard tool found (wl-copy, xclip, xsel)".to_owned())
}
