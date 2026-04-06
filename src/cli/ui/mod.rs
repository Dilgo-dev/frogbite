mod modals;
mod request;
mod response;
mod settings;
mod sidebar;

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
    match app.view {
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
    response::draw_response(frame, app, content_layout[2]);

    if app.method_popup {
        modals::draw_method_popup(frame, app);
    }
    if app.history_open {
        modals::draw_history_overlay(frame, app);
    }
    if app.curl_import_open {
        modals::draw_curl_import_popup(frame, app);
    }
    if app.curl_export_open {
        modals::draw_curl_export_popup(frame, app);
    }
    if app.postman_import_open {
        modals::draw_postman_import_popup(frame, app);
    }
    if app.env_popup_open || app.env_renaming {
        modals::draw_env_popup(frame, app);
    }
    if app.env_import_open {
        modals::draw_env_import_popup(frame, app);
    }
    if app.timeout_popup_open {
        modals::draw_timeout_popup(frame, app);
    }
    if app.tls_popup_open {
        modals::draw_tls_popup(frame, app);
    }
    if app.cookies_popup_open {
        modals::draw_cookies_popup(frame, app);
    }
    if app.extractors_popup_open {
        modals::draw_extractors_popup(frame, app);
    }
    if app.assertions_popup_open {
        modals::draw_assertions_popup(frame, app);
    }
    if app.env_editor_open {
        modals::draw_env_editor(frame, app);
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
    } else if app.assertions_popup_open {
        if app.assertion_editing {
            "type assertion  Enter:save  Esc:cancel"
        } else {
            "j/k:nav  a:add  e:edit  d:delete  Esc:close"
        }
    } else if app.extractors_popup_open {
        if app.extractor_editor.editing {
            "type name/path  Tab:switch  Enter:save  Esc:cancel"
        } else {
            "j/k:nav  a:add  e:edit  d:delete  Esc:close"
        }
    } else if app.cookies_popup_open {
        "j/k:nav  d:delete  D:clear all  Esc:close"
    } else if app.tls_popup_open {
        if app.tls_editing {
            "type path  Enter:save  Esc:cancel"
        } else {
            "j/k:nav  Enter:toggle/edit  d:clear  Esc:close"
        }
    } else if app.timeout_popup_open {
        "type seconds  Enter:save  Esc:cancel"
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
            View::Main if app.confirm_delete => "y:confirm delete  any:cancel",
            View::Main => match app.focus {
                Focus::Sidebar => {
                    "j/k:nav  a:new  A:folder  d:del  D:dup  r:rename  i:curl  I:postman  q:quit"
                }
                Focus::UrlBar => {
                    "e:edit  m:method  A:auth  R:redir  T:tout  S:tls  C:cookies  X:extract  V:assert"
                }
                Focus::Body => match app.request_tab {
                    RequestTab::Body => {
                        if app.body_type == BodyType::Raw {
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

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(help, Style::default().fg(MUTED)))).bg(SURFACE),
        help_area,
    );
}
