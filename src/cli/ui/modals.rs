use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use super::*;
use crate::app::{App, Method};

pub(super) fn draw_method_popup(frame: &mut Frame, app: &App) {
    let methods = Method::all();
    let popup_h = methods.len() as u16 + 2;
    let popup_w: u16 = 14;

    let area = frame.area();
    let x = (area.width.saturating_sub(popup_w)) / 2;
    let y = (area.height.saturating_sub(popup_h)) / 2;
    let popup_area = Rect::new(x, y, popup_w, popup_h);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Method ")
        .title_style(Style::default().fg(GREEN).bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(GREEN))
        .bg(BG);

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    for (i, method) in methods.iter().enumerate() {
        let row_y = inner.y + i as u16;
        if row_y >= inner.y + inner.height {
            break;
        }
        let row = Rect::new(inner.x, row_y, inner.width, 1);
        let selected = i == app.method_popup.selected;

        if selected {
            frame.render_widget(Paragraph::new("").bg(SURFACE), row);
        }

        let line = Line::from(vec![
            Span::styled(
                if selected { " > " } else { "   " },
                Style::default().fg(GREEN),
            ),
            Span::styled(
                method.as_str(),
                Style::default().fg(method_color(method)).bold(),
            ),
        ]);

        frame.render_widget(Paragraph::new(line), row);
    }
}

pub(super) fn draw_history_overlay(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let popup_w = area.width.saturating_sub(10).min(80);
    let popup_h = area.height.saturating_sub(6).min(25);
    let x = (area.width.saturating_sub(popup_w)) / 2;
    let y = (area.height.saturating_sub(popup_h)) / 2;
    let popup_area = Rect::new(x, y, popup_w, popup_h);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" History ")
        .title_style(Style::default().fg(GREEN).bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(GREEN))
        .bg(BG);

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    if app.history.entries.is_empty() {
        let msg = Paragraph::new(Text::styled(
            "No history yet",
            Style::default().fg(MUTED).italic(),
        ))
        .alignment(Alignment::Center);
        frame.render_widget(msg, inner);
        return;
    }

    for (i, entry) in app.history.entries.iter().rev().enumerate() {
        let row_y = inner.y + i as u16;
        if row_y >= inner.y + inner.height {
            break;
        }
        let selected = i == app.history.selected;
        let row = Rect::new(inner.x, row_y, inner.width, 1);

        if selected {
            frame.render_widget(Paragraph::new("").bg(SURFACE), row);
        }

        let method = Method::from_str(&entry.method);
        let status_str = entry
            .status
            .map_or_else(|| "ERR".to_owned(), |s| s.to_string());
        let s_color = entry.status.map_or(RED, status_color);
        let duration_str = entry
            .duration_ms
            .map_or_else(|| "-".to_owned(), |d| format!("{d}ms"));

        let max_url = (inner.width as usize).saturating_sub(25);
        let url_display = if entry.url.len() > max_url {
            format!("{}...", &entry.url[..max_url.saturating_sub(3)])
        } else {
            entry.url.clone()
        };

        let line = Line::from(vec![
            Span::styled(
                if selected { " > " } else { "   " },
                Style::default().fg(GREEN),
            ),
            Span::styled(
                format!("{status_str:>3}"),
                Style::default().fg(s_color).bold(),
            ),
            Span::raw("  "),
            Span::styled(
                format!("{:>6}", method.as_str()),
                Style::default().fg(method_color(&method)).bold(),
            ),
            Span::raw("  "),
            Span::styled(
                url_display,
                Style::default().fg(if selected { FG } else { MUTED }),
            ),
            Span::raw("  "),
            Span::styled(duration_str, Style::default().fg(MUTED)),
        ]);

        frame.render_widget(Paragraph::new(line), row);
    }
}

