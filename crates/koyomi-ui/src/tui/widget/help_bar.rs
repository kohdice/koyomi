use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::tui::model::Model;
use crate::tui::theme;

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

fn help_entry(key: &str, desc: &str) -> Vec<Span<'static>> {
    vec![
        Span::styled(key.to_string(), theme::HELP_KEY),
        Span::styled(format!(" {desc}  "), theme::HELP_BAR),
    ]
}
