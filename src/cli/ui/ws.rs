use std::fmt::Write as _;

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use super::{bg, fg, green, muted, orange, red, surface, teal, yellow};
use crate::app::{App, Focus, WsDirection, WsFormat, WsStatus};

pub(super) fn draw_ws_panel(frame: &mut Frame, app: &App, area: Rect) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(area);

    draw_status_bar(frame, app, layout[0]);
    draw_stream(frame, app, layout[1]);
    draw_input(frame, app, layout[2]);
}

fn draw_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let (label, color) = match app.ws.status {
        WsStatus::Disconnected => (" DISCONNECTED ", muted()),
        WsStatus::Connecting => (" CONNECTING ", yellow()),
        WsStatus::Connected => (" CONNECTED ", green()),
        WsStatus::Closed => (" CLOSED ", muted()),
    };
    let line = Line::from(vec![
        Span::styled(label, Style::default().fg(bg()).bg(color).bold()),
        Span::raw("  "),
        Span::styled("WS", Style::default().fg(teal()).bold()),
        Span::raw("  "),
        Span::styled(
            format!("{} msg", app.ws.messages.len()),
            Style::default().fg(muted()),
        ),
        Span::raw("  "),
        Span::styled(
            format!("in:{}", app.ws.input_format.label()),
            Style::default().fg(orange()).bold(),
        ),
        Span::raw("  "),
        Span::styled(
            format!("view:{}", app.ws.view_format.label()),
            Style::default().fg(teal()).bold(),
        ),
    ]);
    frame.render_widget(Paragraph::new(line).bg(surface()), area);
}

fn draw_stream(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.ui.focus == Focus::Response;
    let block = Block::default()
        .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
        .border_style(if is_focused {
            Style::default().fg(green())
        } else {
            Style::default().fg(muted())
        })
        .bg(bg());

    let view_format = app.ws.view_format;
    let mut lines: Vec<Line> = Vec::new();
    for m in &app.ws.messages {
        let (marker, color) = match m.direction {
            WsDirection::Sent => ("> ", teal()),
            WsDirection::Recv => ("< ", green()),
            WsDirection::Info => ("- ", muted()),
            WsDirection::Error => ("! ", red()),
        };
        if let Some(data) = &m.data {
            // Header line: marker + size annotation
            lines.push(Line::from(vec![
                Span::styled(marker, Style::default().fg(color).bold()),
                Span::styled(
                    format!("binary {} bytes", data.len()),
                    Style::default().fg(orange()),
                ),
            ]));
            match view_format {
                WsFormat::Hex | WsFormat::Text => {
                    for hex_line in hex_dump(data) {
                        lines.push(Line::from(vec![
                            Span::styled("  ", Style::default()),
                            Span::styled(hex_line, Style::default().fg(fg())),
                        ]));
                    }
                }
                WsFormat::Base64 => {
                    let b64 = encode_base64(data);
                    for chunk in b64.as_bytes().chunks(64) {
                        if let Ok(s) = std::str::from_utf8(chunk) {
                            lines.push(Line::from(vec![
                                Span::styled("  ", Style::default()),
                                Span::styled(s.to_owned(), Style::default().fg(fg())),
                            ]));
                        }
                    }
                }
            }
        } else {
            for (i, line) in m.text.lines().enumerate() {
                let prefix = if i == 0 { marker } else { "  " };
                lines.push(Line::from(vec![
                    Span::styled(prefix, Style::default().fg(color).bold()),
                    Span::styled(line.to_owned(), Style::default().fg(fg())),
                ]));
            }
        }
    }

    let paragraph = Paragraph::new(Text::from(lines))
        .block(block)
        .scroll((app.ws.scroll, 0));
    frame.render_widget(paragraph, area);
}

fn draw_input(frame: &mut Frame, app: &App, area: Rect) {
    let editing = app.ws.input_editing;
    let border_color = if editing { orange() } else { muted() };
    let title = format!(" message [{}] ", app.ws.input_format.label());
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(
            title,
            Style::default().fg(border_color).bold(),
        ))
        .bg(bg());

    let content = if editing {
        format!("{}\u{2588}", app.ws.input)
    } else if app.ws.input.is_empty() {
        match app.ws.status {
            WsStatus::Connected => match app.ws.input_format {
                WsFormat::Text => "i:edit  b:fmt  u:upload".to_owned(),
                WsFormat::Hex => "type hex pairs (b:fmt  u:upload)".to_owned(),
                WsFormat::Base64 => "type base64 (b:fmt  u:upload)".to_owned(),
            },
            _ => String::new(),
        }
    } else {
        app.ws.input.clone()
    };

    let color = if editing {
        fg()
    } else if app.ws.input.is_empty() {
        muted()
    } else {
        fg()
    };

    frame.render_widget(
        Paragraph::new(Line::styled(content, Style::default().fg(color))).block(block),
        area,
    );
}

/// Renders a hex dump of `data` with 16 bytes per line, offset prefix and
/// ASCII gutter, like `xxd`.
fn hex_dump(data: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    for (i, chunk) in data.chunks(16).enumerate() {
        let mut s = format!("{:08x}  ", i * 16);
        for (j, b) in chunk.iter().enumerate() {
            if j == 8 {
                s.push(' ');
            }
            let _ = write!(s, "{b:02x} ");
        }
        for j in chunk.len()..16 {
            if j == 8 {
                s.push(' ');
            }
            s.push_str("   ");
        }
        s.push_str(" |");
        for b in chunk {
            if (0x20..0x7f).contains(b) {
                s.push(*b as char);
            } else {
                s.push('.');
            }
        }
        s.push('|');
        out.push(s);
    }
    out
}

const BASE64_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn encode_base64(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0];
        let b1 = chunk.get(1).copied().unwrap_or(0);
        let b2 = chunk.get(2).copied().unwrap_or(0);
        out.push(BASE64_ALPHABET[(b0 >> 2) as usize] as char);
        out.push(BASE64_ALPHABET[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            out.push(BASE64_ALPHABET[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(BASE64_ALPHABET[(b2 & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}
