use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use super::*;
use crate::app::{App, Focus, KvEditorState, Method, RequestTab};
use crate::collections::Auth;

pub(super) fn draw_url_bar(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.ui.focus == Focus::UrlBar;

    let method_str = format!(" {} ", app.request.method.as_str());
    let color = method_color(&app.request.method);

    let line = if app.request.editing_url {
        let pos = app.request.cursor_pos.min(app.request.url.len());
        let before = &app.request.url[..pos];
        let cursor_ch = app.request.url[pos..].chars().next().unwrap_or(' ');
        let after_start = pos + cursor_ch.len_utf8().min(app.request.url.len() - pos);
        let after = &app.request.url[after_start..];

        Line::from(vec![
            Span::styled(method_str, Style::default().fg(BG).bg(color).bold()),
            Span::raw(" "),
            Span::styled(before, Style::default().fg(FG)),
            Span::styled(cursor_ch.to_string(), Style::default().fg(BG).bg(GREEN)),
            Span::styled(after, Style::default().fg(FG)),
        ])
    } else {
        Line::from(vec![
            Span::styled(method_str, Style::default().fg(BG).bg(color).bold()),
            Span::raw(" "),
            Span::styled(&app.request.url, Style::default().fg(FG)),
        ])
    };

    let redirects_part = if app.follow_redirects {
        ""
    } else {
        "  [redirects: off]"
    };
    let timeout_part = if app.timeout.secs == 30 {
        String::new()
    } else {
        format!("  [timeout: {}s]", app.timeout.secs)
    };
    let tls_custom = !app.tls.verify
        || !app.tls.ca_cert.is_empty()
        || !app.tls.client_cert.is_empty()
        || !app.tls.min_version.is_empty();
    let tls_part = if tls_custom {
        if app.tls.verify {
            "  [tls]"
        } else {
            "  [tls: insecure]"
        }
    } else {
        ""
    };
    let proxy_part = if app.proxy.url.is_empty() {
        String::new()
    } else {
        format!("  [proxy: {}]", app.proxy.url)
    };
    let gql_part = if app.request.method == Method::Graphql {
        let vars = app.active_gql_variables();
        if vars.trim().is_empty() {
            "  [graphql]".to_owned()
        } else {
            "  [graphql +vars]".to_owned()
        }
    } else {
        String::new()
    };
    let grpc_part = if app.request.method == Method::Grpc {
        let label = app.active_grpc_method_label();
        if label.is_empty() {
            "  [grpc: no method]".to_owned()
        } else {
            format!("  [grpc: {label}]")
        }
    } else {
        String::new()
    };
    let title = format!(
        " Request{redirects_part}{timeout_part}{tls_part}{proxy_part}{grpc_part}{gql_part} "
    );
    let block = Block::default()
        .title(title)
        .title_style(Style::default().fg(MUTED))
        .borders(Borders::ALL)
        .border_style(if is_focused {
            Style::default().fg(GREEN)
        } else {
            Style::default().fg(MUTED)
        })
        .bg(BG);

    let paragraph = Paragraph::new(line).block(block);
    frame.render_widget(paragraph, area);
}

pub(super) fn draw_request_panel(frame: &mut Frame, app: &App, area: Rect) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(1)])
        .split(area);

    draw_request_tab_bar(frame, app, layout[0]);

    match app.request.tab {
        RequestTab::Body => draw_body_content(frame, app, layout[1]),
        RequestTab::Headers => {
            draw_kv_content(
                frame,
                app.ui.focus == Focus::Body,
                &app.request.header_editor,
                "header",
                layout[1],
            );
        }
        RequestTab::Auth => draw_auth_content(frame, app, layout[1]),
        RequestTab::Params => {
            draw_kv_content(
                frame,
                app.ui.focus == Focus::Body,
                &app.request.param_editor,
                "param",
                layout[1],
            );
        }
    }
}

