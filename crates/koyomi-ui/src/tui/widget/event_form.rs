use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::tui::model::{FormField, FormMode, Model};
use crate::tui::theme;

const LABEL_WIDTH: usize = 14;

pub fn render(f: &mut Frame, model: &Model) {
    let Some(state) = &model.event_form else {
        return;
    };

    let area = f.area();

    let width = 70u16.min(area.width);
    let height = 20u16.min(area.height);
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let form_area = Rect::new(x, y, width, height);

    f.render_widget(Clear, form_area);

    let title = match &state.mode {
        FormMode::Add => " Add Event ",
        FormMode::Edit { .. } => " Edit Event ",
    };

    let block =
        Block::default().title(title).borders(Borders::ALL).border_style(theme::FOCUSED_BORDER);

    let inner = block.inner(form_area);
    f.render_widget(block, form_area);

    let field_width = inner.width as usize;
    let input_width = field_width.saturating_sub(LABEL_WIDTH + 4);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));

    let fields = [
        (FormField::Summary, "Summary:"),
        (FormField::Start, "Start:"),
        (FormField::End, "End:"),
        (FormField::Description, "Description:"),
        (FormField::Location, "Location:"),
        (FormField::Status, "Status:"),
        (FormField::Attendees, "Attendees:"),
        (FormField::Reminders, "Reminders:"),
    ];

    for (field, label) in &fields {
        let is_focused = state.focused_field == *field;
        let input = &state.fields[field_index(field)];
        let content = input.content();

        let label_style = if is_focused { theme::DETAIL_LABEL } else { theme::HELP_BAR };

        let is_selector = *field == FormField::Reminders;

        let display_content = if is_selector {
            content.to_string()
        } else if content.len() > input_width {
            let cursor = input.cursor();
            let start = cursor.saturating_sub(input_width);
            let end = (start + input_width).min(content.chars().count());
            content.chars().skip(start).take(end - start).collect::<String>()
        } else {
            content.to_string()
        };

        let bracket_style = if is_focused { theme::FOCUSED_BORDER } else { Style::default() };

        let mut spans = vec![
            Span::styled(format!("  {label:<width$}", width = LABEL_WIDTH), label_style),
            Span::styled("[", bracket_style),
            Span::raw(display_content),
            Span::styled("]", bracket_style),
        ];

        if is_selector && is_focused {
            spans.push(Span::styled(" ▲▼", theme::HELP_KEY));
        }

        lines.push(Line::from(spans));
    }

    if let Some(err) = &state.validation_error {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(format!("  {err}"), theme::ERROR_STYLE)));
    }

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, inner);

    if let Some(form) = &model.event_form {
        let is_selector = form.focused_field == FormField::Reminders;

        if !is_selector {
            let field_idx = field_index(&form.focused_field);
            let input = &form.fields[field_idx];
            let cursor_char = input.cursor();

            let input_width = inner.width as usize - LABEL_WIDTH - 4;
            let scroll_start = cursor_char.saturating_sub(input_width);
            let cursor_in_view = cursor_char - scroll_start;

            // +2 for "  " indent, +LABEL_WIDTH for label, +1 for "["
            let cursor_x = inner.x + 2 + LABEL_WIDTH as u16 + 1 + cursor_in_view as u16;
            // +1 for empty line, field_idx for the field row
            let cursor_y = inner.y + 1 + field_idx as u16;

            if cursor_x < inner.x + inner.width && cursor_y < inner.y + inner.height {
                f.set_cursor_position((cursor_x, cursor_y));
            }
        }

        let help_y = form_area.y + form_area.height - 1;
        if help_y > inner.y {
            let help_area =
                Rect::new(form_area.x + 1, help_y, form_area.width.saturating_sub(2), 1);
            let mut spans = Vec::new();
            use crate::tui::widget::utils::help_entry;
            spans.extend(help_entry("Tab", "next"));
            spans.extend(help_entry("S-Tab", "prev"));
            if is_selector {
                spans.extend(help_entry("↑↓", "select"));
            }
            spans.extend(help_entry("Enter", "submit"));
            spans.extend(help_entry("Esc", "cancel"));

            f.render_widget(Paragraph::new(Line::from(spans)), help_area);
        }
    }
}

fn field_index(field: &FormField) -> usize {
    match field {
        FormField::Summary => 0,
        FormField::Start => 1,
        FormField::End => 2,
        FormField::Description => 3,
        FormField::Location => 4,
        FormField::Status => 5,
        FormField::Attendees => 6,
        FormField::Reminders => 7,
    }
}
