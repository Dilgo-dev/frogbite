mod app;
mod assertions;
mod collections;
mod cookies;
mod curl;
mod environments;
mod history;
mod postman;
mod run_cmd;
mod settings;
mod ui;
mod update;

use std::io;
use std::process::ExitCode;

use clap::Parser;
use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{prelude::*, widgets::Paragraph};

use app::{App, Focus, Method, RequestTab, ResponseTab, View};

#[derive(Debug, Parser)]
#[command(name = "frogbite", version, about = "Terminal API tester")]
struct Cli {
    #[command(subcommand)]
    command: Option<run_cmd::Command>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    if let Some(cmd) = cli.command {
        return run_cmd::execute(&cmd);
    }
    match run_tui() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}

fn run_tui() -> io::Result<()> {
    let s = settings::load();

    let update_rx = if s.update_check {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            if let Ok(latest) = update::fetch_latest_version() {
                if update::is_newer(&latest, env!("CARGO_PKG_VERSION")) {
                    let _ = tx.send(latest);
                }
            }
        });
        Some(rx)
    } else {
        None
    };

    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    if s.splash_animation {
        show_splash(&mut terminal)?;
    }

    let result = run(&mut terminal, update_rx);

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

fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    update_rx: Option<std::sync::mpsc::Receiver<String>>,
) -> io::Result<()> {
    let mut app = App::new();
    if let Some(rx) = update_rx {
        app.set_update_check_rx(rx);
    }

    loop {
        terminal.draw(|frame| {
            ui::draw(frame, &app);
            ui::draw_help_bar(frame, &app);
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
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

        app.poll_pending();
        app.poll_update_check();
    }
}

/// Returns `true` when the app should quit.
fn handle_key(app: &mut App, key: &event::KeyEvent) -> bool {
    app.response.clipboard_msg = None;
    if app.sidebar.confirm_delete {
        match key.code {
            KeyCode::Char('y') => app.request_delete(),
            _ => app.cancel_delete(),
        }
        return false;
    }
    if app.ui.view == View::Settings {
        return handle_settings_key(app, key.code);
    }
    if try_dispatch_modal(app, key) {
        return false;
    }
    if try_dispatch_editing(app, key) {
        return false;
    }
    handle_normal_key(app, key.code)
}

/// Returns `true` if a modal popup consumed the key.
fn try_dispatch_modal(app: &mut App, key: &event::KeyEvent) -> bool {
    if app.response.searching {
        handle_search_key(app, key.code);
    } else if app.timeout.popup_open {
        handle_timeout_popup_key(app, key.code);
    } else if app.tls.popup_open {
        handle_tls_popup_key(app, key.code);
    } else if app.cookies.popup_open {
        handle_cookies_popup_key(app, key.code);
    } else if app.extractors.popup_open {
        handle_extractors_popup_key(app, key.code);
    } else if app.assertions.popup_open {
        handle_assertions_popup_key(app, key.code);
    } else if app.env.popup_open {
        handle_env_popup_key(app, key.code);
    } else if app.curl_io.export_open {
        if key.code == KeyCode::Esc {
            app.curl_io.export_open = false;
        }
    } else if app.postman_io.open {
        handle_postman_import_key(app, key);
    } else if app.curl_io.import_open {
        handle_curl_import_key(app, key);
    } else if app.history.open {
        handle_history_key(app, key.code);
    } else if app.method_popup.open {
        handle_method_popup_key(app, key.code);
    } else {
        return false;
    }
    true
}

/// Returns `true` if an inline editing mode consumed the key.
fn try_dispatch_editing(app: &mut App, key: &event::KeyEvent) -> bool {
    if app.request.form_editor.editing {
        handle_kv_edit_key(&mut app.request.form_editor, key.code);
        if !app.request.form_editor.editing {
            app.sync_to_collection();
        }
    } else if app.request.header_editor.editing {
        handle_kv_edit_key(&mut app.request.header_editor, key.code);
        if !app.request.header_editor.editing {
            app.sync_to_collection();
        }
    } else if app.request.param_editor.editing {
        handle_kv_edit_key(&mut app.request.param_editor, key.code);
        if !app.request.param_editor.editing {
            app.sync_params_to_url();
        }
    } else if app.auth.editing {
        handle_auth_edit_key(app, key.code);
    } else if app.auth.selecting_type {
        handle_auth_type_select_key(app, key.code);
    } else if app.env.editor.editing_var {
        handle_env_var_edit_key(app, key.code);
    } else if app.env.editor.open {
        handle_env_editor_key(app, key.code);
    } else if app.env.renaming {
        handle_env_rename_key(app, key.code);
    } else if app.env.import.open {
        handle_env_import_key(app, key.code);
    } else if app.sidebar.editing_name {
        handle_sidebar_edit_key(app, key.code);
    } else if app.request.editing_url {
        handle_url_edit_key(app, key.code);
    } else if app.request.editing_body {
        handle_body_edit_key(app, key.code);
    } else {
        return false;
    }
    true
}

fn handle_settings_key(app: &mut App, key: KeyCode) -> bool {
    match key {
        KeyCode::Esc | KeyCode::Char('s') => app.ui.view = View::Main,
        KeyCode::Char('j') | KeyCode::Down => {
            let max = app.settings_items().len().saturating_sub(1);
            if app.ui.settings_selected < max {
                app.ui.settings_selected += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.ui.settings_selected = app.ui.settings_selected.saturating_sub(1);
        }
        KeyCode::Enter | KeyCode::Char(' ') => app.toggle_setting(),
        KeyCode::Char('q') => return true,
        _ => {}
    }
    false
}

fn handle_history_key(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc | KeyCode::Char('h') => app.history.open = false,
        KeyCode::Char('j') | KeyCode::Down => {
            let max = app.history.entries.len().saturating_sub(1);
            if app.history.selected < max {
                app.history.selected += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.history.selected = app.history.selected.saturating_sub(1);
        }
        KeyCode::Enter => app.load_from_history(),
        _ => {}
    }
}

fn handle_method_popup_key(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => app.method_popup.open = false,
        KeyCode::Char('j') | KeyCode::Down => {
            let max = Method::all().len().saturating_sub(1);
            if app.method_popup.selected < max {
                app.method_popup.selected += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.method_popup.selected = app.method_popup.selected.saturating_sub(1);
        }
        KeyCode::Enter => app.confirm_method_popup(),
        _ => {}
    }
}

fn handle_postman_import_key(app: &mut App, key: &event::KeyEvent) {
    match key.code {
        KeyCode::Esc => app.postman_io.open = false,
        KeyCode::Enter => app.confirm_postman_import(),
        KeyCode::Backspace => {
            app.postman_io.buffer.pop();
            app.postman_io.error = false;
        }
        KeyCode::Char(c) => {
            app.postman_io.buffer.push(c);
            app.postman_io.error = false;
        }
        _ => {}
    }
}

fn handle_curl_import_key(app: &mut App, key: &event::KeyEvent) {
    match key.code {
        KeyCode::Esc => app.curl_io.import_open = false,
        KeyCode::Enter => {
            app.curl_io.import_buffer.push('\n');
            app.curl_io.import_error = false;
        }
        KeyCode::Backspace => {
            app.curl_io.import_buffer.pop();
            app.curl_io.import_error = false;
        }
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.confirm_curl_import();
        }
        KeyCode::Char(c) => {
            app.curl_io.import_buffer.push(c);
            app.curl_io.import_error = false;
        }
        _ => {}
    }
}

fn handle_sidebar_edit_key(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => app.sidebar.editing_name = false,
        KeyCode::Enter => app.confirm_editing_name(),
        KeyCode::Backspace => {
            app.sidebar.edit_buffer.pop();
        }
        KeyCode::Char(c) => app.sidebar.edit_buffer.push(c),
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

fn handle_search_key(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => app.cancel_search(),
        KeyCode::Enter => app.confirm_search(),
        KeyCode::Backspace => {
            app.response.search_buf.pop();
        }
        KeyCode::Char(c) => app.response.search_buf.push(c),
        _ => {}
    }
}

fn handle_kv_edit_key(editor: &mut app::KvEditorState, key: KeyCode) {
    match key {
        KeyCode::Esc => editor.cancel_edit(),
        KeyCode::Tab => editor.edit_field = 1 - editor.edit_field,
        KeyCode::Enter => editor.confirm_edit(),
        KeyCode::Backspace => {
            if editor.edit_field == 0 {
                editor.edit_key_buf.pop();
            } else {
                editor.edit_value_buf.pop();
            }
        }
        KeyCode::Char(c) => {
            if editor.edit_field == 0 {
                editor.edit_key_buf.push(c);
            } else {
                editor.edit_value_buf.push(c);
            }
        }
        _ => {}
    }
}

fn handle_auth_type_select_key(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => app.auth.selecting_type = false,
        KeyCode::Char('j') | KeyCode::Down => {
            if app.auth.type_selected < App::AUTH_TYPES.len() - 1 {
                app.auth.type_selected += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.auth.type_selected = app.auth.type_selected.saturating_sub(1);
        }
        KeyCode::Enter => app.select_auth_type(),
        _ => {}
    }
}

fn handle_auth_edit_key(app: &mut App, key: KeyCode) {
    let has_two_fields = !matches!(&app.auth.config, collections::Auth::Bearer { .. });
    match key {
        KeyCode::Esc => app.auth.editing = false,
        KeyCode::Tab if has_two_fields => app.auth.field = 1 - app.auth.field,
        KeyCode::Enter => app.confirm_auth_edit(),
        KeyCode::Backspace => {
            if app.auth.field == 0 {
                app.auth.buf_a.pop();
            } else {
                app.auth.buf_b.pop();
            }
        }
        KeyCode::Char(c) => {
            if app.auth.field == 0 {
                app.auth.buf_a.push(c);
            } else {
                app.auth.buf_b.push(c);
            }
        }
        _ => {}
    }
}

fn handle_env_popup_key(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => app.env.popup_open = false,
        KeyCode::Char('j') | KeyCode::Down => {
            let max = app.env_popup_count().saturating_sub(1);
            if app.env.popup_selected < max {
                app.env.popup_selected += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.env.popup_selected = app.env.popup_selected.saturating_sub(1);
        }
        KeyCode::Enter => app.select_env_from_popup(),
        KeyCode::Char('a') => app.create_environment(),
        KeyCode::Char('d') => app.delete_env_from_popup(),
        KeyCode::Char('r') => app.start_env_rename(),
        KeyCode::Char('e') => app.open_env_editor(),
        KeyCode::Char('i') => app.open_env_import(),
        _ => {}
    }
}

fn handle_env_rename_key(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => app.cancel_env_rename(),
        KeyCode::Enter => app.confirm_env_rename(),
        KeyCode::Backspace => {
            app.env.name_buffer.pop();
        }
        KeyCode::Char(c) => app.env.name_buffer.push(c),
        _ => {}
    }
}

fn handle_env_editor_key(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => {
            app.env.editor.open = false;
            app.env.popup_open = true;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            let max = app.env_editor_count().saturating_sub(1);
            if app.env.editor.selected < max {
                app.env.editor.selected += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.env.editor.selected = app.env.editor.selected.saturating_sub(1);
        }
        KeyCode::Char('a') | KeyCode::Enter => app.start_edit_var(),
        KeyCode::Char('d') => app.delete_var(),
        KeyCode::Char('s') => app.toggle_var_secret(),
        _ => {}
    }
}

fn handle_env_var_edit_key(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => app.env.editor.editing_var = false,
        KeyCode::Tab => app.env.editor.var_field = 1 - app.env.editor.var_field,
        KeyCode::Enter => app.confirm_var_edit(),
        KeyCode::Backspace => {
            if app.env.editor.var_field == 0 {
                app.env.editor.var_key_buffer.pop();
            } else {
                app.env.editor.var_value_buffer.pop();
            }
        }
        KeyCode::Char(c) => {
            if app.env.editor.var_field == 0 {
                app.env.editor.var_key_buffer.push(c);
            } else {
                app.env.editor.var_value_buffer.push(c);
            }
        }
        _ => {}
    }
}