fn draw_request_tab_bar(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.ui.focus == Focus::Body;
    let tabs = [
        ("Body", RequestTab::Body),
        ("Headers", RequestTab::Headers),
        ("Auth", RequestTab::Auth),
        ("Params", RequestTab::Params),
    ];

    let mut spans = Vec::new();
    spans.push(Span::raw("  "));
    for (i, (label, tab)) in tabs.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("  "));
        }
        let is_active = app.request.tab == *tab;
        spans.push(Span::styled(
            format!(" {label} "),
            if is_active {
                Style::default().fg(GREEN).bold().underlined()
            } else {
                Style::default().fg(MUTED)
            },
        ));
    }

    let border_color = if is_focused {
        Style::default().fg(GREEN)
    } else {
        Style::default().fg(MUTED)
    };

    frame.render_widget(
        Paragraph::new(Line::from(spans)).bg(SURFACE).block(
            Block::default()
                .borders(Borders::LEFT | Borders::RIGHT | Borders::TOP)
                .border_style(border_color),
        ),
        area,
    );
}

fn draw_body_content(frame: &mut Frame, app: &App, area: Rect) {
    use crate::collections::BodyType;

    let is_focused = app.ui.focus == Focus::Body;

    if app.request.body_type != BodyType::Raw {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(1)])
            .split(area);

        let type_bar = Line::from(vec![
            Span::styled("  Type: ", Style::default().fg(MUTED)),
            Span::styled(
                app.request.body_type.label(),
                Style::default().fg(ORANGE).bold(),
            ),
            Span::styled("  (b to change)", Style::default().fg(MUTED)),
        ]);
        frame.render_widget(
            Paragraph::new(type_bar).bg(BG).block(
                Block::default()
                    .borders(Borders::LEFT | Borders::RIGHT)
                    .border_style(if is_focused {
                        Style::default().fg(GREEN)
                    } else {
                        Style::default().fg(MUTED)
                    }),
            ),
            layout[0],
        );

        let label = if app.request.body_type == BodyType::Form {
            "field"
        } else {
            "part"
        };
        draw_kv_content(
            frame,
            is_focused,
            &app.request.form_editor,
            label,
            layout[1],
        );
        return;
    }

    let block = Block::default()
        .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
        .border_style(if is_focused {
            Style::default().fg(GREEN)
        } else {
            Style::default().fg(MUTED)
        })
        .bg(BG);

    let mut header_lines = vec![Line::from(vec![
        Span::styled("  Type: ", Style::default().fg(MUTED)),
        Span::styled("Raw", Style::default().fg(ORANGE).bold()),
        Span::styled(" / ", Style::default().fg(MUTED)),
        Span::styled(
            app.request.content_type.label(),
            Style::default().fg(TEAL).bold(),
        ),
        Span::styled("  (b:type  c:format)", Style::default().fg(MUTED)),
    ])];

    if app.request.body.is_empty() && !app.request.editing_body {
        header_lines.push(Line::from(Span::styled(
            "(empty body)",
            Style::default().fg(MUTED).italic(),
        )));
    } else {
        let body_str = if app.request.body.is_empty() {
            "\n"
        } else {
            &app.request.body
        };
        for (i, line) in body_str.split('\n').enumerate() {
            let num = Span::styled(format!("{:>3} ", i + 1), Style::default().fg(MUTED));

            if app.request.editing_body && i == app.request.body_row {
                let col = app.request.body_col.min(line.len());
                let before = &line[..col];
                let cursor_ch = line[col..].chars().next().unwrap_or(' ');
                let after_start = col + cursor_ch.len_utf8().min(line.len() - col);
                let after = &line[after_start..];

                header_lines.push(Line::from(vec![
                    num,
                    Span::styled(before, Style::default().fg(FG)),
                    Span::styled(cursor_ch.to_string(), Style::default().fg(BG).bg(GREEN)),
                    Span::styled(after, Style::default().fg(FG)),
                ]));
            } else {
                header_lines.push(Line::from(vec![
                    num,
                    Span::styled(line, Style::default().fg(FG)),
                ]));
            }
        }
    }

    let paragraph = Paragraph::new(Text::from(header_lines)).block(block);
    frame.render_widget(paragraph, area);
}