pub(super) fn draw_curl_import_popup(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let popup_w = area.width.saturating_sub(10).min(70);
    let popup_h = area.height.saturating_sub(6).min(14);
    let x = (area.width.saturating_sub(popup_w)) / 2;
    let y = (area.height.saturating_sub(popup_h)) / 2;
    let popup_area = Rect::new(x, y, popup_w, popup_h);

    frame.render_widget(Clear, popup_area);

    let title = if app.curl_io.import_error {
        " Import cURL (invalid) "
    } else {
        " Import cURL "
    };
    let title_style = if app.curl_io.import_error {
        Style::default().fg(RED).bold()
    } else {
        Style::default().fg(GREEN).bold()
    };

    let block = Block::default()
        .title(title)
        .title_style(title_style)
        .borders(Borders::ALL)
        .border_style(if app.curl_io.import_error {
            Style::default().fg(RED)
        } else {
            Style::default().fg(GREEN)
        })
        .bg(BG);

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let display = format!("{}\u{2588}", &app.curl_io.import_buffer);
    let paragraph = Paragraph::new(Text::styled(&display, Style::default().fg(FG)))
        .wrap(ratatui::widgets::Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}

pub(super) fn draw_curl_export_popup(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let popup_w = area.width.saturating_sub(10).min(80);
    let popup_h = area.height.saturating_sub(6).min(16);
    let x = (area.width.saturating_sub(popup_w)) / 2;
    let y = (area.height.saturating_sub(popup_h)) / 2;
    let popup_area = Rect::new(x, y, popup_w, popup_h);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Export cURL ")
        .title_style(Style::default().fg(GREEN).bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(GREEN))
        .bg(BG);

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let paragraph = Paragraph::new(Text::styled(
        &app.curl_io.export_content,
        Style::default().fg(FG),
    ))
    .wrap(ratatui::widgets::Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}

pub(super) fn draw_postman_import_popup(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let popup_w = area.width.saturating_sub(10).min(70);
    let popup_h: u16 = 5;
    let x = (area.width.saturating_sub(popup_w)) / 2;
    let y = (area.height.saturating_sub(popup_h)) / 2;
    let popup_area = Rect::new(x, y, popup_w, popup_h);

    frame.render_widget(Clear, popup_area);

    let title = if app.postman_io.error {
        " Import Postman (invalid) "
    } else {
        " Import Postman "
    };
    let title_style = if app.postman_io.error {
        Style::default().fg(RED).bold()
    } else {
        Style::default().fg(GREEN).bold()
    };

    let block = Block::default()
        .title(title)
        .title_style(title_style)
        .borders(Borders::ALL)
        .border_style(if app.postman_io.error {
            Style::default().fg(RED)
        } else {
            Style::default().fg(GREEN)
        })
        .bg(BG);

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let display = format!("{}\u{2588}", &app.postman_io.buffer);
    let lines = vec![
        Line::from(Span::styled(
            "Path to collection JSON:",
            Style::default().fg(MUTED),
        )),
        Line::default(),
        Line::from(Span::styled(display, Style::default().fg(FG))),
    ];
    let paragraph = Paragraph::new(Text::from(lines));
    frame.render_widget(paragraph, inner);
}

pub(super) fn draw_env_popup(frame: &mut Frame, app: &App) {
    let count = app.env_popup_count();
    let popup_h = (count as u16 + 2).min(20);
    let popup_w: u16 = 40;

    let area = frame.area();
    let x = (area.width.saturating_sub(popup_w)) / 2;
    let y = (area.height.saturating_sub(popup_h)) / 2;
    let popup_area = Rect::new(x, y, popup_w, popup_h);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Environments ")
        .title_style(Style::default().fg(TEAL).bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(TEAL))
        .bg(BG);

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    // First row: "No environment"
    let row_y = inner.y;
    if row_y < inner.y + inner.height {
        let row = Rect::new(inner.x, row_y, inner.width, 1);
        let selected = app.env.popup_selected == 0;
        let is_active = app.env.active_id.is_none();

        if selected {
            frame.render_widget(Paragraph::new("").bg(SURFACE), row);
        }

        let dot = if is_active { "\u{25cf}" } else { "\u{25cb}" };
        let line = Line::from(vec![
            Span::styled(
                if selected { " > " } else { "   " },
                Style::default().fg(TEAL),
            ),
            Span::styled(format!("{dot} "), Style::default().fg(MUTED)),
            Span::styled(
                "No environment",
                if selected {
                    Style::default().fg(FG).italic()
                } else {
                    Style::default().fg(MUTED).italic()
                },
            ),
        ]);
        frame.render_widget(Paragraph::new(line), row);
    }

    // Environment rows
    for (i, env) in app.env.environments.iter().enumerate() {
        let row_y = inner.y + (i as u16 + 1);
        if row_y >= inner.y + inner.height {
            break;
        }
        let row = Rect::new(inner.x, row_y, inner.width, 1);
        let idx = i + 1;
        let selected = app.env.popup_selected == idx;
        let is_active = app.env.active_id.as_deref() == Some(&env.id);

        if selected {
            frame.render_widget(Paragraph::new("").bg(SURFACE), row);
        }

        let dot = if is_active { "\u{25cf}" } else { "\u{25cb}" };
        let name = if app.env.renaming && selected {
            format!("{}\u{2588}", &app.env.name_buffer)
        } else {
            env.name.clone()
        };
        let name_style = if app.env.renaming && selected {
            Style::default().fg(TEAL)
        } else if selected {
            Style::default().fg(FG)
        } else if is_active {
            Style::default().fg(TEAL)
        } else {
            Style::default().fg(MUTED)
        };

        let line = Line::from(vec![
            Span::styled(
                if selected { " > " } else { "   " },
                Style::default().fg(TEAL),
            ),
            Span::styled(
                format!("{dot} "),
                if is_active {
                    Style::default().fg(TEAL)
                } else {
                    Style::default().fg(MUTED)
                },
            ),
            Span::styled(name, name_style),
        ]);
        frame.render_widget(Paragraph::new(line), row);
    }
}

pub(super) fn draw_assertions_popup(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let popup_w = area.width.saturating_sub(6).min(100);
    let popup_h = area.height.saturating_sub(4).min(24);
    let x = (area.width.saturating_sub(popup_w)) / 2;
    let y = (area.height.saturating_sub(popup_h)) / 2;
    let popup_area = Rect::new(x, y, popup_w, popup_h);

    frame.render_widget(Clear, popup_area);

    let passed = app.assertions.results.iter().filter(|r| r.passed).count();
    let total = app.assertions.exprs.len();
    let title = if app.assertions.results.is_empty() {
        format!(" Assertions ({total}) ")
    } else {
        format!(" Assertions ({passed}/{total} passed) ")
    };

    let block = Block::default()
        .title(title)
        .title_style(Style::default().fg(TEAL).bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(TEAL))
        .bg(BG);

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    if app.assertions.editing {
        draw_assertion_editor(frame, app, inner);
    } else {
        draw_assertions_list(frame, app, inner);
    }
}

fn draw_assertion_editor(frame: &mut Frame, app: &App, inner: Rect) {
    let lines = vec![
        Line::default(),
        Line::from(Span::styled(
            "  Examples:",
            Style::default().fg(MUTED).italic(),
        )),
        Line::from(Span::styled(
            "    status == 200",
            Style::default().fg(MUTED),
        )),
        Line::from(Span::styled(
            "    body contains \"hello\"",
            Style::default().fg(MUTED),
        )),
        Line::from(Span::styled(
            "    header Content-Type contains json",
            Style::default().fg(MUTED),
        )),
        Line::from(Span::styled(
            "    json $.token != \"\"",
            Style::default().fg(MUTED),
        )),
        Line::default(),
        Line::from(vec![
            Span::styled("  > ", Style::default().fg(GREEN)),
            Span::styled(
                format!("{}\u{2588}", &app.assertions.edit_buffer),
                Style::default().fg(FG),
            ),
        ]),
        Line::default(),
        Line::from(Span::styled(
            "  Enter:save  Esc:cancel",
            Style::default().fg(MUTED),
        )),
    ];
    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}

fn draw_assertions_list(frame: &mut Frame, app: &App, inner: Rect) {
    let mut y_pos = inner.y;
    if app.assertions.exprs.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "  No assertions. Press a to add one.",
                Style::default().fg(MUTED).italic(),
            ))),
            Rect::new(inner.x, y_pos, inner.width, 1),
        );
    } else {
        for (i, expr) in app.assertions.exprs.iter().enumerate() {
            if y_pos >= inner.y + inner.height - 2 {
                break;
            }
            let row = Rect::new(inner.x, y_pos, inner.width, 1);
            let selected = i == app.assertions.selected;
            if selected {
                frame.render_widget(Paragraph::new("").bg(SURFACE), row);
            }
            let result = app.assertions.results.get(i);
            let (icon, icon_color, detail) = match result {
                Some(r) if r.passed => ("\u{2713}", GREEN, r.message.clone()),
                Some(r) => ("\u{2717}", RED, r.message.clone()),
                None => ("\u{25cb}", MUTED, "(not yet evaluated)".to_owned()),
            };
            let line = Line::from(vec![
                Span::styled(
                    if selected { " > " } else { "   " },
                    Style::default().fg(GREEN),
                ),
                Span::styled(format!("{icon} "), Style::default().fg(icon_color).bold()),
                Span::styled(expr, Style::default().fg(FG)),
                Span::styled("   ", Style::default()),
                Span::styled(detail, Style::default().fg(MUTED).italic()),
            ]);
            frame.render_widget(Paragraph::new(line), row);
            y_pos += 1;
        }
    }

    let footer = Rect::new(inner.x, inner.y + inner.height - 1, inner.width, 1);
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "  j/k:nav  a:add  e:edit  d:delete  Esc:close",
            Style::default().fg(MUTED),
        ))),
        footer,
    );
}

