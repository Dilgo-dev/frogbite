use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use super::*;
use crate::app::App;

pub(super) fn draw_settings(frame: &mut Frame, app: &App) {
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