pub(super) fn draw_kv_content(
    frame: &mut Frame,
    is_focused: bool,
    editor: &KvEditorState,
    item_label: &str,
    area: Rect,
) {
    let block = Block::default()
        .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
        .border_style(if is_focused {
            Style::default().fg(GREEN)
        } else {
            Style::default().fg(MUTED)
        })
        .bg(BG);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if editor.editing {
        draw_kv_edit_inline(frame, editor, inner);
        return;
    }

    for (i, (key, value)) in editor.entries.iter().enumerate() {
        let row_y = inner.y + i as u16;
        if row_y >= inner.y + inner.height {
            break;
        }
        let row = Rect::new(inner.x, row_y, inner.width, 1);
        let selected = i == editor.selected;

        if selected && is_focused {
            frame.render_widget(Paragraph::new("").bg(SURFACE), row);
        }

        let line = Line::from(vec![
            Span::styled(
                if selected && is_focused { " > " } else { "   " },
                Style::default().fg(GREEN),
            ),
            Span::styled(key, Style::default().fg(TEAL).bold()),
            Span::styled(": ", Style::default().fg(MUTED)),
            Span::styled(
                value,
                if selected && is_focused {
                    Style::default().fg(FG)
                } else {
                    Style::default().fg(MUTED)
                },
            ),
        ]);
        frame.render_widget(Paragraph::new(line), row);
    }

    let add_y = inner.y + editor.entries.len() as u16;
    if add_y < inner.y + inner.height {
        let row = Rect::new(inner.x, add_y, inner.width, 1);
        let selected = editor.selected >= editor.entries.len();

        if selected && is_focused {
            frame.render_widget(Paragraph::new("").bg(SURFACE), row);
        }

        let line = Line::from(Span::styled(
            format!("   + add {item_label}"),
            if selected && is_focused {
                Style::default().fg(GREEN)
            } else {
                Style::default().fg(MUTED)
            },
        ));
        frame.render_widget(Paragraph::new(line), row);
    }
}

fn draw_kv_edit_inline(frame: &mut Frame, editor: &KvEditorState, area: Rect) {
    let key_style = if editor.edit_field == 0 {
        Style::default().fg(GREEN)
    } else {
        Style::default().fg(FG)
    };
    let val_style = if editor.edit_field == 1 {
        Style::default().fg(GREEN)
    } else {
        Style::default().fg(FG)
    };

    let key_display = if editor.edit_field == 0 {
        format!("{}\u{2588}", &editor.edit_key_buf)
    } else {
        editor.edit_key_buf.clone()
    };
    let val_display = if editor.edit_field == 1 {
        format!("{}\u{2588}", &editor.edit_value_buf)
    } else {
        editor.edit_value_buf.clone()
    };

    let lines = vec![
        Line::default(),
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
    frame.render_widget(paragraph, area);
}

pub(super) fn draw_auth_content(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.ui.focus == Focus::Body;

    let block = Block::default()
        .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
        .border_style(if is_focused {
            Style::default().fg(GREEN)
        } else {
            Style::default().fg(MUTED)
        })
        .bg(BG);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.auth.selecting_type {
        draw_auth_type_selector(frame, app, inner);
        return;
    }

    if app.auth.editing {
        draw_auth_fields(frame, app, inner, true);
        return;
    }

    let type_label = App::AUTH_TYPES[app.auth_type_index()];
    let mut lines = vec![
        Line::default(),
        Line::from(vec![
            Span::styled("  Type: ", Style::default().fg(MUTED)),
            Span::styled(type_label, Style::default().fg(ORANGE).bold()),
            Span::styled("  (t to change)", Style::default().fg(MUTED)),
        ]),
    ];

    lines.push(Line::default());
    if app.auth.config == Auth::None {
        lines.push(Line::from(Span::styled(
            "  No authentication configured",
            Style::default().fg(MUTED).italic(),
        )));
    } else {
        draw_auth_fields_static(app, &mut lines);
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            "  e:edit fields  t:change type",
            Style::default().fg(MUTED),
        )));
    }

    let paragraph = Paragraph::new(Text::from(lines));
    frame.render_widget(paragraph, inner);
}

