use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::tui::model::Model;
use crate::tui::theme;

pub fn render(f: &mut Frame, model: &Model) {
    let Some(state) = &model.delete_confirm else {
        return;
    };

    let area = f.area();

    let width = 42u16.min(area.width);
    let height = 7u16.min(area.height);
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let dialog_area = Rect::new(x, y, width, height);

    f.render_widget(Clear, dialog_area);

    let block = Block::default()
        .title(" Confirm Delete ")
        .borders(Borders::ALL)
        .border_style(theme::FOCUSED_BORDER);

    let inner = block.inner(dialog_area);
    f.render_widget(block, dialog_area);

    let summary = &state.event_summary;
    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  Delete '"),
            Span::styled(summary.clone(), theme::EVENT_TITLE),
            Span::raw("'?"),
        ]),
        Line::from(""),
        Line::from(Span::styled("  Press [y] to confirm, [n] to cancel", theme::HELP_BAR)),
    ];

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, inner);
}