pub(super) fn draw_extractors_popup(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let popup_w = area.width.saturating_sub(6).min(90);
    let popup_h = area.height.saturating_sub(4).min(22);
    let x = (area.width.saturating_sub(popup_w)) / 2;
    let y = (area.height.saturating_sub(popup_h)) / 2;
    let popup_area = Rect::new(x, y, popup_w, popup_h);

    frame.render_widget(Clear, popup_area);

    let title = format!(
        " Response extractors ({}) ",
        app.extractors.editor.entries.len()
    );
    let block = Block::default()
        .title(title)
        .title_style(Style::default().fg(TEAL).bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(TEAL))
        .bg(BG);

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    if app.extractors.editor.editing {
        draw_extractor_editor(frame, app, inner);
    } else {
        draw_extractors_list(frame, app, inner);
    }
}

fn draw_extractor_editor(frame: &mut Frame, app: &App, inner: Rect) {
    let lines = vec![
        Line::default(),
        Line::from(Span::styled(
            "  Variable name (used as {{name}}):",
            Style::default().fg(MUTED),
        )),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                if app.extractors.editor.edit_field == 0 {
                    format!("{}\u{2588}", app.extractors.editor.edit_key_buf)
                } else {
                    app.extractors.editor.edit_key_buf.clone()
                },
                Style::default().fg(if app.extractors.editor.edit_field == 0 {
                    GREEN
                } else {
                    FG
                }),
            ),
        ]),
        Line::default(),
        Line::from(Span::styled(
            "  JSONPath (e.g. $.token, data.users[0].id):",
            Style::default().fg(MUTED),
        )),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                if app.extractors.editor.edit_field == 1 {
                    format!("{}\u{2588}", app.extractors.editor.edit_value_buf)
                } else {
                    app.extractors.editor.edit_value_buf.clone()
                },
                Style::default().fg(if app.extractors.editor.edit_field == 1 {
                    GREEN
                } else {
                    FG
                }),
            ),
        ]),
        Line::default(),
        Line::from(Span::styled(
            "  Tab:switch  Enter:save  Esc:cancel",
            Style::default().fg(MUTED),
        )),
    ];
    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}