fn handle_assertions_popup_key(app: &mut App, key: KeyCode) {
    if app.assertions.editing {
        match key {
            KeyCode::Esc => app.assertions.editing = false,
            KeyCode::Enter => app.assertions_confirm_edit(),
            KeyCode::Backspace => {
                app.assertions.edit_buffer.pop();
            }
            KeyCode::Char(c) => app.assertions.edit_buffer.push(c),
            _ => {}
        }
        return;
    }
    match key {
        KeyCode::Esc | KeyCode::Char('q') => app.assertions.popup_open = false,
        KeyCode::Char('j') | KeyCode::Down => app.assertions_popup_down(),
        KeyCode::Char('k') | KeyCode::Up => app.assertions_popup_up(),
        KeyCode::Char('a') => app.assertions_start_add(),
        KeyCode::Char('e') | KeyCode::Enter => app.assertions_start_edit(),
        KeyCode::Char('d') => app.assertions_delete(),
        _ => {}
    }
}

fn handle_extractors_popup_key(app: &mut App, key: KeyCode) {
    if app.extractors.editor.editing {
        handle_kv_edit_key(&mut app.extractors.editor, key);
        if !app.extractors.editor.editing {
            app.sync_to_collection();
        }
        return;
    }
    match key {
        KeyCode::Esc | KeyCode::Char('q') => app.extractors.popup_open = false,
        KeyCode::Char('j') | KeyCode::Down => app.extractors.editor.move_down(),
        KeyCode::Char('k') | KeyCode::Up => app.extractors.editor.move_up(),
        KeyCode::Char('a') => app.extractors.editor.start_add(),
        KeyCode::Char('e') | KeyCode::Enter => app.extractors.editor.start_edit(),
        KeyCode::Char('d') => {
            app.extractors.editor.delete_selected();
            app.sync_to_collection();
        }
        _ => {}
    }
}

