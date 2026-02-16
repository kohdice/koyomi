use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::tui::model::Model;
use crate::tui::theme;
use crate::tui::widget::utils::help_entry;

pub fn render(f: &mut Frame, area: Rect, model: &Model) {
    let mut spans = Vec::new();
    spans.extend(help_entry("h/j/k/l", "move"));
    spans.extend(help_entry("n/p", "month"));
    spans.extend(help_entry("t", "today"));
    spans.extend(help_entry("Enter", "events"));
    spans.extend(help_entry("q", "quit"));

    if let Some(err) = &model.error_message {
        spans.push(Span::styled(format!(" | Error: {err}"), theme::ERROR_STYLE));
    }

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line);
    f.render_widget(paragraph, area);
}
