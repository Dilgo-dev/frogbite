use ratatui::prelude::*;
use ratatui::widgets::{
    Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
};

use super::*;
use crate::app::{App, Focus, ResponseTab};

pub(super) fn draw_response(frame: &mut Frame, app: &App, area: Rect) {
    let has_search = app.response_searching || !app.response_search.is_empty();
    let constraints = if has_search {
        vec![
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Min(1),
        ]
    } else {
        vec![Constraint::Length(2), Constraint::Min(1)]
    };
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    draw_response_status_bar(frame, app, layout[0]);

    if has_search {
        draw_search_bar(frame, app, layout[1]);
        draw_response_content(frame, app, layout[2]);
    } else {
        draw_response_content(frame, app, layout[1]);
    }
}

fn draw_search_bar(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.focus == Focus::Response;
    let border_style = if is_focused {
        Style::default().fg(GREEN)
    } else {
        Style::default().fg(MUTED)
    };

    let display = if app.response_searching {
        format!(" /{}\u{2588}", &app.response_search_buf)
    } else {
        format!(" /{}", &app.response_search)
    };

    let line = Line::from(vec![
        Span::styled(
            display,
            Style::default().fg(if app.response_searching { GREEN } else { MUTED }),
        ),
        Span::styled("  n:next  N:prev  Esc:clear", Style::default().fg(MUTED)),
    ]);

    frame.render_widget(
        Paragraph::new(line).bg(SURFACE).block(
            Block::default()
                .borders(Borders::LEFT | Borders::RIGHT)
                .border_style(border_style),
        ),
        area,
    );
}

fn draw_response_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let status_line = match &app.response {
        Some(Ok(resp)) => {
            let color = status_color(resp.status);
            let mut spans = vec![
                Span::styled(
                    format!(" {} ", resp.status_text),
                    Style::default().fg(BG).bg(color).bold(),
                ),
                Span::raw("  "),
                Span::styled(
                    format!("{}ms", resp.duration_ms),
                    Style::default().fg(MUTED),
                ),
                Span::raw("  "),
                Span::styled(format_size(resp.body.len()), Style::default().fg(MUTED)),
                if resp.redirect_chain.is_empty() {
                    Span::raw("")
                } else {
                    Span::styled(
                        format!("  \u{21aa}{}", resp.redirect_chain.len()),
                        Style::default().fg(TEAL),
                    )
                },
                Span::raw("    "),
                if app.response_tab == ResponseTab::Body {
                    Span::styled("Body", Style::default().fg(GREEN).bold().underlined())
                } else {
                    Span::styled("Body", Style::default().fg(MUTED))
                },
                Span::raw("  "),
                if app.response_tab == ResponseTab::Headers {
                    Span::styled("Headers", Style::default().fg(GREEN).bold().underlined())
                } else {
                    Span::styled("Headers", Style::default().fg(MUTED))
                },
            ];
            if let Some(msg) = &app.clipboard_msg {
                spans.push(Span::raw("    "));
                spans.push(Span::styled(msg, Style::default().fg(GREEN).bold()));
            }
            Line::from(spans)
        }
        Some(Err(_)) => Line::from(vec![Span::styled(
            " ERROR ",
            Style::default().fg(BG).bg(RED).bold(),
        )]),
        None if app.loading => Line::from(vec![Span::styled(
            " Sending... ",
            Style::default().fg(YELLOW),
        )]),
        None => Line::from(vec![Span::styled(" Response ", Style::default().fg(MUTED))]),
    };

    frame.render_widget(Paragraph::new(status_line).bg(SURFACE), area);
}

fn draw_response_content(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.focus == Focus::Response;

    let block = Block::default()
        .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
        .border_style(if is_focused {
            Style::default().fg(GREEN)
        } else {
            Style::default().fg(MUTED)
        })
        .bg(BG);

    let inner = block.inner(area);

    match &app.response {
        Some(Ok(resp)) if app.response_tab == ResponseTab::Headers => {
            let mut lines: Vec<Line> = Vec::new();
            if !resp.redirect_chain.is_empty() {
                lines.push(Line::from(Span::styled(
                    format!("Redirect chain ({}):", resp.redirect_chain.len()),
                    Style::default().fg(TEAL).bold(),
                )));
                for (i, url) in resp.redirect_chain.iter().enumerate() {
                    lines.push(Line::from(vec![
                        Span::styled(format!("  {}. ", i + 1), Style::default().fg(MUTED)),
                        Span::styled(url, Style::default().fg(FG)),
                    ]));
                }
                lines.push(Line::default());
            }
            lines.extend(resp.headers.iter().map(|(k, v)| {
                Line::from(vec![
                    Span::styled(k, Style::default().fg(MUTED)),
                    Span::styled(": ", Style::default().fg(MUTED)),
                    Span::styled(v, Style::default().fg(FG)),
                ])
            }));
            let paragraph = Paragraph::new(Text::from(lines))
                .block(block)
                .scroll((app.response_scroll, 0));
            frame.render_widget(paragraph, area);
        }
        Some(Ok(_) | Err(_)) => {
            let body = app.formatted_response_body();
            let is_json = body.starts_with('{') || body.starts_with('[');

            let search = &app.response_search;
            let lines: Vec<Line> = body
                .lines()
                .enumerate()
                .map(|(i, line)| {
                    let mut spans = vec![Span::styled(
                        format!("{:>3} ", i + 1),
                        Style::default().fg(MUTED),
                    )];
                    if !search.is_empty()
                        && line
                            .to_ascii_lowercase()
                            .contains(&search.to_ascii_lowercase())
                    {
                        spans.extend(highlight_search_in_line(line, search));
                    } else if is_json {
                        spans.extend(highlight_json_line(line));
                    } else {
                        spans.push(Span::styled(line, Style::default().fg(FG)));
                    }
                    Line::from(spans)
                })
                .collect();

            let total_lines = lines.len() as u16;
            let paragraph = Paragraph::new(Text::from(lines))
                .block(block)
                .scroll((app.response_scroll, 0));
            frame.render_widget(paragraph, area);

            if total_lines > inner.height {
                let mut scrollbar_state = ScrollbarState::new(total_lines as usize)
                    .position(app.response_scroll as usize);
                frame.render_stateful_widget(
                    Scrollbar::new(ScrollbarOrientation::VerticalRight)
                        .thumb_style(Style::default().fg(MUTED)),
                    inner,
                    &mut scrollbar_state,
                );
            }
        }
        None => {
            let msg = if app.loading {
                "Sending request..."
            } else {
                "Press Enter to send a request"
            };
            let paragraph = Paragraph::new(Text::styled(msg, Style::default().fg(MUTED).italic()))
                .block(block)
                .alignment(Alignment::Center);
            frame.render_widget(paragraph, area);
        }
    }
}