fn draw_extractors_list(frame: &mut Frame, app: &App, inner: Rect) {
    let mut y_pos = inner.y;
    if app.extractors.editor.entries.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "  No extractors. Press a to add one.",
                Style::default().fg(MUTED).italic(),
            ))),
            Rect::new(inner.x, y_pos, inner.width, 1),
        );
    } else {
        for (i, (name, path)) in app.extractors.editor.entries.iter().enumerate() {
            if y_pos >= inner.y + inner.height - 2 {
                break;
            }
            let row = Rect::new(inner.x, y_pos, inner.width, 1);
            let selected = i == app.extractors.editor.selected;
            if selected {
                frame.render_widget(Paragraph::new("").bg(SURFACE), row);
            }
            let preview = app
                .extractors
                .extracted
                .get(name)
                .cloned()
                .unwrap_or_default();
            let line = Line::from(vec![
                Span::styled(
                    if selected { " > " } else { "   " },
                    Style::default().fg(GREEN),
                ),
                Span::styled(name, Style::default().fg(ORANGE).bold()),
                Span::styled("  <- ", Style::default().fg(MUTED)),
                Span::styled(path, Style::default().fg(TEAL)),
                Span::styled("   = ", Style::default().fg(MUTED)),
                Span::styled(preview, Style::default().fg(FG)),
            ]);
            frame.render_widget(Paragraph::new(line), row);
            y_pos += 1;
        }
    }

    let footer = Rect::new(inner.x, inner.y + inner.height - 1, inner.width, 1);
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "  j/k:nav  a:add  e:edit  d:delete  Esc:close",
            Style::default().fg(MUTED),
        ))),
        footer,
    );
}

pub(super) fn draw_cookies_popup(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let popup_w = area.width.saturating_sub(6).min(100);
    let popup_h = area.height.saturating_sub(4).min(25);
    let x = (area.width.saturating_sub(popup_w)) / 2;
    let y = (area.height.saturating_sub(popup_h)) / 2;
    let popup_area = Rect::new(x, y, popup_w, popup_h);

    frame.render_widget(Clear, popup_area);

    let title = format!(" Cookies ({}) ", app.cookies.store.cookies.len());
    let block = Block::default()
        .title(title)
        .title_style(Style::default().fg(TEAL).bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(TEAL))
        .bg(BG);

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    if app.cookies.store.cookies.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "  No cookies stored. They will be captured automatically from Set-Cookie headers.",
                Style::default().fg(MUTED).italic(),
            ))),
            inner,
        );
        return;
    }

    let visible_rows = inner.height.saturating_sub(2) as usize;
    let total = app.cookies.store.cookies.len();
    let start = app
        .cookies
        .popup_selected
        .saturating_sub(visible_rows.saturating_sub(1));

    let mut y_pos = inner.y;
    for (i, cookie) in app
        .cookies
        .store
        .cookies
        .iter()
        .enumerate()
        .skip(start)
        .take(visible_rows)
    {
        let row = Rect::new(inner.x, y_pos, inner.width, 1);
        let selected = i == app.cookies.popup_selected;
        if selected {
            frame.render_widget(Paragraph::new("").bg(SURFACE), row);
        }
        let marker = if selected { " > " } else { "   " };
        let flags = {
            let mut f = String::new();
            if cookie.secure {
                f.push_str(" secure");
            }
            if cookie.http_only {
                f.push_str(" httponly");
            }
            f
        };
        let line = Line::from(vec![
            Span::styled(marker, Style::default().fg(GREEN)),
            Span::styled(&cookie.domain, Style::default().fg(TEAL).bold()),
            Span::styled(&cookie.path, Style::default().fg(MUTED)),
            Span::raw("  "),
            Span::styled(&cookie.name, Style::default().fg(ORANGE).bold()),
            Span::styled("=", Style::default().fg(MUTED)),
            Span::styled(&cookie.value, Style::default().fg(FG)),
            Span::styled(flags, Style::default().fg(MUTED).italic()),
        ]);
        frame.render_widget(Paragraph::new(line), row);
        y_pos += 1;
        if y_pos >= inner.y + inner.height - 1 {
            break;
        }
    }

    let footer = Rect::new(inner.x, inner.y + inner.height - 1, inner.width, 1);
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            format!(
                "  {}/{}  j/k:nav  d:delete  D:clear all  Esc:close",
                app.cookies.popup_selected + 1,
                total,
            ),
            Style::default().fg(MUTED),
        ))),
        footer,
    );
}