fn draw_auth_type_selector(frame: &mut Frame, app: &App, area: Rect) {
    let types = App::AUTH_TYPES;
    let current = app.auth_type_index();

    let mut y = area.y;
    for (i, label) in types.iter().enumerate() {
        if y >= area.y + area.height {
            break;
        }
        let row = Rect::new(area.x, y, area.width, 1);
        let selected = i == app.auth.type_selected;
        let is_active = i == current;

        if selected {
            frame.render_widget(Paragraph::new("").bg(SURFACE), row);
        }

        let dot = if is_active { "\u{25cf}" } else { "\u{25cb}" };
        let line = Line::from(vec![
            Span::styled(
                if selected { " > " } else { "   " },
                Style::default().fg(ORANGE),
            ),
            Span::styled(
                format!("{dot} "),
                if is_active {
                    Style::default().fg(ORANGE)
                } else {
                    Style::default().fg(MUTED)
                },
            ),
            Span::styled(
                *label,
                if selected {
                    Style::default().fg(FG)
                } else {
                    Style::default().fg(MUTED)
                },
            ),
        ]);
        frame.render_widget(Paragraph::new(line), row);
        y += 1;
    }
}

fn draw_auth_fields(frame: &mut Frame, app: &App, area: Rect, editing: bool) {
    let (label_a, label_b) = match &app.auth.config {
        Auth::Bearer { .. } => ("Token:    ", ""),
        Auth::Basic { .. } => ("Username: ", "Password: "),
        Auth::ApiKey { .. } => ("Header:   ", "Value:    "),
        Auth::None => return,
    };

    let mut lines = vec![Line::default()];

    if editing {
        let style_a = if app.auth.field == 0 {
            Style::default().fg(GREEN)
        } else {
            Style::default().fg(FG)
        };
        let buf_a = if app.auth.field == 0 {
            format!("{}\u{2588}", &app.auth.buf_a)
        } else {
            app.auth.buf_a.clone()
        };
        lines.push(Line::from(vec![
            Span::styled(format!("  {label_a}"), Style::default().fg(MUTED)),
            Span::styled(buf_a, style_a),
        ]));

        if !label_b.is_empty() {
            let style_b = if app.auth.field == 1 {
                Style::default().fg(GREEN)
            } else {
                Style::default().fg(FG)
            };
            let buf_b = if app.auth.field == 1 {
                format!("{}\u{2588}", &app.auth.buf_b)
            } else {
                app.auth.buf_b.clone()
            };
            lines.push(Line::default());
            lines.push(Line::from(vec![
                Span::styled(format!("  {label_b}"), Style::default().fg(MUTED)),
                Span::styled(buf_b, style_b),
            ]));
        }

        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            if label_b.is_empty() {
                "  Enter:save  Esc:cancel"
            } else {
                "  Tab:switch  Enter:save  Esc:cancel"
            },
            Style::default().fg(MUTED),
        )));
    }

    let paragraph = Paragraph::new(Text::from(lines));
    frame.render_widget(paragraph, area);
}

fn draw_auth_fields_static<'a>(app: &'a App, lines: &mut Vec<Line<'a>>) {
    match &app.auth.config {
        Auth::Bearer { token } => {
            lines.push(Line::from(vec![
                Span::styled("  Token:    ", Style::default().fg(MUTED)),
                Span::styled(token, Style::default().fg(FG)),
            ]));
        }
        Auth::Basic { username, password } => {
            lines.push(Line::from(vec![
                Span::styled("  Username: ", Style::default().fg(MUTED)),
                Span::styled(username, Style::default().fg(FG)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Password: ", Style::default().fg(MUTED)),
                Span::styled(
                    "\u{2022}".repeat(password.len().clamp(4, 20)),
                    Style::default().fg(FG),
                ),
            ]));
        }
        Auth::ApiKey { header, value } => {
            lines.push(Line::from(vec![
                Span::styled("  Header:   ", Style::default().fg(MUTED)),
                Span::styled(header, Style::default().fg(FG)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Value:    ", Style::default().fg(MUTED)),
                Span::styled(value, Style::default().fg(FG)),
            ]));
        }
        Auth::None => {}
    }
}
