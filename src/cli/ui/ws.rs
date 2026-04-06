use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use super::{bg, fg, green, muted, orange, red, surface, teal, yellow};
use crate::app::{App, Focus, WsDirection, WsStatus};

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

    let lines: Vec<Line> = app
        .ws
        .messages
        .iter()
        .flat_map(|m| {
            let (marker, color) = match m.direction {
                WsDirection::Sent => ("> ", teal()),
                WsDirection::Recv => ("< ", green()),
                WsDirection::Info => ("- ", muted()),
                WsDirection::Error => ("! ", red()),
            };
            m.text.lines().enumerate().map(move |(i, line)| {
                let prefix = if i == 0 { marker } else { "  " };
                Line::from(vec![
                    Span::styled(prefix, Style::default().fg(color).bold()),
                    Span::styled(line.to_owned(), Style::default().fg(fg())),
                ])
            })
        })
        .collect();

    let paragraph = Paragraph::new(Text::from(lines))
        .block(block)
        .scroll((app.ws.scroll, 0));
    frame.render_widget(paragraph, area);
}

fn draw_input(frame: &mut Frame, app: &App, area: Rect) {
    let editing = app.ws.input_editing;
    let border_color = if editing { orange() } else { muted() };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(
            " message ",
            Style::default().fg(border_color).bold(),
        ))
        .bg(bg());

    let content = if editing {
        format!("{}\u{2588}", app.ws.input)
    } else if app.ws.input.is_empty() {
        match app.ws.status {
            WsStatus::Connected => "press i to type a message".to_owned(),
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
