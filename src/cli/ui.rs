use ratatui::{
    prelude::*,
    widgets::{
        Block, Borders, Clear, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState,
    },
};

use crate::app::{App, Focus, Method, RequestTab, ResponseTab, SidebarItem, View};
use crate::collections::Auth;

const GREEN: Color = Color::Rgb(124, 179, 66);
const ORANGE: Color = Color::Rgb(255, 111, 0);
const PURPLE: Color = Color::Rgb(156, 39, 176);
const RED: Color = Color::Rgb(211, 47, 47);
const YELLOW: Color = Color::Rgb(255, 143, 0);
const TEAL: Color = Color::Rgb(0, 137, 123);
const MUTED: Color = Color::Rgb(107, 138, 107);
const SURFACE: Color = Color::Rgb(22, 34, 32);
const BG: Color = Color::Rgb(13, 27, 26);
const FG: Color = Color::Rgb(232, 232, 224);

const fn method_color(method: &Method) -> Color {
    match method {
        Method::Get => GREEN,
        Method::Post => ORANGE,
        Method::Put => PURPLE,
        Method::Patch => YELLOW,
        Method::Delete => RED,
        Method::Head => TEAL,
        Method::Options => MUTED,
    }
}

const fn status_color(status: u16) -> Color {
    match status {
        200..=299 => GREEN,
        300..=399 => TEAL,
        400..=499 => YELLOW,
        500..=599 => RED,
        _ => MUTED,
    }
}

pub fn draw(frame: &mut Frame, app: &App) {
    match app.view {
        View::Main => draw_main(frame, app),
        View::Settings => draw_settings(frame, app),
    }
}

fn draw_main(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let main_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(30), Constraint::Min(1)])
        .split(area);

    draw_sidebar(frame, app, main_layout[0]);

    let content_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Percentage(35),
            Constraint::Min(1),
        ])
        .split(main_layout[1]);

    draw_url_bar(frame, app, content_layout[0]);
    draw_request_panel(frame, app, content_layout[1]);
    draw_response(frame, app, content_layout[2]);

    if app.method_popup {
        draw_method_popup(frame, app);
    }
    if app.history_open {
        draw_history_overlay(frame, app);
    }
    if app.curl_import_open {
        draw_curl_import_popup(frame, app);
    }
    if app.curl_export_open {
        draw_curl_export_popup(frame, app);
    }
    if app.postman_import_open {
        draw_postman_import_popup(frame, app);
    }
    if app.env_popup_open || app.env_renaming {
        draw_env_popup(frame, app);
    }
    if app.env_import_open {
        draw_env_import_popup(frame, app);
    }
    if app.env_editor_open {
        draw_env_editor(frame, app);
    }
}

fn draw_sidebar(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.focus == Focus::Sidebar;

    let env_label = app.active_env_name().map(|n| format!(" frogbite [{n}] "));
    let title = if app.confirm_delete {
        " Delete? (d=yes, Esc=no) ".to_owned()
    } else {
        env_label.unwrap_or_else(|| " frogbite ".to_owned())
    };

    let title_style = if app.confirm_delete {
        Style::default().fg(RED).bold()
    } else if app.active_env_id.is_some() {
        Style::default().fg(TEAL).bold()
    } else {
        Style::default().fg(GREEN).bold()
    };

    let block = Block::default()
        .title(title)
        .title_style(title_style)
        .borders(Borders::ALL)
        .border_style(if is_focused {
            Style::default().fg(GREEN)
        } else {
            Style::default().fg(MUTED)
        })
        .bg(BG);

    let sidebar_items = app.sidebar_items();
    let items: Vec<ListItem> = sidebar_items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let selected = i == app.sidebar_selected;
            let editing_name = selected && app.editing_sidebar_name;

            let line = match item {
                SidebarItem::Folder(f) => {
                    let arrow = if f.expanded { "v " } else { "> " };
                    let name = if editing_name {
                        format!("{}\u{2588}", &app.sidebar_edit_buffer)
                    } else {
                        f.name.clone()
                    };
                    let name_style = if editing_name {
                        Style::default().fg(GREEN)
                    } else if selected {
                        Style::default().fg(FG).bold()
                    } else {
                        Style::default().fg(MUTED).bold()
                    };
                    Line::from(vec![
                        Span::styled(arrow, Style::default().fg(MUTED)),
                        Span::styled(name, name_style),
                    ])
                }
                SidebarItem::Request(req) => {
                    let method_str = format!("{:>6}", Method::from_str(&req.method).as_str());
                    let method = Method::from_str(&req.method);
                    let is_active = app.active_request_id.as_deref() == Some(&req.id);

                    let name = if editing_name {
                        format!("{}\u{2588}", &app.sidebar_edit_buffer)
                    } else {
                        req.name.clone()
                    };
                    let name_style = if editing_name {
                        Style::default().fg(GREEN)
                    } else if selected || is_active {
                        Style::default().fg(FG)
                    } else {
                        Style::default().fg(MUTED)
                    };

                    let indent = if req.folder_id.is_some() { "  " } else { "" };
                    Line::from(vec![
                        Span::raw(indent),
                        Span::styled(
                            method_str,
                            Style::default().fg(method_color(&method)).bold(),
                        ),
                        Span::raw("  "),
                        Span::styled(name, name_style),
                    ])
                }
                SidebarItem::NewRequest => Line::from(vec![Span::styled(
                    "  + new request",
                    if selected {
                        Style::default().fg(GREEN)
                    } else {
                        Style::default().fg(MUTED)
                    },
                )]),
            };

            if selected {
                ListItem::new(line).bg(SURFACE)
            } else {
                ListItem::new(line)
            }
        })
        .collect();

    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}

