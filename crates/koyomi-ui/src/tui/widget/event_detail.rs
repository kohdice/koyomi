use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::tui::calendar_grid::{self, events_for_date};
use crate::tui::model::Model;
use crate::tui::theme;

pub fn render(f: &mut Frame, model: &Model) {
    let area = f.area();

    let modal_width = (area.width as f32 * 0.6).max(40.0).min(area.width as f32) as u16;
    let modal_height = (area.height as f32 * 0.7).max(10.0).min(area.height as f32) as u16;

    let x = (area.width.saturating_sub(modal_width)) / 2;
    let y = (area.height.saturating_sub(modal_height)) / 2;
    let modal_area = Rect::new(x, y, modal_width, modal_height);

    f.render_widget(Clear, modal_area);

    let events = model
        .events_cache
        .get(&(model.current_year, model.current_month))
        .map(Vec::as_slice)
        .unwrap_or_default();

    let day_events = events_for_date(events, model.selected_date);

    let Some(event) = day_events.get(model.event_list_index) else {
        let block = Block::default()
            .title(" Event Detail ")
            .borders(Borders::ALL)
            .border_style(theme::MODAL_BORDER);
        let paragraph = Paragraph::new("No event selected.");
        f.render_widget(block, modal_area);
        f.render_widget(paragraph, modal_area);
        return;
    };

    let title = event.summary.as_deref().unwrap_or("(No title)");
    let block = Block::default()
        .title(format!(" {title} "))
        .borders(Borders::ALL)
        .border_style(theme::MODAL_BORDER);

    let inner = block.inner(modal_area);
    f.render_widget(block, modal_area);

    let lines = build_detail_lines(event, inner.width as usize);

    let paragraph =
        Paragraph::new(lines).wrap(Wrap { trim: false }).scroll((model.detail_scroll_offset, 0));

    f.render_widget(paragraph, inner);
}

fn build_detail_lines(event: &koyomi_core::calendar::Event, _width: usize) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();

    if let (Some(_start), Some(end)) = (&event.start, &event.end) {
        let start_str = calendar_grid::format_event_time(event);
        let end_str = end.to_display_string().to_string();
        lines.push(Line::from(vec![
            Span::styled("Time: ", theme::DETAIL_LABEL),
            Span::raw(format!("{start_str} — {end_str}")),
        ]));
        lines.push(Line::from(""));
    }

    if let Some(location) = &event.location {
        lines.push(Line::from(vec![
            Span::styled("Location: ", theme::DETAIL_LABEL),
            Span::styled(location.clone(), theme::EVENT_LOCATION),
        ]));
        lines.push(Line::from(""));
    }

    if let Some(description) = &event.description {
        lines.push(Line::from(Span::styled("Description:", theme::DETAIL_LABEL)));
        for line in description.lines() {
            lines.push(Line::from(line.to_string()));
        }
        lines.push(Line::from(""));
    }

    if let Some(organizer) = &event.organizer {
        let name =
            organizer.display_name.as_deref().or(organizer.email.as_deref()).unwrap_or("Unknown");
        lines.push(Line::from(vec![
            Span::styled("Organizer: ", theme::DETAIL_LABEL),
            Span::raw(name.to_string()),
        ]));
        lines.push(Line::from(""));
    }

    let human_attendees: Vec<_> = event.attendees.iter().filter(|a| !a.resource).collect();
    if !human_attendees.is_empty() {
        lines.push(Line::from(Span::styled("Attendees:", theme::DETAIL_LABEL)));
        for attendee in &human_attendees {
            let name =
                attendee.display_name.as_deref().or(attendee.email.as_deref()).unwrap_or("Unknown");
            let status =
                attendee.response_status.as_ref().map(|s| format!(" ({s:?})")).unwrap_or_default();
            lines.push(Line::from(format!("  • {name}{status}")));
        }
        lines.push(Line::from(""));
    }

    if let Some(conf) = &event.conference_data {
        if let Some(solution) = &conf.conference_solution {
            lines.push(Line::from(vec![
                Span::styled("Conference: ", theme::DETAIL_LABEL),
                Span::raw(solution.name.clone()),
            ]));
        }
        for ep in &conf.entry_points {
            lines.push(Line::from(format!("  {}", ep.uri)));
        }
        lines.push(Line::from(""));
    }

    if let Some(link) = &event.html_link {
        lines.push(Line::from(vec![
            Span::styled("Link: ", theme::DETAIL_LABEL),
            Span::raw(link.clone()),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "j/k scroll | g/G top/bottom | Enter/Esc close",
        theme::HELP_BAR,
    )));

    lines
}
