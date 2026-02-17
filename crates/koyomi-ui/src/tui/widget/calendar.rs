use chrono::{Datelike, NaiveDate, Weekday};
use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders};
use unicode_width::UnicodeWidthStr;

use crate::tui::calendar_grid::{self, build_month_grid, events_for_date, is_current_month};
use crate::tui::model::{Focus, Model};
use crate::tui::theme;
use crate::tui::widget::utils::truncate_str;

const WEEKDAY_HEADERS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

const MONTH_NAMES: [&str; 13] = [
    "",
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

const GRID_ROWS: usize = 6;

// Header area inside the block: month_header(1) + separator(1) + weekday(1) + separator(1)
const HEADER_LINES: u16 = 4;

// Number of horizontal separators between 6 grid rows
const ROW_SEPARATOR_COUNT: u16 = 5;

// Number of vertical separators between 7 columns
const COL_SEPARATOR_COUNT: u16 = 6;

pub fn render(f: &mut Frame, area: Rect, model: &Model) {
    if area.width < 25 || area.height < 12 {
        return;
    }

    let border_style = if model.focus == Focus::Calendar {
        theme::FOCUSED_BORDER
    } else {
        theme::UNFOCUSED_BORDER
    };

    let title = format_title(model);
    let block = Block::default().title(title).borders(Borders::ALL).border_style(border_style);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.width < 13 || inner.height < 10 {
        return;
    }

    let buf = f.buffer_mut();

    let (col_xs, col_ws, sep_xs) = calculate_column_layout(inner);
    let (row_ys, row_hs, row_sep_ys) = calculate_row_layout(inner);

    draw_month_header(buf, inner, model);
    draw_h_separator(buf, area, &sep_xs, inner.y + 1, border_style, '┬');
    draw_weekday_headers(buf, &col_xs, &col_ws, inner.y + 2, &sep_xs, border_style);
    draw_h_separator(buf, area, &sep_xs, inner.y + 3, border_style, '┼');

    let grid = build_month_grid(model.current_year, model.current_month);
    let events = model
        .events_cache
        .get(&(model.current_year, model.current_month))
        .map(Vec::as_slice)
        .unwrap_or_default();

    for (ri, row) in grid.iter().enumerate().take(GRID_ROWS) {
        let y = row_ys[ri];
        let h = row_hs[ri];

        for (ci, cell) in row.iter().enumerate() {
            if let Some(date) = cell {
                let cell_area = Rect::new(col_xs[ci], y, col_ws[ci], h);
                render_cell(buf, cell_area, model, events, *date);
            }
        }

        for line in 0..h {
            for &sx in &sep_xs {
                set_cell(buf, sx, y + line, "│", border_style);
            }
        }

        if ri < 5 {
            draw_h_separator(buf, area, &sep_xs, row_sep_ys[ri], border_style, '┼');
        }
    }

    let bottom_y = area.y + area.height - 1;
    for &sx in &sep_xs {
        set_cell(buf, sx, bottom_y, "┴", border_style);
    }
}

fn calculate_column_layout(inner: Rect) -> ([u16; 7], [u16; 7], [u16; 6]) {
    let cols_available = inner.width.saturating_sub(COL_SEPARATOR_COUNT);
    let base_w = cols_available / 7;
    let extra = (cols_available % 7) as usize;

    let mut col_xs = [0u16; 7];
    let mut col_ws = [0u16; 7];
    let mut cx = inner.x;

    for i in 0..7 {
        col_xs[i] = cx;
        col_ws[i] = base_w + if i < extra { 1 } else { 0 };
        cx += col_ws[i];
        if i < 6 {
            cx += 1;
        }
    }

    let mut sep_xs = [0u16; 6];
    for i in 0..6 {
        sep_xs[i] = col_xs[i] + col_ws[i];
    }

    (col_xs, col_ws, sep_xs)
}

fn calculate_row_layout(inner: Rect) -> ([u16; 6], [u16; 6], [u16; 5]) {
    let grid_total = inner.height.saturating_sub(HEADER_LINES);
    let rows_available = grid_total.saturating_sub(ROW_SEPARATOR_COUNT);
    let base_h = rows_available / 6;
    let extra = (rows_available % 6) as usize;

    let mut row_ys = [0u16; 6];
    let mut row_hs = [0u16; 6];
    let mut ry = inner.y + HEADER_LINES;

    for i in 0..GRID_ROWS {
        row_ys[i] = ry;
        row_hs[i] = base_h + if i < extra { 1 } else { 0 };
        ry += row_hs[i];
        if i < 5 {
            ry += 1;
        }
    }

    let mut row_sep_ys = [0u16; 5];
    for i in 0..5 {
        row_sep_ys[i] = row_ys[i] + row_hs[i];
    }

    (row_ys, row_hs, row_sep_ys)
}

fn format_title(model: &Model) -> String {
    let month = MONTH_NAMES.get(model.current_month as usize).unwrap_or(&"???");
    let label = if model.is_current_month_loading() {
        " (Loading...)".to_string()
    } else {
        model.calendar_name.as_deref().map(|n| format!(" ({n})")).unwrap_or_default()
    };
    format!(" Calendar - {month} {}{label} ", model.current_year)
}

fn draw_month_header(buf: &mut Buffer, inner: Rect, model: &Model) {
    let month_text = format!(
        "{} {}",
        MONTH_NAMES.get(model.current_month as usize).unwrap_or(&"???"),
        model.current_year,
    );
    let centered = center_in_width(&month_text, inner.width as usize);
    buf.set_string(inner.x, inner.y, &centered, theme::MONTH_HEADER);
}

fn draw_weekday_headers(
    buf: &mut Buffer,
    col_xs: &[u16; 7],
    col_ws: &[u16; 7],
    y: u16,
    sep_xs: &[u16; 6],
    border_style: Style,
) {
    for i in 0..7 {
        let style = match i {
            0 => theme::SUNDAY_STYLE,
            6 => theme::SATURDAY_STYLE,
            _ => theme::WEEKDAY_STYLE,
        };
        let header = center_in_width(WEEKDAY_HEADERS[i], col_ws[i] as usize);
        buf.set_string(col_xs[i], y, &header, style);
    }

    for &sx in sep_xs {
        set_cell(buf, sx, y, "│", border_style);
    }
}

fn render_cell(
    buf: &mut Buffer,
    cell_area: Rect,
    model: &Model,
    events: &[koyomi_core::calendar::Event],
    date: NaiveDate,
) {
    let (x, y, w, h) = (cell_area.x, cell_area.y, cell_area.width, cell_area.height);
    let is_selected = date == model.selected_date;
    let is_today = date == model.today;
    let in_month = is_current_month(date, model.current_year, model.current_month);

    let bg_style = if is_selected {
        theme::SELECTED_DATE
    } else if is_today {
        theme::TODAY_DATE
    } else {
        Style::default()
    };

    if is_selected || is_today {
        buf.set_style(cell_area, bg_style);
    }

    let date_fg = if !in_month { theme::OTHER_MONTH_DATE } else { day_of_week_style(date) };
    let date_style = bg_style.patch(date_fg);

    let day_str = format!("{:>w$}", date.day(), w = w as usize);
    buf.set_string(x, y, &day_str, date_style);

    if in_month && h > 1 {
        let day_events = events_for_date(events, date, model.tz);
        let max_lines = (h as usize).saturating_sub(1);
        let cw = w as usize;
        let ev_style = bg_style.patch(theme::EVENT_TIME);

        for (i, ev) in day_events.iter().take(max_lines).enumerate() {
            if i == max_lines - 1 && day_events.len() > max_lines {
                let more = format!("+{}", day_events.len() - i);
                buf.set_string(x, y + 1 + i as u16, &*truncate_str(&more, cw), ev_style);
                break;
            }
            let time = calendar_grid::format_event_time_compact(ev, model.tz);
            let title = ev.summary.as_deref().unwrap_or("");
            let combined = format!("{time}{title}");
            let text = truncate_str(&combined, cw);
            buf.set_string(x, y + 1 + i as u16, &*text, ev_style);
        }
    }
}

fn draw_h_separator(
    buf: &mut Buffer,
    area: Rect,
    sep_xs: &[u16; 6],
    y: u16,
    style: Style,
    intersection: char,
) {
    set_cell(buf, area.x, y, "├", style);
    for x in (area.x + 1)..(area.x + area.width - 1) {
        set_cell(buf, x, y, "─", style);
    }
    set_cell(buf, area.x + area.width - 1, y, "┤", style);

    let sym = intersection.to_string();
    for &sx in sep_xs {
        set_cell(buf, sx, y, &sym, style);
    }
}

fn set_cell(buf: &mut Buffer, x: u16, y: u16, symbol: &str, style: Style) {
    if let Some(cell) = buf.cell_mut(Position::new(x, y)) {
        cell.set_symbol(symbol);
        cell.set_style(style);
    }
}

fn day_of_week_style(date: NaiveDate) -> Style {
    match date.weekday() {
        Weekday::Sun => Style::default().fg(theme::RED),
        Weekday::Sat => Style::default().fg(theme::BLUE),
        _ => Style::default(),
    }
}

fn center_in_width(text: &str, width: usize) -> String {
    let tw = UnicodeWidthStr::width(text);
    if tw >= width {
        return truncate_str(text, width).into_owned();
    }
    let left = (width - tw) / 2;
    let right = width - tw - left;
    format!("{}{}{}", " ".repeat(left), text, " ".repeat(right))
}