fn draw_url_bar(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.focus == Focus::UrlBar;

    let method_str = format!(" {} ", app.method.as_str());
    let color = method_color(&app.method);

    let line = if app.editing_url {
        let pos = app.cursor_pos.min(app.url.len());
        let before = &app.url[..pos];
        let cursor_ch = app.url[pos..].chars().next().unwrap_or(' ');
        let after_start = pos + cursor_ch.len_utf8().min(app.url.len() - pos);
        let after = &app.url[after_start..];

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
            Span::styled(&app.url, Style::default().fg(FG)),
        ])
    };

    let block = Block::default()
        .title(" Request ")
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

fn draw_request_panel(frame: &mut Frame, app: &App, area: Rect) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(1)])
        .split(area);

    draw_request_tab_bar(frame, app, layout[0]);

    match app.request_tab {
        RequestTab::Body => draw_body_content(frame, app, layout[1]),
        RequestTab::Headers => {
            draw_kv_content(
                frame,
                app.focus == Focus::Body,
                &app.header_editor,
                "header",
                layout[1],
            );
        }
        RequestTab::Auth => draw_auth_content(frame, app, layout[1]),
        RequestTab::Params => {
            draw_kv_content(
                frame,
                app.focus == Focus::Body,
                &app.param_editor,
                "param",
                layout[1],
            );
        }
    }
}

