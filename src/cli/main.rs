mod app;
mod collections;
mod curl;
mod history;
mod postman;
mod settings;
mod ui;

use std::io;

use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{prelude::*, widgets::Paragraph};

#[allow(unused_imports)]
use app::{App, Focus, Method, ResponseTab, View};

fn main() -> io::Result<()> {
    let s = settings::load();

    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    if s.splash_animation {
        show_splash(&mut terminal)?;
    }

    let result = run(&mut terminal);

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;

    result
}

const LOGO: &[&str] = &[
    r"___________                   __________.__  __          ",
    r"\_   _____/______  ____   ____\______   \__|/  |_  ____ ",
    r" |    __) \_  __ \/  _ \ / ___\|    |  _/  \   __\/ __ \",
    r" |     \   |  | \(  <_> ) /_/  >    |   \  ||  | \  ___/",
    r" \___  /   |__|   \____/\___  /|______  /__||__|  \___  >",
    r"     \/                /_____/        \/              \/",
];

fn show_splash(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    let logo_width = LOGO.iter().map(|l| l.len()).max().unwrap_or(0) as u16;
    let logo_height = LOGO.len() as u16;

    for step in 0..=logo_height {
        terminal.draw(|frame| {
            let area = frame.area();
            let x = area.width.saturating_sub(logo_width) / 2;
            let y = area.height.saturating_sub(logo_height + 2) / 2;

            for (i, line) in LOGO.iter().enumerate() {
                if (i as u16) >= step {
                    break;
                }
                let line_area = Rect::new(x, y + i as u16, logo_width, 1);

                let brightness = if (i as u16) == step - 1 {
                    Color::Rgb(180, 230, 120) // bright for newest line
                } else {
                    Color::Rgb(124, 179, 66) // normal green
                };

                frame.render_widget(
                    Paragraph::new(Line::styled(*line, Style::default().fg(brightness))),
                    line_area,
                );
            }

            if step == logo_height {
                let sub = "API tester for the terminal";
                let sub_x = area.width.saturating_sub(sub.len() as u16) / 2;
                let sub_area = Rect::new(sub_x, y + logo_height + 1, sub.len() as u16, 1);
                frame.render_widget(
                    Paragraph::new(Line::styled(
                        sub,
                        Style::default().fg(Color::Rgb(107, 138, 107)),
                    )),
                    sub_area,
                );
            }
        })?;

        std::thread::sleep(std::time::Duration::from_millis(if step == logo_height {
            600
        } else {
            80
        }));
    }

    std::thread::sleep(std::time::Duration::from_millis(400));

    Ok(())
}

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    let mut app = App::new();

    loop {
        terminal.draw(|frame| {
            ui::draw(frame, &app);
            ui::draw_help_bar(frame, &app);
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                return Ok(());
            }

            if handle_key(&mut app, &key) {
                return Ok(());
            }
        }
    }
}

/// Returns `true` when the app should quit.
fn handle_key(app: &mut App, key: &event::KeyEvent) -> bool {
    if app.view == View::Settings {
        return handle_settings_key(app, key.code);
    }
    if app.curl_export_open {
        if key.code == KeyCode::Esc {
            app.curl_export_open = false;
        }
        return false;
    }
    if app.postman_import_open {
        handle_postman_import_key(app, key);
        return false;
    }
    if app.curl_import_open {
        handle_curl_import_key(app, key);
        return false;
    }
    if app.history_open {
        handle_history_key(app, key.code);
        return false;
    }
    if app.method_popup {
        handle_method_popup_key(app, key.code);
        return false;
    }
    if app.editing_sidebar_name {
        handle_sidebar_edit_key(app, key.code);
        return false;
    }
    if app.editing_url {
        handle_url_edit_key(app, key.code);
        return false;
    }
    if app.editing_body {
        handle_body_edit_key(app, key.code);
        return false;
    }
    handle_normal_key(app, key.code)
}

fn handle_settings_key(app: &mut App, key: KeyCode) -> bool {
    match key {
        KeyCode::Esc | KeyCode::Char('s') => app.view = View::Main,
        KeyCode::Char('j') | KeyCode::Down => {
            let max = app.settings_items().len().saturating_sub(1);
            if app.settings_selected < max {
                app.settings_selected += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.settings_selected = app.settings_selected.saturating_sub(1);
        }
        KeyCode::Enter | KeyCode::Char(' ') => app.toggle_setting(),
        KeyCode::Char('q') => return true,
        _ => {}
    }
    false
}

fn handle_history_key(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc | KeyCode::Char('h') => app.history_open = false,
        KeyCode::Char('j') | KeyCode::Down => {
            let max = app.history.len().saturating_sub(1);
            if app.history_selected < max {
                app.history_selected += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.history_selected = app.history_selected.saturating_sub(1);
        }
        KeyCode::Enter => app.load_from_history(),
        _ => {}
    }
}

fn handle_method_popup_key(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => app.method_popup = false,
        KeyCode::Char('j') | KeyCode::Down => {
            let max = Method::all().len().saturating_sub(1);
            if app.method_popup_selected < max {
                app.method_popup_selected += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.method_popup_selected = app.method_popup_selected.saturating_sub(1);
        }
        KeyCode::Enter => app.confirm_method_popup(),
        _ => {}
    }
}

fn handle_postman_import_key(app: &mut App, key: &event::KeyEvent) {
    match key.code {
        KeyCode::Esc => app.postman_import_open = false,
        KeyCode::Enter => app.confirm_postman_import(),
        KeyCode::Backspace => {
            app.postman_import_buffer.pop();
            app.postman_import_error = false;
        }
        KeyCode::Char(c) => {
            app.postman_import_buffer.push(c);
            app.postman_import_error = false;
        }
        _ => {}
    }
}

fn handle_curl_import_key(app: &mut App, key: &event::KeyEvent) {
    match key.code {
        KeyCode::Esc => app.curl_import_open = false,
        KeyCode::Enter => {
            app.curl_import_buffer.push('\n');
            app.curl_import_error = false;
        }
        KeyCode::Backspace => {
            app.curl_import_buffer.pop();
            app.curl_import_error = false;
        }
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.confirm_curl_import();
        }
        KeyCode::Char(c) => {
            app.curl_import_buffer.push(c);
            app.curl_import_error = false;
        }
        _ => {}
    }
}

