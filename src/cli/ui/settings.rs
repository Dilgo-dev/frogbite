use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use super::*;
use crate::app::{App, SettingDisplay};

pub(super) fn draw_settings(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let block = Block::default()
        .title(" Settings ")
        .title_style(Style::default().fg(green()).bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(green()))
        .bg(bg());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let items = app.settings_items();

    for (i, (label, value)) in items.iter().enumerate() {
        let y = inner.y + i as u16;
        if y >= inner.y + inner.height {
            break;
        }

        let selected = i == app.ui.settings_selected;
        let row_area = Rect::new(inner.x, y, inner.width, 1);

        if selected {
            frame.render_widget(Paragraph::new("").bg(surface()), row_area);
        }

        let indicator = if selected { "> " } else { "  " };
        let (display, display_color) = match value {
            SettingDisplay::Toggle(true) => ("[on] ".to_owned(), green()),
            SettingDisplay::Toggle(false) => ("[off]".to_owned(), muted()),
            SettingDisplay::Choice(v) => (format!("< {v} >"), teal()),
        };

        let line = Line::from(vec![
            Span::styled(indicator, Style::default().fg(green())),
            Span::styled(display, Style::default().fg(display_color).bold()),
            Span::raw("  "),
            Span::styled(
                *label,
                if selected {
                    Style::default().fg(fg())
                } else {
                    Style::default().fg(muted())
                },
            ),
        ]);

        frame.render_widget(Paragraph::new(line), row_area);
    }
}
