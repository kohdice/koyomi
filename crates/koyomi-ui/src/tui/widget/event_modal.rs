use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};

use crate::tui::model::{Focus, Model};
use crate::tui::theme;
use crate::tui::widget;
use crate::tui::widget::utils::help_entry;

const HORIZONTAL_LAYOUT_MIN_WIDTH: u16 = 120;

pub fn render(f: &mut Frame, model: &Model) {
    let area = f.area();

    let modal_width = (area.width as f32 * 0.8).max(40.0).min(area.width as f32) as u16;
    let modal_height = (area.height as f32 * 0.8).max(10.0).min(area.height as f32) as u16;

    let x = (area.width.saturating_sub(modal_width)) / 2;
    let y = (area.height.saturating_sub(modal_height)) / 2;
    let modal_area = Rect::new(x, y, modal_width, modal_height);

    f.render_widget(Clear, modal_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(modal_area);

    let content_area = chunks[0];
    let help_area = chunks[1];

    let (list_area, detail_area) = if content_area.width >= HORIZONTAL_LAYOUT_MIN_WIDTH {
        let panes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(content_area);
        (panes[0], panes[1])
    } else {
        let panes = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(content_area);
        (panes[0], panes[1])
    };

    widget::event_list::render(f, list_area, model);
    widget::event_detail::render(f, detail_area, model);

    render_help_bar(f, help_area, model);
}

fn render_help_bar(f: &mut Frame, area: Rect, model: &Model) {
    let mut spans = Vec::new();

    let focus_label = match model.focus {
        Focus::EventList => "List",
        Focus::EventDetail => "Detail",
        Focus::Calendar => "",
    };

    if !focus_label.is_empty() {
        spans.push(Span::styled(format!("[{focus_label}] "), theme::HELP_KEY));
    }

    spans.extend(help_entry("Tab", "switch pane"));
    spans.extend(help_entry("j/k", "navigate"));
    spans.extend(help_entry("d", "delete"));
    spans.extend(help_entry("e", "edit"));

    if model.focus == Focus::EventDetail {
        spans.extend(help_entry("g/G", "top/bottom"));
    }

    spans.extend(help_entry("Esc", "close"));

    let line = Line::from(spans);
    f.render_widget(Paragraph::new(line), area);
}
