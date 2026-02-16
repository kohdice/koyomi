use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::tui::model::{Focus, Model};
use crate::tui::theme;

pub fn render(f: &mut Frame, area: Rect, model: &Model) {
    let spans = match model.focus {
        Focus::Calendar => calendar_help_spans(model),
        Focus::EventList => event_list_help_spans(),
    };

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line);
    f.render_widget(paragraph, area);
}

fn help_entry(key: &str, desc: &str) -> Vec<Span<'static>> {
    vec![
        Span::styled(key.to_string(), theme::HELP_KEY),
        Span::styled(format!(" {desc}  ",), theme::HELP_BAR),
    ]
}

fn calendar_help_spans(model: &Model) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    spans.extend(help_entry("h/j/k/l", "move"));
    spans.extend(help_entry("n/p", "month"));
    spans.extend(help_entry("t", "today"));
    spans.extend(help_entry("e", "events"));
    if model.sidebar_visible {
        spans.extend(help_entry("Tab", "focus"));
    }
    spans.extend(help_entry("Enter", "select"));
    spans.extend(help_entry("q", "quit"));

    if let Some(err) = &model.error_message {
        spans.push(Span::styled(format!(" | Error: {err}"), theme::ERROR_STYLE));
    }

    spans
}

fn event_list_help_spans() -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    spans.extend(help_entry("j/k", "navigate"));
    spans.extend(help_entry("Enter", "detail"));
    spans.extend(help_entry("Tab", "calendar"));
    spans.extend(help_entry("e", "hide events"));
    spans.extend(help_entry("q", "quit"));
    spans
}
