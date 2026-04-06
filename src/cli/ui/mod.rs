mod modals;
mod request;
mod response;
mod settings;
mod sidebar;
mod ws;

use ratatui::{prelude::*, widgets::Paragraph};

use crate::app::{App, Focus, Method, RequestTab, View};
use crate::collections::BodyType;

pub const GREEN: Color = Color::Rgb(124, 179, 66);
pub const ORANGE: Color = Color::Rgb(255, 111, 0);
pub const PURPLE: Color = Color::Rgb(156, 39, 176);
pub const RED: Color = Color::Rgb(211, 47, 47);
pub const YELLOW: Color = Color::Rgb(255, 143, 0);
pub const TEAL: Color = Color::Rgb(0, 137, 123);
pub const MUTED: Color = Color::Rgb(107, 138, 107);
pub const SURFACE: Color = Color::Rgb(22, 34, 32);
pub const BG: Color = Color::Rgb(13, 27, 26);
pub const FG: Color = Color::Rgb(232, 232, 224);

pub const fn method_color(method: &Method) -> Color {
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

pub const fn status_color(status: u16) -> Color {
    match status {
        200..=299 => GREEN,
        300..=399 => TEAL,
        400..=499 => YELLOW,
        500..=599 => RED,
        _ => MUTED,
    }
}

pub fn draw(frame: &mut Frame, app: &App) {
    match app.ui.view {
        View::Main => draw_main(frame, app),
        View::Settings => settings::draw_settings(frame, app),
    }
}

fn draw_main(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let main_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(30), Constraint::Min(1)])
        .split(area);

    sidebar::draw_sidebar(frame, app, main_layout[0]);

    let content_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Percentage(35),
            Constraint::Min(1),
        ])
        .split(main_layout[1]);

    request::draw_url_bar(frame, app, content_layout[0]);
    request::draw_request_panel(frame, app, content_layout[1]);
    if app.ws_active() {
        ws::draw_ws_panel(frame, app, content_layout[2]);
    } else {
        response::draw_response(frame, app, content_layout[2]);
    }

    if app.method_popup.open {
        modals::draw_method_popup(frame, app);
    }
    if app.history.open {
        modals::draw_history_overlay(frame, app);
    }
    if app.curl_io.import_open {
        modals::draw_curl_import_popup(frame, app);
    }
    if app.curl_io.export_open {
        modals::draw_curl_export_popup(frame, app);
    }
    if app.postman_io.open {
        modals::draw_postman_import_popup(frame, app);
    }
    if app.env.popup_open || app.env.renaming {
        modals::draw_env_popup(frame, app);
    }
    if app.env.import.open {
        modals::draw_env_import_popup(frame, app);
    }
    if app.timeout.popup_open {
        modals::draw_timeout_popup(frame, app);
    }
    if app.tls.popup_open {
        modals::draw_tls_popup(frame, app);
    }
    if app.cookies.popup_open {
        modals::draw_cookies_popup(frame, app);
    }
    if app.extractors.popup_open {
        modals::draw_extractors_popup(frame, app);
    }
    if app.assertions.popup_open {
        modals::draw_assertions_popup(frame, app);
    }
    if app.env.editor.open {
        modals::draw_env_editor(frame, app);
    }
}

#[allow(clippy::too_many_lines)]
pub fn draw_help_bar(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let help_area = Rect::new(0, area.height.saturating_sub(1), area.width, 1);

    let help = if app.ws.input_editing {
        "type message  Enter:send  Esc:cancel"
    } else if app.ws_active() && app.ui.focus == Focus::Response {
        "i:type  Enter:send  d:disconnect  c:clear  x:close  j/k:scroll"
    } else if app.response.searching {
        "type search term  Enter:search  Esc:cancel"
    } else if app.auth.editing {
        "type value  Tab:switch  Enter:save  Esc:cancel"
    } else if app.auth.selecting_type {
        "j/k:navigate  Enter:select  Esc:cancel"
    } else if app.request.form_editor.editing
        || app.request.header_editor.editing
        || app.request.param_editor.editing
    {
        "type key/value  Tab:switch  Enter:save  Esc:cancel"
    } else if app.env.editor.editing_var {
        "type key/value  Tab:switch field  Enter:save  Esc:cancel"
    } else if app.env.editor.open {
        "j/k:navigate  Enter/a:edit  d:delete  s:secret  Esc:back"
    } else if app.env.renaming {
        "type name  Enter:confirm  Esc:cancel"
    } else if app.assertions.popup_open {
        if app.assertions.editing {
            "type assertion  Enter:save  Esc:cancel"
        } else {
            "j/k:nav  a:add  e:edit  d:delete  Esc:close"
        }
    } else if app.extractors.popup_open {
        if app.extractors.editor.editing {
            "type name/path  Tab:switch  Enter:save  Esc:cancel"
        } else {
            "j/k:nav  a:add  e:edit  d:delete  Esc:close"
        }
    } else if app.cookies.popup_open {
        "j/k:nav  d:delete  D:clear all  Esc:close"
    } else if app.tls.popup_open {
        if app.tls.editing {
            "type path  Enter:save  Esc:cancel"
        } else {
            "j/k:nav  Enter:toggle/edit  d:clear  Esc:close"
        }
    } else if app.timeout.popup_open {
        "type seconds  Enter:save  Esc:cancel"
    } else if app.env.import.open {
        "type path  Enter:import  Esc:cancel"
    } else if app.env.popup_open {
        "j/k:nav  Enter:select  a:new  d:del  r:rename  e:vars  i:.env  Esc:close"
    } else if app.postman_io.open {
        "type path  Enter:import  Esc:cancel"
    } else if app.curl_io.export_open {
        "Esc:close"
    } else if app.curl_io.import_open {
        "paste cURL  Ctrl+S:import  Esc:cancel"
    } else if app.method_popup.open {
        "j/k:navigate  Enter:select  Esc:cancel"
    } else if app.history.open {
        "j/k:navigate  Enter:load  Esc:close"
    } else {
        match app.ui.view {
            View::Settings => "j/k:navigate  Space/Enter:toggle  Esc:back",
            View::Main if app.sidebar.editing_name => "type name  Enter:confirm  Esc:cancel",
            View::Main if app.request.editing_url => "type URL  arrows:move  Enter:send  Esc:stop",
            View::Main if app.request.editing_body => {
                "type body  arrows:move  Tab:indent  Enter:newline  Esc:stop"
            }
            View::Main if app.sidebar.confirm_delete => "y:confirm delete  any:cancel",
            View::Main => match app.ui.focus {
                Focus::Sidebar => {
                    "j/k:nav  a:new  A:folder  d:del  D:dup  r:rename  i:curl  I:postman  q:quit"
                }
                Focus::UrlBar => {
                    "e:edit  m:method  A:auth  R:redir  T:tout  S:tls  C:cookies  X:extract  V:assert"
                }
                Focus::Body => match app.request.tab {
                    RequestTab::Body => {
                        if app.request.body_type == BodyType::Raw {
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
                Focus::Response => "j/k:scroll  /:search  n/N:match  y:copy  1:body 2:headers",
            },
        }
    };

    let mut spans = vec![Span::styled(help, Style::default().fg(MUTED))];
    if let Some(v) = &app.update_available {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            format!("• update {v} available (frogbite update)"),
            Style::default().fg(Color::Rgb(255, 111, 0)),
        ));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)).bg(SURFACE), help_area);
}
