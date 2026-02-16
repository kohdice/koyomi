use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::tui::calendar_grid::{self, events_for_date};
use crate::tui::model::{Focus, Model};
use crate::tui::theme;
use crate::tui::widget::utils::truncate_str;

pub fn render(f: &mut Frame, area: Rect, model: &Model) {
    let border_style = if model.focus == Focus::EventList {
        theme::FOCUSED_BORDER
    } else {
        theme::UNFOCUSED_BORDER
    };

    let header = format!(" Events — {} ", model.selected_date.format("%Y/%m/%d"));

    let block = Block::default().title(header).borders(Borders::ALL).border_style(border_style);

    let inner = block.inner(area);
    f.render_widget(block, area);

    let events = model
        .events_cache
        .get(&(model.current_year, model.current_month))
        .map(Vec::as_slice)
        .unwrap_or_default();

    let day_events = events_for_date(events, model.selected_date, model.tz);

    if day_events.is_empty() {
        let msg = if model.is_current_month_loading() { "Loading..." } else { "No events" };
        let paragraph = Paragraph::new(Line::from(Span::styled(msg, theme::HELP_BAR)));
        f.render_widget(paragraph, inner);
        return;
    }

    let mut lines: Vec<Line> = Vec::new();
    let content_width = inner.width as usize;

    for (i, event) in day_events.iter().enumerate() {
        let indicator = if i == model.event_list_index && model.focus == Focus::EventList {
            Span::styled("▶ ", theme::SELECTED_EVENT_INDICATOR)
        } else {
            Span::styled("● ", theme::EVENT_TIME)
        };

        let time = calendar_grid::format_event_time(event, model.tz);
        let time_span = Span::styled(format!("{time} "), theme::EVENT_TIME);

        let title = event.summary.as_deref().unwrap_or("(No title)");
        let title_span = Span::styled(
            truncate_str(title, content_width.saturating_sub(time.len() + 4)).into_owned(),
            theme::EVENT_TITLE,
        );

        lines.push(Line::from(vec![indicator, time_span, title_span]));

        if let Some(location) = &event.location {
            let loc_display = truncate_str(location, content_width.saturating_sub(4)).into_owned();
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(loc_display, theme::EVENT_LOCATION),
            ]));
        }

        if i < day_events.len() - 1 {
            lines.push(Line::from(""));
        }
    }

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    f.render_widget(paragraph, inner);
}