fn handle_sidebar_edit_key(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => app.editing_sidebar_name = false,
        KeyCode::Enter => app.confirm_editing_name(),
        KeyCode::Backspace => {
            app.sidebar_edit_buffer.pop();
        }
        KeyCode::Char(c) => app.sidebar_edit_buffer.push(c),
        _ => {}
    }
}

fn handle_url_edit_key(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => app.finish_url_edit(),
        KeyCode::Enter => {
            app.finish_url_edit();
            app.send_request();
        }
        KeyCode::Backspace => app.url_backspace(),
        KeyCode::Delete => app.url_delete(),
        KeyCode::Left => app.url_cursor_left(),
        KeyCode::Right => app.url_cursor_right(),
        KeyCode::Home => app.url_cursor_home(),
        KeyCode::End => app.url_cursor_end(),
        KeyCode::Char(c) => app.url_insert(c),
        _ => {}
    }
}

fn handle_body_edit_key(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => app.finish_body_edit(),
        KeyCode::Enter => app.body_insert_newline(),
        KeyCode::Backspace => app.body_backspace(),
        KeyCode::Delete => app.body_delete(),
        KeyCode::Left => app.body_cursor_left(),
        KeyCode::Right => app.body_cursor_right(),
        KeyCode::Up => app.body_cursor_up(),
        KeyCode::Down => app.body_cursor_down(),
        KeyCode::Home => app.body_cursor_home(),
        KeyCode::End => app.body_cursor_end(),
        KeyCode::Tab => app.body_insert_tab(),
        KeyCode::Char(c) => app.body_insert(c),
        _ => {}
    }
}

/// Returns `true` when the app should quit.
fn handle_normal_key(app: &mut App, key: KeyCode) -> bool {
    match app.focus {
        Focus::Sidebar => {
            if app.confirm_delete {
                match key {
                    KeyCode::Char('d') => app.request_delete(),
                    _ => app.cancel_delete(),
                }
                return false;
            }
            match key {
                KeyCode::Char('q') => {
                    app.save_collections();
                    return true;
                }
                KeyCode::Char('s') => app.view = View::Settings,
                KeyCode::Char('h') => app.open_history(),
                KeyCode::Char('r') => app.start_editing_name(),
                KeyCode::Char('m') => app.cycle_sidebar_method(),
                KeyCode::Char('a') => app.create_request(),
                KeyCode::Char('A') => app.create_folder(),
                KeyCode::Char('d') => app.request_delete(),
                KeyCode::Char('D') => app.duplicate_request(),
                KeyCode::Char('i') => app.open_curl_import(),
                KeyCode::Char('I') => app.open_postman_import(),
                KeyCode::Char('c') => app.open_curl_export(),
                KeyCode::Char('j') | KeyCode::Down => {
                    let max = app.sidebar_len().saturating_sub(1);
                    if app.sidebar_selected < max {
                        app.sidebar_selected += 1;
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    app.sidebar_selected = app.sidebar_selected.saturating_sub(1);
                }
                KeyCode::Enter => {
                    let is_folder = matches!(
                        app.selected_sidebar_item(),
                        Some(app::SidebarItem::Folder(_))
                    );
                    app.load_selected();
                    if !is_folder {
                        app.focus = Focus::UrlBar;
                    }
                }
                KeyCode::Tab => app.focus = Focus::UrlBar,
                _ => {}
            }
        }
        Focus::UrlBar => match key {
            KeyCode::Char('q') => return true,
            KeyCode::Char('s') => app.view = View::Settings,
            KeyCode::Char('h') => app.open_history(),
            KeyCode::Char('e' | 'i') => {
                app.editing_url = true;
                app.cursor_pos = app.url.len();
            }
            KeyCode::Char('m') => app.open_method_popup(),
            KeyCode::Enter => app.send_request(),
            KeyCode::Tab => app.focus = Focus::Body,
            KeyCode::BackTab => app.focus = Focus::Sidebar,
            _ => {}
        },
        Focus::Body => match key {
            KeyCode::Char('q') => return true,
            KeyCode::Char('s') => app.view = View::Settings,
            KeyCode::Char('h') => app.open_history(),
            KeyCode::Char('e' | 'i') => app.enter_body_edit(),
            KeyCode::Enter => app.send_request(),
            KeyCode::Tab => app.focus = Focus::Response,
            KeyCode::BackTab => app.focus = Focus::UrlBar,
            _ => {}
        },
        Focus::Response => match key {
            KeyCode::Char('q') => return true,
            KeyCode::Char('s') => app.view = View::Settings,
            KeyCode::Char('h') => app.open_history(),
            KeyCode::Char('j') | KeyCode::Down => {
                app.response_scroll = app.response_scroll.saturating_add(1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                app.response_scroll = app.response_scroll.saturating_sub(1);
            }
            KeyCode::Char('1') => app.response_tab = ResponseTab::Body,
            KeyCode::Char('2') => app.response_tab = ResponseTab::Headers,
            KeyCode::Tab => app.focus = Focus::Sidebar,
            KeyCode::BackTab => app.focus = Focus::Body,
            _ => {}
        },
    }
    false
}