fn draw_request_tab_bar(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.focus == Focus::Body;
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
        let is_active = app.request_tab == *tab;
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

    let is_focused = app.focus == Focus::Body;

    if app.body_type != BodyType::Raw {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(1)])
            .split(area);

        let type_bar = Line::from(vec![
            Span::styled("  Type: ", Style::default().fg(MUTED)),
            Span::styled(app.body_type.label(), Style::default().fg(ORANGE).bold()),
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

        let label = if app.body_type == BodyType::Form {
            "field"
        } else {
            "part"
        };
        draw_kv_content(frame, is_focused, &app.form_editor, label, layout[1]);
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
        Span::styled(app.content_type.label(), Style::default().fg(TEAL).bold()),
        Span::styled("  (b:type  c:format)", Style::default().fg(MUTED)),
    ])];

    if app.body.is_empty() && !app.editing_body {
        header_lines.push(Line::from(Span::styled(
            "(empty body)",
            Style::default().fg(MUTED).italic(),
        )));
    } else {
        let body_str = if app.body.is_empty() { "\n" } else { &app.body };
        for (i, line) in body_str.split('\n').enumerate() {
            let num = Span::styled(format!("{:>3} ", i + 1), Style::default().fg(MUTED));

            if app.editing_body && i == app.body_row {
                let col = app.body_col.min(line.len());
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

fn draw_kv_content(
    frame: &mut Frame,
    is_focused: bool,
    editor: &crate::app::KvEditorState,
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

fn draw_kv_edit_inline(frame: &mut Frame, editor: &crate::app::KvEditorState, area: Rect) {
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

fn draw_response(frame: &mut Frame, app: &App, area: Rect) {
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
            Line::from(vec![
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
            ])
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
            let lines: Vec<Line> = resp
                .headers
                .iter()
                .map(|(k, v)| {
                    Line::from(vec![
                        Span::styled(k, Style::default().fg(MUTED)),
                        Span::styled(": ", Style::default().fg(MUTED)),
                        Span::styled(v, Style::default().fg(FG)),
                    ])
                })
                .collect();
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

fn draw_method_popup(frame: &mut Frame, app: &App) {
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
        let selected = i == app.method_popup_selected;

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

fn draw_history_overlay(frame: &mut Frame, app: &App) {
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

    if app.history.is_empty() {
        let msg = Paragraph::new(Text::styled(
            "No history yet",
            Style::default().fg(MUTED).italic(),
        ))
        .alignment(Alignment::Center);
        frame.render_widget(msg, inner);
        return;
    }

    for (i, entry) in app.history.iter().rev().enumerate() {
        let row_y = inner.y + i as u16;
        if row_y >= inner.y + inner.height {
            break;
        }
        let selected = i == app.history_selected;
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

fn draw_curl_import_popup(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let popup_w = area.width.saturating_sub(10).min(70);
    let popup_h = area.height.saturating_sub(6).min(14);
    let x = (area.width.saturating_sub(popup_w)) / 2;
    let y = (area.height.saturating_sub(popup_h)) / 2;
    let popup_area = Rect::new(x, y, popup_w, popup_h);

    frame.render_widget(Clear, popup_area);

    let title = if app.curl_import_error {
        " Import cURL (invalid) "
    } else {
        " Import cURL "
    };
    let title_style = if app.curl_import_error {
        Style::default().fg(RED).bold()
    } else {
        Style::default().fg(GREEN).bold()
    };

    let block = Block::default()
        .title(title)
        .title_style(title_style)
        .borders(Borders::ALL)
        .border_style(if app.curl_import_error {
            Style::default().fg(RED)
        } else {
            Style::default().fg(GREEN)
        })
        .bg(BG);

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let display = format!("{}\u{2588}", &app.curl_import_buffer);
    let paragraph = Paragraph::new(Text::styled(&display, Style::default().fg(FG)))
        .wrap(ratatui::widgets::Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}

fn draw_curl_export_popup(frame: &mut Frame, app: &App) {
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
        &app.curl_export_content,
        Style::default().fg(FG),
    ))
    .wrap(ratatui::widgets::Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}

fn draw_postman_import_popup(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let popup_w = area.width.saturating_sub(10).min(70);
    let popup_h: u16 = 5;
    let x = (area.width.saturating_sub(popup_w)) / 2;
    let y = (area.height.saturating_sub(popup_h)) / 2;
    let popup_area = Rect::new(x, y, popup_w, popup_h);

    frame.render_widget(Clear, popup_area);

    let title = if app.postman_import_error {
        " Import Postman (invalid) "
    } else {
        " Import Postman "
    };
    let title_style = if app.postman_import_error {
        Style::default().fg(RED).bold()
    } else {
        Style::default().fg(GREEN).bold()
    };

    let block = Block::default()
        .title(title)
        .title_style(title_style)
        .borders(Borders::ALL)
        .border_style(if app.postman_import_error {
            Style::default().fg(RED)
        } else {
            Style::default().fg(GREEN)
        })
        .bg(BG);

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let display = format!("{}\u{2588}", &app.postman_import_buffer);
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

fn draw_env_popup(frame: &mut Frame, app: &App) {
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
        let selected = app.env_popup_selected == 0;
        let is_active = app.active_env_id.is_none();

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
    for (i, env) in app.environments.iter().enumerate() {
        let row_y = inner.y + (i as u16 + 1);
        if row_y >= inner.y + inner.height {
            break;
        }
        let row = Rect::new(inner.x, row_y, inner.width, 1);
        let idx = i + 1;
        let selected = app.env_popup_selected == idx;
        let is_active = app.active_env_id.as_deref() == Some(&env.id);

        if selected {
            frame.render_widget(Paragraph::new("").bg(SURFACE), row);
        }

        let dot = if is_active { "\u{25cf}" } else { "\u{25cb}" };
        let name = if app.env_renaming && selected {
            format!("{}\u{2588}", &app.env_name_buffer)
        } else {
            env.name.clone()
        };
        let name_style = if app.env_renaming && selected {
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

fn draw_env_import_popup(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let popup_w = area.width.saturating_sub(10).min(70);
    let popup_h: u16 = 5;
    let x = (area.width.saturating_sub(popup_w)) / 2;
    let y = (area.height.saturating_sub(popup_h)) / 2;
    let popup_area = Rect::new(x, y, popup_w, popup_h);

    frame.render_widget(Clear, popup_area);

    let title = if app.env_import_error {
        " Import .env (invalid) "
    } else {
        " Import .env "
    };
    let title_style = if app.env_import_error {
        Style::default().fg(RED).bold()
    } else {
        Style::default().fg(TEAL).bold()
    };

    let block = Block::default()
        .title(title)
        .title_style(title_style)
        .borders(Borders::ALL)
        .border_style(if app.env_import_error {
            Style::default().fg(RED)
        } else {
            Style::default().fg(TEAL)
        })
        .bg(BG);

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let display = format!("{}\u{2588}", &app.env_import_buffer);
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

fn draw_env_editor(frame: &mut Frame, app: &App) {
    let env = app.environments.iter().find(|e| e.id == app.env_editor_id);
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

    if app.env_editing_var {
        draw_env_var_edit(frame, app, inner);
        return;
    }

    for (i, var) in vars.iter().enumerate() {
        let row_y = inner.y + i as u16;
        if row_y >= inner.y + inner.height {
            break;
        }
        let row = Rect::new(inner.x, row_y, inner.width, 1);
        let selected = i == app.env_editor_selected;

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
        let selected = app.env_editor_selected >= vars.len();

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

    let key_style = if app.env_var_field == 0 {
        Style::default().fg(GREEN)
    } else {
        Style::default().fg(FG)
    };
    let val_style = if app.env_var_field == 1 {
        Style::default().fg(GREEN)
    } else {
        Style::default().fg(FG)
    };

    let key_display = if app.env_var_field == 0 {
        format!("{}\u{2588}", &app.env_var_key_buffer)
    } else {
        app.env_var_key_buffer.clone()
    };
    let val_display = if app.env_var_field == 1 {
        format!("{}\u{2588}", &app.env_var_value_buffer)
    } else {
        app.env_var_value_buffer.clone()
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

fn draw_auth_content(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.focus == Focus::Body;

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

    if app.auth_selecting_type {
        draw_auth_type_selector(frame, app, inner);
        return;
    }

    if app.auth_editing {
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
    if app.auth == Auth::None {
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
        let selected = i == app.auth_type_selected;
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
    let (label_a, label_b) = match &app.auth {
        Auth::Bearer { .. } => ("Token:    ", ""),
        Auth::Basic { .. } => ("Username: ", "Password: "),
        Auth::ApiKey { .. } => ("Header:   ", "Value:    "),
        Auth::None => return,
    };

    let mut lines = vec![Line::default()];

    if editing {
        let style_a = if app.auth_field == 0 {
            Style::default().fg(GREEN)
        } else {
            Style::default().fg(FG)
        };
        let buf_a = if app.auth_field == 0 {
            format!("{}\u{2588}", &app.auth_buf_a)
        } else {
            app.auth_buf_a.clone()
        };
        lines.push(Line::from(vec![
            Span::styled(format!("  {label_a}"), Style::default().fg(MUTED)),
            Span::styled(buf_a, style_a),
        ]));

        if !label_b.is_empty() {
            let style_b = if app.auth_field == 1 {
                Style::default().fg(GREEN)
            } else {
                Style::default().fg(FG)
            };
            let buf_b = if app.auth_field == 1 {
                format!("{}\u{2588}", &app.auth_buf_b)
            } else {
                app.auth_buf_b.clone()
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
    match &app.auth {
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

fn draw_settings(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let block = Block::default()
        .title(" Settings ")
        .title_style(Style::default().fg(GREEN).bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(GREEN))
        .bg(BG);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let items = app.settings_items();

    for (i, (label, value)) in items.iter().enumerate() {
        let y = inner.y + i as u16;
        if y >= inner.y + inner.height {
            break;
        }

        let selected = i == app.settings_selected;
        let row_area = Rect::new(inner.x, y, inner.width, 1);

        if selected {
            frame.render_widget(Paragraph::new("").bg(SURFACE), row_area);
        }

        let indicator = if selected { "> " } else { "  " };
        let toggle = if *value { "[x]" } else { "[ ]" };
        let toggle_color = if *value { GREEN } else { MUTED };

        let line = Line::from(vec![
            Span::styled(indicator, Style::default().fg(GREEN)),
            Span::styled(toggle, Style::default().fg(toggle_color).bold()),
            Span::raw("  "),
            Span::styled(
                *label,
                if selected {
                    Style::default().fg(FG)
                } else {
                    Style::default().fg(MUTED)
                },
            ),
        ]);

        frame.render_widget(Paragraph::new(line), row_area);
    }
}

pub fn draw_help_bar(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let help_area = Rect::new(0, area.height.saturating_sub(1), area.width, 1);

    let help = if app.response_searching {
        "type search term  Enter:search  Esc:cancel"
    } else if app.auth_editing {
        "type value  Tab:switch  Enter:save  Esc:cancel"
    } else if app.auth_selecting_type {
        "j/k:navigate  Enter:select  Esc:cancel"
    } else if app.form_editor.editing || app.header_editor.editing || app.param_editor.editing {
        "type key/value  Tab:switch  Enter:save  Esc:cancel"
    } else if app.env_editing_var {
        "type key/value  Tab:switch field  Enter:save  Esc:cancel"
    } else if app.env_editor_open {
        "j/k:navigate  Enter/a:edit  d:delete  s:secret  Esc:back"
    } else if app.env_renaming {
        "type name  Enter:confirm  Esc:cancel"
    } else if app.env_import_open {
        "type path  Enter:import  Esc:cancel"
    } else if app.env_popup_open {
        "j/k:nav  Enter:select  a:new  d:del  r:rename  e:vars  i:.env  Esc:close"
    } else if app.postman_import_open {
        "type path  Enter:import  Esc:cancel"
    } else if app.curl_export_open {
        "Esc:close"
    } else if app.curl_import_open {
        "paste cURL  Ctrl+S:import  Esc:cancel"
    } else if app.method_popup {
        "j/k:navigate  Enter:select  Esc:cancel"
    } else if app.history_open {
        "j/k:navigate  Enter:load  Esc:close"
    } else {
        match app.view {
            View::Settings => "j/k:navigate  Space/Enter:toggle  Esc:back",
            View::Main if app.editing_sidebar_name => "type name  Enter:confirm  Esc:cancel",
            View::Main if app.editing_url => "type URL  arrows:move  Enter:send  Esc:stop",
            View::Main if app.editing_body => {
                "type body  arrows:move  Tab:indent  Enter:newline  Esc:stop"
            }
            View::Main if app.confirm_delete => "d:confirm delete  any:cancel",
            View::Main => match app.focus {
                Focus::Sidebar => {
                    "j/k:nav  a:new  d:del  D:dup  r:rename  i:curl  I:postman  E:env  q:quit"
                }
                Focus::UrlBar => {
                    "e:edit  m:method  A:auth  Enter:send  h:history  E:env  s:settings"
                }
                Focus::Body => match app.request_tab {
                    RequestTab::Body => {
                        if app.body_type == crate::collections::BodyType::Raw {
                            "1-4:tabs  b:type  c:format  e:edit  Enter:send  q:quit"
                        } else {
                            "1-4:tabs  b:type  j/k:nav  e:edit  a:add  d:del  Enter:send"
                        }
                    }
                    RequestTab::Headers | RequestTab::Params => {
                        "1-4:tabs  j/k:nav  e:edit  a:add  d:del  Enter:send"
                    }
                    RequestTab::Auth => "1-4:tabs  t:type  e:edit  Enter:send",
                },
                Focus::Response => "j/k:scroll  /:search  n/N:next/prev  1:body 2:headers",
            },
        }
    };

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(help, Style::default().fg(MUTED)))).bg(SURFACE),
        help_area,
    );
}