fn highlight_search_in_line<'a>(line: &'a str, needle: &str) -> Vec<Span<'a>> {
    let lower_line = line.to_ascii_lowercase();
    let lower_needle = needle.to_ascii_lowercase();
    let mut spans = Vec::new();
    let mut pos = 0;

    while let Some(idx) = lower_line[pos..].find(&lower_needle) {
        let start = pos + idx;
        let end = start + needle.len();
        if start > pos {
            spans.push(Span::styled(&line[pos..start], Style::default().fg(FG)));
        }
        spans.push(Span::styled(
            &line[start..end],
            Style::default().fg(BG).bg(YELLOW).bold(),
        ));
        pos = end;
    }

    if pos < line.len() {
        spans.push(Span::styled(&line[pos..], Style::default().fg(FG)));
    }

    spans
}

fn highlight_json_line(line: &str) -> Vec<Span<'_>> {
    let trimmed = line.trim_start();
    let indent = line.len() - trimmed.len();
    let mut spans = Vec::new();

    if indent > 0 {
        spans.push(Span::styled(&line[..indent], Style::default().fg(FG)));
    }

    if trimmed.is_empty() {
        return spans;
    }

    let first = trimmed.as_bytes().first().copied().unwrap_or(0);

    match first {
        b'{' | b'}' | b'[' | b']' => {
            spans.push(Span::styled(trimmed, Style::default().fg(MUTED)));
        }
        b'"' => {
            if let Some(colon_pos) = trimmed.find("\": ").or_else(|| trimmed.find("\":")) {
                let key_end = colon_pos + 1;
                spans.push(Span::styled(&trimmed[..key_end], Style::default().fg(TEAL)));
                let rest = &trimmed[key_end..];
                spans.extend(highlight_json_value(rest));
            } else {
                spans.push(Span::styled(trimmed, Style::default().fg(ORANGE)));
            }
        }
        b'0'..=b'9' | b'-' => {
            let num_end = trimmed
                .find(|c: char| {
                    !c.is_ascii_digit() && c != '.' && c != '-' && c != 'e' && c != 'E' && c != '+'
                })
                .unwrap_or(trimmed.len());
            spans.push(Span::styled(
                &trimmed[..num_end],
                Style::default().fg(PURPLE),
            ));
            if num_end < trimmed.len() {
                spans.push(Span::styled(
                    &trimmed[num_end..],
                    Style::default().fg(MUTED),
                ));
            }
        }
        b't' | b'f' => {
            let word_end = trimmed
                .find(|c: char| !c.is_ascii_alphabetic())
                .unwrap_or(trimmed.len());
            spans.push(Span::styled(
                &trimmed[..word_end],
                Style::default().fg(YELLOW),
            ));
            if word_end < trimmed.len() {
                spans.push(Span::styled(
                    &trimmed[word_end..],
                    Style::default().fg(MUTED),
                ));
            }
        }
        b'n' => {
            let word_end = trimmed
                .find(|c: char| !c.is_ascii_alphabetic())
                .unwrap_or(trimmed.len());
            spans.push(Span::styled(
                &trimmed[..word_end],
                Style::default().fg(MUTED).italic(),
            ));
            if word_end < trimmed.len() {
                spans.push(Span::styled(
                    &trimmed[word_end..],
                    Style::default().fg(MUTED),
                ));
            }
        }
        _ => {
            spans.push(Span::styled(trimmed, Style::default().fg(FG)));
        }
    }

    spans
}

fn highlight_json_value(rest: &str) -> Vec<Span<'_>> {
    let trimmed = rest.trim_start();
    let ws_len = rest.len() - trimmed.len();
    let mut spans = Vec::new();

    if ws_len > 0 {
        spans.push(Span::styled(&rest[..ws_len], Style::default().fg(FG)));
    }

    if trimmed.is_empty() {
        return spans;
    }

    let first = trimmed.as_bytes().first().copied().unwrap_or(0);

    match first {
        b'"' => spans.push(Span::styled(trimmed, Style::default().fg(ORANGE))),
        b'0'..=b'9' | b'-' => spans.push(Span::styled(trimmed, Style::default().fg(PURPLE))),
        b't' | b'f' => spans.push(Span::styled(trimmed, Style::default().fg(YELLOW))),
        b'n' => spans.push(Span::styled(trimmed, Style::default().fg(MUTED).italic())),
        b'{' | b'[' => spans.push(Span::styled(trimmed, Style::default().fg(MUTED))),
        _ => spans.push(Span::styled(trimmed, Style::default().fg(FG))),
    }

    spans
}

fn format_size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}