pub(super) fn draw_tls_popup(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let popup_w = area.width.saturating_sub(8).min(80);
    let popup_h: u16 = 14;
    let x = (area.width.saturating_sub(popup_w)) / 2;
    let y = (area.height.saturating_sub(popup_h)) / 2;
    let popup_area = Rect::new(x, y, popup_w, popup_h);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" TLS / Certificates ")
        .title_style(Style::default().fg(TEAL).bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(TEAL))
        .bg(BG);

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let min_label = if app.tls.min_version.is_empty() {
        "auto".to_owned()
    } else {
        format!("TLS {}", app.tls.min_version)
    };

    let fields: [(&str, String); 5] = [
        (
            "Verify TLS",
            if app.tls.verify {
                "on"
            } else {
                "off (insecure)"
            }
            .to_owned(),
        ),
        ("CA cert", display_or_none(&app.tls.ca_cert)),
        ("Client cert", display_or_none(&app.tls.client_cert)),
        ("Client key", display_or_none(&app.tls.client_key)),
        ("Min TLS version", min_label),
    ];

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::default());
    for (i, (label, value)) in fields.iter().enumerate() {
        let selected = i == app.tls.popup_selected;
        let marker = if selected { " > " } else { "   " };
        let value_style = if selected {
            Style::default().fg(FG)
        } else {
            Style::default().fg(MUTED)
        };
        lines.push(Line::from(vec![
            Span::styled(marker, Style::default().fg(GREEN)),
            Span::styled(format!("{label:<18}"), Style::default().fg(TEAL).bold()),
            Span::styled(value.clone(), value_style),
        ]));
    }

    lines.push(Line::default());
    if app.tls.editing {
        lines.push(Line::from(vec![
            Span::styled("   path: ", Style::default().fg(MUTED)),
            Span::styled(
                format!("{}\u{2588}", &app.tls.edit_buffer),
                Style::default().fg(GREEN),
            ),
        ]));
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            "   Enter:save  Esc:cancel",
            Style::default().fg(MUTED),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "   j/k:nav  Enter/Space:toggle/edit  d:clear  Esc:close",
            Style::default().fg(MUTED),
        )));
    }

    let paragraph = Paragraph::new(Text::from(lines));
    frame.render_widget(paragraph, inner);
}

fn display_or_none(s: &str) -> String {
    if s.is_empty() {
        "(none)".to_owned()
    } else {
        s.to_owned()
    }
}

pub(super) fn draw_timeout_popup(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let popup_w = area.width.saturating_sub(10).min(50);
    let popup_h: u16 = 6;
    let x = (area.width.saturating_sub(popup_w)) / 2;
    let y = (area.height.saturating_sub(popup_h)) / 2;
    let popup_area = Rect::new(x, y, popup_w, popup_h);

    frame.render_widget(Clear, popup_area);

    let title = if app.timeout.error {
        " Timeout (1-3600) "
    } else {
        " Request timeout "
    };
    let color = if app.timeout.error { RED } else { TEAL };

    let block = Block::default()
        .title(title)
        .title_style(Style::default().fg(color).bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(color))
        .bg(BG);

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let display = format!("{}\u{2588}", &app.timeout.buffer);
    let lines = vec![
        Line::from(Span::styled("Seconds:", Style::default().fg(MUTED))),
        Line::default(),
        Line::from(Span::styled(display, Style::default().fg(FG))),
        Line::default(),
        Line::from(Span::styled(
            "Enter:save  Esc:cancel",
            Style::default().fg(MUTED),
        )),
    ];
    let paragraph = Paragraph::new(Text::from(lines));
    frame.render_widget(paragraph, inner);
}

pub(super) fn draw_env_import_popup(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let popup_w = area.width.saturating_sub(10).min(70);
    let popup_h: u16 = 5;
    let x = (area.width.saturating_sub(popup_w)) / 2;
    let y = (area.height.saturating_sub(popup_h)) / 2;
    let popup_area = Rect::new(x, y, popup_w, popup_h);

    frame.render_widget(Clear, popup_area);

    let title = if app.env.import.error {
        " Import .env (invalid) "
    } else {
        " Import .env "
    };
    let title_style = if app.env.import.error {
        Style::default().fg(RED).bold()
    } else {
        Style::default().fg(TEAL).bold()
    };

    let block = Block::default()
        .title(title)
        .title_style(title_style)
        .borders(Borders::ALL)
        .border_style(if app.env.import.error {
            Style::default().fg(RED)
        } else {
            Style::default().fg(TEAL)
        })
        .bg(BG);

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let display = format!("{}\u{2588}", &app.env.import.buffer);
    let lines = vec![
        Line::from(Span::styled(
            "Path to .env file:",
            Style::default().fg(MUTED),
        )),
        Line::default(),
        Line::from(Span::styled(display, Style::default().fg(FG))),
    ];
    let paragraph = Paragraph::new(Text::from(lines));
    frame.render_widget(paragraph, inner);
}