fn handle_cookies_popup_key(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc | KeyCode::Char('q') => app.cookies.popup_open = false,
        KeyCode::Char('j') | KeyCode::Down => app.cookies_popup_down(),
        KeyCode::Char('k') | KeyCode::Up => app.cookies_popup_up(),
        KeyCode::Char('d') => app.cookies_popup_delete(),
        KeyCode::Char('D') => app.cookies_popup_clear_all(),
        _ => {}
    }
}

fn handle_tls_popup_key(app: &mut App, key: KeyCode) {
    if app.tls.editing {
        match key {
            KeyCode::Esc => {
                app.tls.editing = false;
                app.tls.edit_buffer.clear();
            }
            KeyCode::Enter => app.tls_confirm_edit(),
            KeyCode::Backspace => {
                app.tls.edit_buffer.pop();
            }
            KeyCode::Char(c) => app.tls.edit_buffer.push(c),
            _ => {}
        }
        return;
    }
    match key {
        KeyCode::Esc => app.tls.popup_open = false,
        KeyCode::Char('j') | KeyCode::Down => app.tls_popup_down(),
        KeyCode::Char('k') | KeyCode::Up => app.tls_popup_up(),
        KeyCode::Enter | KeyCode::Char(' ' | 'e') => app.tls_popup_activate(),
        KeyCode::Char('d') => app.tls_popup_clear_field(),
        _ => {}
    }
}

