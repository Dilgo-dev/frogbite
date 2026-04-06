use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem};

use super::*;
use crate::app::{App, Focus, Method, SidebarItem};

pub(super) fn draw_sidebar(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.ui.focus == Focus::Sidebar;

    let env_label = app.active_env_name().map(|n| format!(" frogbite [{n}] "));
    let title = if app.sidebar.confirm_delete {
        " Delete? (y=yes, Esc=no) ".to_owned()
    } else {
        env_label.unwrap_or_else(|| " frogbite ".to_owned())
    };

    let title_style = if app.sidebar.confirm_delete {
        Style::default().fg(red()).bold()
    } else if app.env.active_id.is_some() {
        Style::default().fg(teal()).bold()
    } else {
        Style::default().fg(green()).bold()
    };

    let block = Block::default()
        .title(title)
        .title_style(title_style)
        .borders(Borders::ALL)
        .border_style(if is_focused {
            Style::default().fg(green())
        } else {
            Style::default().fg(muted())
        })
        .bg(bg());

    let sidebar_items = app.sidebar_items();
    let items: Vec<ListItem> = sidebar_items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let selected = i == app.sidebar.selected;
            let editing_name = selected && app.sidebar.editing_name;

            let line = match item {
                SidebarItem::Folder(f) => {
                    let arrow = if f.expanded { "v " } else { "> " };
                    let name = if editing_name {
                        format!("{}\u{2588}", &app.sidebar.edit_buffer)
                    } else {
                        f.name.clone()
                    };
                    let name_style = if editing_name {
                        Style::default().fg(green())
                    } else if selected {
                        Style::default().fg(fg()).bold()
                    } else {
                        Style::default().fg(muted()).bold()
                    };
                    Line::from(vec![
                        Span::styled(arrow, Style::default().fg(muted())),
                        Span::styled(name, name_style),
                    ])
                }
                SidebarItem::Request(req) => {
                    let method_str = format!("{:>6}", Method::from_str(&req.method).as_str());
                    let method = Method::from_str(&req.method);
                    let is_active = app.sidebar.active_request_id.as_deref() == Some(&req.id);

                    let name = if editing_name {
                        format!("{}\u{2588}", &app.sidebar.edit_buffer)
                    } else {
                        req.name.clone()
                    };
                    let name_style = if editing_name {
                        Style::default().fg(green())
                    } else if selected || is_active {
                        Style::default().fg(fg())
                    } else {
                        Style::default().fg(muted())
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
                        Style::default().fg(green())
                    } else {
                        Style::default().fg(muted())
                    },
                )]),
            };

            if selected {
                ListItem::new(line).bg(surface())
            } else {
                ListItem::new(line)
            }
        })
        .collect();

    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}