pub(super) fn draw_env_editor(frame: &mut Frame, app: &App) {
    let env = app
        .env
        .environments
        .iter()
        .find(|e| e.id == app.env.editor.id);
    let env_name = env.map_or("?", |e| &e.name);
    let vars = env.map_or(&[][..], |e| &e.variables);

    let area = frame.area();
    let popup_w = area.width.saturating_sub(10).min(70);
    let popup_h = area.height.saturating_sub(6).min(25);
    let x = (area.width.saturating_sub(popup_w)) / 2;
    let y = (area.height.saturating_sub(popup_h)) / 2;
    let popup_area = Rect::new(x, y, popup_w, popup_h);

    frame.render_widget(Clear, popup_area);

    let title = format!(" {env_name} - Variables ");
    let block = Block::default()
        .title(title)
        .title_style(Style::default().fg(TEAL).bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(TEAL))
        .bg(BG);

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    if app.env.editor.editing_var {
        draw_env_var_edit(frame, app, inner);
        return;
    }

    for (i, var) in vars.iter().enumerate() {
        let row_y = inner.y + i as u16;
        if row_y >= inner.y + inner.height {
            break;
        }
        let row = Rect::new(inner.x, row_y, inner.width, 1);
        let selected = i == app.env.editor.selected;

        if selected {
            frame.render_widget(Paragraph::new("").bg(SURFACE), row);
        }

        let display_value: String = if var.secret {
            "\u{2022}".repeat(var.value.len().clamp(6, 20))
        } else {
            var.value.clone()
        };
        let secret_indicator = if var.secret { " \u{1f512}" } else { "" };

        let line = Line::from(vec![
            Span::styled(
                if selected { " > " } else { "   " },
                Style::default().fg(TEAL),
            ),
            Span::styled(&var.key, Style::default().fg(TEAL).bold()),
            Span::styled(secret_indicator, Style::default().fg(YELLOW)),
            Span::styled(" = ", Style::default().fg(MUTED)),
            Span::styled(
                display_value,
                if selected {
                    Style::default().fg(FG)
                } else {
                    Style::default().fg(MUTED)
                },
            ),
        ]);
        frame.render_widget(Paragraph::new(line), row);
    }

    let add_y = inner.y + vars.len() as u16;
    if add_y < inner.y + inner.height {
        let row = Rect::new(inner.x, add_y, inner.width, 1);
        let selected = app.env.editor.selected >= vars.len();

        if selected {
            frame.render_widget(Paragraph::new("").bg(SURFACE), row);
        }

        let line = Line::from(Span::styled(
            "   + add variable",
            if selected {
                Style::default().fg(TEAL)
            } else {
                Style::default().fg(MUTED)
            },
        ));
        frame.render_widget(Paragraph::new(line), row);
    }
}

fn draw_env_var_edit(frame: &mut Frame, app: &App, parent: Rect) {
    let edit_h: u16 = 7;
    let edit_w: u16 = 50.min(parent.width);
    let ex = parent.x + (parent.width.saturating_sub(edit_w)) / 2;
    let ey = parent.y + (parent.height.saturating_sub(edit_h)) / 2;
    let edit_area = Rect::new(ex, ey, edit_w, edit_h);

    frame.render_widget(Clear, edit_area);

    let edit_block = Block::default()
        .title(" Edit Variable ")
        .title_style(Style::default().fg(GREEN).bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(GREEN))
        .bg(BG);

    let edit_inner = edit_block.inner(edit_area);
    frame.render_widget(edit_block, edit_area);

    let key_style = if app.env.editor.var_field == 0 {
        Style::default().fg(GREEN)
    } else {
        Style::default().fg(FG)
    };
    let val_style = if app.env.editor.var_field == 1 {
        Style::default().fg(GREEN)
    } else {
        Style::default().fg(FG)
    };

    let key_display = if app.env.editor.var_field == 0 {
        format!("{}\u{2588}", &app.env.editor.var_key_buffer)
    } else {
        app.env.editor.var_key_buffer.clone()
    };
    let val_display = if app.env.editor.var_field == 1 {
        format!("{}\u{2588}", &app.env.editor.var_value_buffer)
    } else {
        app.env.editor.var_value_buffer.clone()
    };

    let lines = vec![
        Line::from(vec![
            Span::styled("  Key:   ", Style::default().fg(MUTED)),
            Span::styled(key_display, key_style),
        ]),
        Line::default(),
        Line::from(vec![
            Span::styled("  Value: ", Style::default().fg(MUTED)),
            Span::styled(val_display, val_style),
        ]),
        Line::default(),
        Line::from(Span::styled(
            "  Tab:switch  Enter:save  Esc:cancel",
            Style::default().fg(MUTED),
        )),
    ];
    let paragraph = Paragraph::new(Text::from(lines));
    frame.render_widget(paragraph, edit_inner);
}

pub(super) fn draw_proto_popup(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let popup_w = area.width.saturating_sub(10).min(80);
    let popup_h: u16 = 7;
    let x = (area.width.saturating_sub(popup_w)) / 2;
    let y = (area.height.saturating_sub(popup_h)) / 2;
    let popup_area = Rect::new(x, y, popup_w, popup_h);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Load .proto ")
        .title_style(Style::default().fg(GREEN).bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(GREEN))
        .bg(BG);
    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let mut lines = vec![
        Line::from(Span::styled(
            "  Path to .proto file:",
            Style::default().fg(MUTED),
        )),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!("{}\u{2588}", app.grpc.proto_buffer),
                Style::default().fg(FG),
            ),
        ]),
        Line::default(),
    ];
    if let Some(err) = &app.grpc.proto_error {
        lines.push(Line::from(Span::styled(
            format!("  ! {err}"),
            Style::default().fg(RED),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "  Enter:load  Esc:cancel",
            Style::default().fg(MUTED),
        )));
    }

    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}