fn handle_timeout_popup_key(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => app.timeout.popup_open = false,
        KeyCode::Enter => app.confirm_timeout_popup(),
        KeyCode::Backspace => {
            app.timeout.buffer.pop();
            app.timeout.error = false;
        }
        KeyCode::Char(c) if c.is_ascii_digit() => {
            app.timeout.buffer.push(c);
            app.timeout.error = false;
        }
        _ => {}
    }
}

fn handle_env_import_key(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => app.env.import.open = false,
        KeyCode::Enter => app.confirm_env_import(),
        KeyCode::Backspace => {
            app.env.import.buffer.pop();
            app.env.import.error = false;
        }
        KeyCode::Char(c) => {
            app.env.import.buffer.push(c);
            app.env.import.error = false;
        }
        _ => {}
    }
}

/// Returns `true` when the app should quit.
fn handle_sidebar_key(app: &mut App, key: KeyCode) -> bool {
    match key {
        KeyCode::Char('q') => {
            app.save_collections();
            return true;
        }
        KeyCode::Char('s') => app.ui.view = View::Settings,
        KeyCode::Char('h') => app.open_history(),
        KeyCode::Char('E') => app.open_env_popup(),
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
            if app.sidebar.selected < max {
                app.sidebar.selected += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.sidebar.selected = app.sidebar.selected.saturating_sub(1);
        }
        KeyCode::Enter => {
            let is_folder = matches!(
                app.selected_sidebar_item(),
                Some(app::SidebarItem::Folder(_))
            );
            app.load_selected();
            if !is_folder {
                app.ui.focus = Focus::UrlBar;
            }
        }
        KeyCode::Tab => app.ui.focus = Focus::UrlBar,
        _ => {}
    }
    false
}

