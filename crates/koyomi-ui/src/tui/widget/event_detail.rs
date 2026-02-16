use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::tui::calendar_grid::{self, events_for_date};
use crate::tui::model::{Focus, Model};
use crate::tui::theme;

pub fn render(f: &mut Frame, area: Rect, model: &mut Model) {
    let border_style = if model.focus == Focus::EventDetail {
        theme::FOCUSED_BORDER
    } else {
        theme::UNFOCUSED_BORDER
    };

    let events = model
        .events_cache
        .get(&(model.current_year, model.current_month))
        .map(Vec::as_slice)
        .unwrap_or_default();

    let day_events = events_for_date(events, model.selected_date, model.tz);

    let Some(event) = day_events.get(model.event_list_index) else {
        let block =
            Block::default().title(" Detail ").borders(Borders::ALL).border_style(border_style);
        let paragraph = Paragraph::new("No event selected.");
        f.render_widget(block, area);
        f.render_widget(paragraph, area);
        return;
    };

    let title = event.summary.as_deref().unwrap_or("(No title)");
    let block = Block::default()
        .title(format!(" {title} "))
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(area);
    f.render_widget(block, area);

    let lines = build_detail_lines(event);

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });

    let content_height = paragraph.line_count(inner.width);
    let max_scroll = content_height.saturating_sub(inner.height as usize) as u16;
    model.detail_scroll_offset = model.detail_scroll_offset.min(max_scroll);

    let paragraph = paragraph.scroll((model.detail_scroll_offset, 0));
    f.render_widget(paragraph, inner);
}

fn build_detail_lines(event: &koyomi_core::calendar::Event) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();

    if let (Some(_start), Some(end)) = (&event.start, &event.end) {
        let start_str = calendar_grid::format_event_time(event);
        let end_str = end.to_display_string().to_string();
        lines.push(Line::from(Span::styled("Time:", theme::DETAIL_LABEL)));
        lines.push(Line::from(format!("  • {start_str} — {end_str}")));
        lines.push(Line::from(""));
    }

    if let Some(status) = &event.status {
        let style = match status {
            koyomi_core::calendar::EventStatus::Confirmed => theme::STATUS_CONFIRMED,
            koyomi_core::calendar::EventStatus::Tentative => theme::STATUS_TENTATIVE,
            koyomi_core::calendar::EventStatus::Cancelled => theme::STATUS_CANCELLED,
            koyomi_core::calendar::EventStatus::Unknown => theme::STATUS_UNKNOWN,
        };
        lines.push(Line::from(Span::styled("Status:", theme::DETAIL_LABEL)));
        lines.push(Line::from(vec![
            Span::raw("  • "),
            Span::styled(status.as_str().to_string(), style),
        ]));
        lines.push(Line::from(""));
    }

    if let Some(location) = &event.location {
        lines.push(Line::from(Span::styled("Location:", theme::DETAIL_LABEL)));
        lines.push(Line::from(vec![
            Span::raw("  • "),
            Span::styled(location.clone(), theme::EVENT_LOCATION),
        ]));
        lines.push(Line::from(""));
    }

    if let Some(description) = &event.description {
        lines.push(Line::from(Span::styled("Description:", theme::DETAIL_LABEL)));
        for line in description.lines() {
            lines.push(Line::from(format!("  {line}")));
        }
        lines.push(Line::from(""));
    }

    if let Some(organizer) = &event.organizer {
        let name =
            organizer.display_name.as_deref().or(organizer.email.as_deref()).unwrap_or("Unknown");
        lines.push(Line::from(Span::styled("Organizer:", theme::DETAIL_LABEL)));
        lines.push(Line::from(format!("  • {name}")));
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
        use koyomi_core::calendar::EntryPointType;

        lines.push(Line::from(Span::styled("Conference:", theme::DETAIL_LABEL)));

        if let Some(solution) = &conf.conference_solution {
            let video_uri = conf
                .entry_points
                .iter()
                .find(|ep| ep.entry_point_type == EntryPointType::Video)
                .map(|ep| ep.uri.as_str());
            match video_uri {
                Some(uri) => lines.push(Line::from(format!("  • {}: {uri}", solution.name))),
                None => lines.push(Line::from(format!("  • {}", solution.name))),
            }
        }

        for ep in &conf.entry_points {
            if ep.entry_point_type == EntryPointType::Video && conf.conference_solution.is_some() {
                continue;
            }
            let label = match ep.entry_point_type {
                EntryPointType::Video => "video",
                EntryPointType::Phone => "phone",
                EntryPointType::Sip => "sip",
                EntryPointType::More => "more",
                EntryPointType::Unknown => "other",
            };
            lines.push(Line::from(format!("  • {label}: {}", ep.uri)));
        }

        lines.push(Line::from(""));
    }

    if let Some(reminders) = &event.reminders {
        let has_overrides = !reminders.overrides.is_empty();
        if reminders.use_default || has_overrides {
            lines.push(Line::from(Span::styled("Reminders:", theme::DETAIL_LABEL)));
            if reminders.use_default && !has_overrides {
                lines.push(Line::from("  • calendar default"));
            }
            for o in &reminders.overrides {
                lines.push(Line::from(format!("  • {}: {} minutes", o.method.as_str(), o.minutes)));
            }
            lines.push(Line::from(""));
        }
    }

    if let Some(link) = &event.html_link {
        lines.push(Line::from(Span::styled("Link:", theme::DETAIL_LABEL)));
        lines.push(Line::from(format!("  •\u{00A0}{link}")));
    }

    lines
}