pub(super) fn draw_plugins_popup(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let popup_w = area.width.saturating_sub(10).min(80);
    let popup_h: u16 = 7;
    let x = (area.width.saturating_sub(popup_w)) / 2;
    let y = (area.height.saturating_sub(popup_h)) / 2;
    let popup_area = Rect::new(x, y, popup_w, popup_h);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Plugins ")
        .title_style(Style::default().fg(GREEN).bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(GREEN))
        .bg(BG);
    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let mut lines = vec![
        Line::from(Span::styled(
            "  comma-separated plugin names from ~/.config/frogbite/plugins/",
            Style::default().fg(MUTED),
        )),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!("{}\u{2588}", app.plugins.buffer),
                Style::default().fg(FG),
            ),
        ]),
        Line::default(),
    ];
    if let Some(err) = &app.plugins.error {
        lines.push(Line::from(Span::styled(
            format!("  ! {err}"),
            Style::default().fg(RED),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "  Enter:save  Esc:cancel",
            Style::default().fg(MUTED),
        )));
    }
    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}

pub(super) fn draw_proxy_popup(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let popup_w = area.width.saturating_sub(10).min(80);
    let popup_h: u16 = 7;
    let x = (area.width.saturating_sub(popup_w)) / 2;
    let y = (area.height.saturating_sub(popup_h)) / 2;
    let popup_area = Rect::new(x, y, popup_w, popup_h);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Proxy URL ")
        .title_style(Style::default().fg(GREEN).bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(GREEN))
        .bg(BG);
    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let mut lines = vec![
        Line::from(Span::styled(
            "  http://, https://, socks5:// or socks5h:// (empty = no proxy)",
            Style::default().fg(MUTED),
        )),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!("{}\u{2588}", app.proxy.buffer),
                Style::default().fg(FG),
            ),
        ]),
        Line::default(),
    ];
    if let Some(err) = &app.proxy.error {
        lines.push(Line::from(Span::styled(
            format!("  ! {err}"),
            Style::default().fg(RED),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "  Enter:save  Esc:cancel",
            Style::default().fg(MUTED),
        )));
    }
    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}

pub(super) fn draw_diff_popup(frame: &mut Frame, app: &App) {
    use similar::{ChangeTag, TextDiff};

    let area = frame.area();
    let popup_w = area.width.saturating_sub(4);
    let popup_h = area.height.saturating_sub(4);
    let x = (area.width.saturating_sub(popup_w)) / 2;
    let y = (area.height.saturating_sub(popup_h)) / 2;
    let popup_area = Rect::new(x, y, popup_w, popup_h);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Diff (snapshot \u{2192} current) ")
        .title_style(Style::default().fg(GREEN).bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(GREEN))
        .bg(BG);
    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let snapshot = app.diff.snapshot.as_deref().unwrap_or("");
    let current = match &app.response.last {
        Some(Ok(r)) => r.body.as_str(),
        Some(Err(e)) => e.as_str(),
        None => "",
    };

    let diff = TextDiff::from_lines(snapshot, current);
    let mut lines: Vec<Line> = Vec::new();
    let mut added = 0usize;
    let mut removed = 0usize;
    for change in diff.iter_all_changes() {
        let (sign, color) = match change.tag() {
            ChangeTag::Equal => (" ", MUTED),
            ChangeTag::Insert => {
                added += 1;
                ("+", GREEN)
            }
            ChangeTag::Delete => {
                removed += 1;
                ("-", RED)
            }
        };
        let text = change.value().trim_end_matches('\n').to_owned();
        lines.push(Line::from(vec![
            Span::styled(format!("{sign} "), Style::default().fg(color).bold()),
            Span::styled(text, Style::default().fg(color)),
        ]));
    }

    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "(both bodies are empty)",
            Style::default().fg(MUTED).italic(),
        )));
    }

    let header = Line::from(vec![
        Span::styled(
            format!(" +{added} "),
            Style::default().fg(BG).bg(GREEN).bold(),
        ),
        Span::raw(" "),
        Span::styled(
            format!(" -{removed} "),
            Style::default().fg(BG).bg(RED).bold(),
        ),
        Span::raw("    "),
        Span::styled(
            "j/k:scroll  s:swap  c:clear snapshot  Esc:close",
            Style::default().fg(MUTED),
        ),
    ]);

    let header_area = Rect::new(inner.x, inner.y, inner.width, 1);
    let body_area = Rect::new(
        inner.x,
        inner.y + 1,
        inner.width,
        inner.height.saturating_sub(1),
    );
    frame.render_widget(Paragraph::new(header), header_area);
    frame.render_widget(
        Paragraph::new(Text::from(lines)).scroll((app.diff.scroll, 0)),
        body_area,
    );
}