fn handle_request_panel_key(app: &mut App, key: KeyCode) -> bool {
    match key {
        KeyCode::Char('q') => return true,
        KeyCode::Char('s') => app.ui.view = View::Settings,
        KeyCode::Char('h') => app.open_history(),
        KeyCode::Char('E') => app.open_env_popup(),
        KeyCode::Char('1') => app.request.tab = RequestTab::Body,
        KeyCode::Char('2') => app.request.tab = RequestTab::Headers,
        KeyCode::Char('3' | 'A') => app.request.tab = RequestTab::Auth,
        KeyCode::Char('4') => app.request.tab = RequestTab::Params,
        KeyCode::Char('b') if app.request.tab == RequestTab::Body => {
            app.request.body_type = app.request.body_type.next();
            app.sync_to_collection();
        }
        KeyCode::Char('c')
            if app.request.tab == RequestTab::Body
                && app.request.body_type == collections::BodyType::Raw =>
        {
            app.request.content_type = app.request.content_type.next();
            app.sync_to_collection();
        }
        KeyCode::Char('e' | 'i') => match app.request.tab {
            RequestTab::Body if app.request.body_type == collections::BodyType::Raw => {
                app.enter_body_edit();
            }
            RequestTab::Body => app.request.form_editor.start_edit(),
            RequestTab::Headers => app.request.header_editor.start_edit(),
            RequestTab::Params => app.request.param_editor.start_edit(),
            RequestTab::Auth => {
                if app.auth.config == collections::Auth::None {
                    app.auth.type_selected = app.auth_type_index();
                    app.auth.selecting_type = true;
                } else {
                    app.open_auth_edit();
                }
            }
        },
        KeyCode::Char('t') if app.request.tab == RequestTab::Auth => {
            app.auth.type_selected = app.auth_type_index();
            app.auth.selecting_type = true;
        }
        KeyCode::Char('a') => match app.request.tab {
            RequestTab::Body if app.request.body_type != collections::BodyType::Raw => {
                app.request.form_editor.start_add();
            }
            RequestTab::Headers => app.request.header_editor.start_add(),
            RequestTab::Params => app.request.param_editor.start_add(),
            RequestTab::Body | RequestTab::Auth => {}
        },
        KeyCode::Char('d') => match app.request.tab {
            RequestTab::Body if app.request.body_type != collections::BodyType::Raw => {
                app.request.form_editor.delete_selected();
                app.sync_to_collection();
            }
            RequestTab::Headers => {
                app.request.header_editor.delete_selected();
                app.sync_to_collection();
            }
            RequestTab::Params => {
                app.request.param_editor.delete_selected();
                app.sync_params_to_url();
            }
            RequestTab::Body | RequestTab::Auth => {}
        },
        KeyCode::Char('j') | KeyCode::Down => match app.request.tab {
            RequestTab::Body if app.request.body_type != collections::BodyType::Raw => {
                app.request.form_editor.move_down();
            }
            RequestTab::Headers => app.request.header_editor.move_down(),
            RequestTab::Params => app.request.param_editor.move_down(),
            RequestTab::Body | RequestTab::Auth => {}
        },
        KeyCode::Char('k') | KeyCode::Up => match app.request.tab {
            RequestTab::Body if app.request.body_type != collections::BodyType::Raw => {
                app.request.form_editor.move_up();
            }
            RequestTab::Headers => app.request.header_editor.move_up(),
            RequestTab::Params => app.request.param_editor.move_up(),
            RequestTab::Body | RequestTab::Auth => {}
        },
        KeyCode::Enter => app.send_request(),
        KeyCode::Tab => app.ui.focus = Focus::Response,
        KeyCode::BackTab => app.ui.focus = Focus::UrlBar,
        _ => {}
    }
    false
}

/// Returns `true` when the app should quit.
fn handle_normal_key(app: &mut App, key: KeyCode) -> bool {
    match app.ui.focus {
        Focus::Sidebar => return handle_sidebar_key(app, key),
        Focus::UrlBar => match key {
            KeyCode::Char('q') => return true,
            KeyCode::Char('s') => app.ui.view = View::Settings,
            KeyCode::Char('h') => app.open_history(),
            KeyCode::Char('E') => app.open_env_popup(),
            KeyCode::Char('A') => {
                app.request.tab = RequestTab::Auth;
                app.ui.focus = Focus::Body;
            }
            KeyCode::Char('e' | 'i') => {
                app.request.editing_url = true;
                app.request.cursor_pos = app.request.url.len();
            }
            KeyCode::Char('m') => app.open_method_popup(),
            KeyCode::Char('R') => app.toggle_follow_redirects(),
            KeyCode::Char('T') => app.open_timeout_popup(),
            KeyCode::Char('S') => app.open_tls_popup(),
            KeyCode::Char('C') => app.open_cookies_popup(),
            KeyCode::Char('X') => app.open_extractors_popup(),
            KeyCode::Char('V') => app.open_assertions_popup(),
            KeyCode::Enter => app.send_request(),
            KeyCode::Tab => app.ui.focus = Focus::Body,
            KeyCode::BackTab => app.ui.focus = Focus::Sidebar,
            _ => {}
        },
        Focus::Body => return handle_request_panel_key(app, key),
        Focus::Response => match key {
            KeyCode::Char('q') => return true,
            KeyCode::Char('s') => app.ui.view = View::Settings,
            KeyCode::Char('h') => app.open_history(),
            KeyCode::Char('E') => app.open_env_popup(),
            KeyCode::Char('A') => {
                app.request.tab = RequestTab::Auth;
                app.ui.focus = Focus::Body;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                app.response.scroll = app.response.scroll.saturating_add(1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                app.response.scroll = app.response.scroll.saturating_sub(1);
            }
            KeyCode::Char('1') => app.response.tab = ResponseTab::Body,
            KeyCode::Char('2') => app.response.tab = ResponseTab::Headers,
            KeyCode::Char('/') => app.open_search(),
            KeyCode::Char('n') => app.next_match(),
            KeyCode::Char('N') => app.prev_match(),
            KeyCode::Char('y') => app.copy_response_to_clipboard(),
            KeyCode::Esc => app.clear_search(),
            KeyCode::Tab => app.ui.focus = Focus::Sidebar,
            KeyCode::BackTab => app.ui.focus = Focus::Body,
            _ => {}
        },
    }
    false
}