pub(super) fn draw_gql_vars_popup(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let popup_w = area.width.saturating_sub(10).min(80);
    let popup_h = area.height.saturating_sub(6).min(20);
    let x = (area.width.saturating_sub(popup_w)) / 2;
    let y = (area.height.saturating_sub(popup_h)) / 2;
    let popup_area = Rect::new(x, y, popup_w, popup_h);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" GraphQL variables (JSON) ")
        .title_style(Style::default().fg(GREEN).bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(GREEN))
        .bg(BG);
    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let row = app.graphql.vars_row;
    let col = app.graphql.vars_col;
    let raw_lines: Vec<&str> = if app.graphql.vars_buffer.is_empty() {
        vec![""]
    } else {
        app.graphql.vars_buffer.split('\n').collect()
    };
    let mut lines: Vec<Line> = raw_lines
        .iter()
        .enumerate()
        .map(|(i, l)| {
            if i == row {
                let c = col.min(l.len());
                let before = &l[..c];
                let cursor_ch = l[c..].chars().next().unwrap_or(' ');
                let after_start = c + cursor_ch.len_utf8().min(l.len() - c);
                let after = &l[after_start..];
                Line::from(vec![
                    Span::styled(before.to_owned(), Style::default().fg(FG)),
                    Span::styled(cursor_ch.to_string(), Style::default().fg(BG).bg(GREEN)),
                    Span::styled(after.to_owned(), Style::default().fg(FG)),
                ])
            } else {
                Line::from(Span::styled((*l).to_owned(), Style::default().fg(FG)))
            }
        })
        .collect();
    if let Some(err) = &app.graphql.vars_error {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            format!("! {err}"),
            Style::default().fg(RED),
        )));
    }
    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}

pub(super) fn draw_gql_schema_popup(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let popup_w = area.width.saturating_sub(10).min(80);
    let popup_h = (app.graphql.operations.len() as u16 + 4)
        .min(area.height.saturating_sub(6))
        .max(6);
    let x = (area.width.saturating_sub(popup_w)) / 2;
    let y = (area.height.saturating_sub(popup_h)) / 2;
    let popup_area = Rect::new(x, y, popup_w, popup_h);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" GraphQL schema ")
        .title_style(Style::default().fg(GREEN).bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(GREEN))
        .bg(BG);
    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    if app.graphql.operations.is_empty() {
        let msg = app
            .graphql
            .schema_error
            .as_ref()
            .map_or_else(|| "No operations".to_owned(), Clone::clone);
        let p = Paragraph::new(Text::styled(msg, Style::default().fg(RED).italic()))
            .alignment(Alignment::Center);
        frame.render_widget(p, inner);
        return;
    }

    for (i, op) in app.graphql.operations.iter().enumerate() {
        let row_y = inner.y + i as u16;
        if row_y >= inner.y + inner.height {
            break;
        }
        let selected = i == app.graphql.schema_popup_selected;
        let row = Rect::new(inner.x, row_y, inner.width, 1);
        if selected {
            frame.render_widget(Paragraph::new("").bg(SURFACE), row);
        }
        let kind_color = match op.kind.as_str() {
            "Query" => GREEN,
            "Mutation" => ORANGE,
            "Subscription" => PURPLE,
            _ => MUTED,
        };
        let line = Line::from(vec![
            Span::styled(
                if selected { " > " } else { "   " },
                Style::default().fg(GREEN),
            ),
            Span::styled(
                format!("{:<13}", op.kind),
                Style::default().fg(kind_color).bold(),
            ),
            Span::styled(&op.name, Style::default().fg(FG)),
        ]);
        frame.render_widget(Paragraph::new(line), row);
    }
}

pub(super) fn draw_grpc_method_popup(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let popup_w = area.width.saturating_sub(10).min(80);
    let popup_h = (app.grpc.methods.len() as u16 + 4)
        .min(area.height.saturating_sub(6))
        .max(6);
    let x = (area.width.saturating_sub(popup_w)) / 2;
    let y = (area.height.saturating_sub(popup_h)) / 2;
    let popup_area = Rect::new(x, y, popup_w, popup_h);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" gRPC method ")
        .title_style(Style::default().fg(GREEN).bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(GREEN))
        .bg(BG);
    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    if app.grpc.methods.is_empty() {
        let msg = Paragraph::new(Text::styled(
            "No methods - load a .proto first (P)",
            Style::default().fg(MUTED).italic(),
        ))
        .alignment(Alignment::Center);
        frame.render_widget(msg, inner);
        return;
    }

    for (i, name) in app.grpc.methods.iter().enumerate() {
        let row_y = inner.y + i as u16;
        if row_y >= inner.y + inner.height {
            break;
        }
        let selected = i == app.grpc.method_popup_selected;
        let row = Rect::new(inner.x, row_y, inner.width, 1);
        if selected {
            frame.render_widget(Paragraph::new("").bg(SURFACE), row);
        }
        let line = Line::from(vec![
            Span::styled(
                if selected { " > " } else { "   " },
                Style::default().fg(GREEN),
            ),
            Span::styled(name, Style::default().fg(FG)),
        ]);
        frame.render_widget(Paragraph::new(line), row);
    }
}
