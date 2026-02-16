use chrono::{Datelike, Duration, NaiveDate};

use super::calendar_grid;
use super::message::Message;
use super::model::{Focus, Model};

pub fn update(model: &mut Model, msg: Message) -> Option<Message> {
    match msg {
        Message::Quit => {
            model.should_quit = true;
            None
        }

        Message::MoveLeft => {
            move_date(model, -1);
            check_month_change(model)
        }
        Message::MoveRight => {
            move_date(model, 1);
            check_month_change(model)
        }
        Message::MoveUp => {
            move_date(model, -7);
            check_month_change(model)
        }
        Message::MoveDown => {
            move_date(model, 7);
            check_month_change(model)
        }
        Message::NextMonth => {
            advance_month(model, 1);
            check_month_change(model)
        }
        Message::PrevMonth => {
            advance_month(model, -1);
            check_month_change(model)
        }
        Message::GoToToday => {
            model.selected_date = model.today;
            model.current_year = model.today.year();
            model.current_month = model.today.month();
            model.event_list_index = 0;
            check_month_change(model)
        }

        Message::ToggleSidebar => {
            model.sidebar_visible = !model.sidebar_visible;
            if !model.sidebar_visible {
                model.focus = Focus::Calendar;
            }
            None
        }
        Message::ToggleFocus => {
            if model.sidebar_visible {
                model.focus = match model.focus {
                    Focus::Calendar => Focus::EventList,
                    Focus::EventList => Focus::Calendar,
                };
                model.event_list_index = 0;
            }
            None
        }
        Message::EventListUp => {
            if model.event_list_index > 0 {
                model.event_list_index -= 1;
            }
            None
        }
        Message::EventListDown => {
            let events =
                calendar_grid::events_for_date(model.selected_date_events(), model.selected_date);
            let max_index = events.len().saturating_sub(1);
            if model.event_list_index < max_index {
                model.event_list_index += 1;
            }
            None
        }
        Message::OpenDetail => {
            let events =
                calendar_grid::events_for_date(model.selected_date_events(), model.selected_date);
            if !events.is_empty() {
                model.detail_modal_open = true;
                model.detail_scroll_offset = 0;
            }
            None
        }
        Message::CloseDetail => {
            model.detail_modal_open = false;
            model.detail_scroll_offset = 0;
            None
        }
        Message::DetailScrollUp => {
            model.detail_scroll_offset = model.detail_scroll_offset.saturating_sub(1);
            None
        }
        Message::DetailScrollDown => {
            model.detail_scroll_offset = model.detail_scroll_offset.saturating_add(1);
            None
        }
        Message::DetailScrollTop => {
            model.detail_scroll_offset = 0;
            None
        }
        Message::DetailScrollBottom => {
            model.detail_scroll_offset = u16::MAX;
            None
        }

        Message::RequestEvents { .. } => {
            model.loading = true;
            model.error_message = None;
            None
        }
        Message::EventsLoaded { year, month, calendar_name, events } => {
            model.events_cache.insert((year, month), events);
            model.calendar_name = Some(calendar_name);
            model.loading = false;
            model.error_message = None;
            None
        }
        Message::EventsLoadFailed { error } => {
            model.loading = false;
            model.error_message = Some(error);
            None
        }
    }
}

fn move_date(model: &mut Model, days: i64) {
    if let Some(new_date) = model.selected_date.checked_add_signed(Duration::days(days)) {
        model.selected_date = new_date;
        model.current_year = new_date.year();
        model.current_month = new_date.month();
        model.event_list_index = 0;
    }
}

fn advance_month(model: &mut Model, delta: i32) {
    let mut year = model.current_year;
    let mut month = model.current_month as i32 + delta;

    while month < 1 {
        month += 12;
        year -= 1;
    }
    while month > 12 {
        month -= 12;
        year += 1;
    }

    model.current_year = year;
    model.current_month = month as u32;

    let max_day = days_in_month(year, model.current_month);
    let day = model.selected_date.day().min(max_day);

    if let Some(new_date) = NaiveDate::from_ymd_opt(year, model.current_month, day) {
        model.selected_date = new_date;
    }

    model.event_list_index = 0;
}

fn days_in_month(year: i32, month: u32) -> u32 {
    NaiveDate::from_ymd_opt(year, month + 1, 1)
        .or_else(|| NaiveDate::from_ymd_opt(year + 1, 1, 1))
        .and_then(|d| d.pred_opt())
        .map(|d| d.day())
        .unwrap_or(28)
}

fn check_month_change(model: &Model) -> Option<Message> {
    let key = (model.current_year, model.current_month);
    if !model.events_cache.contains_key(&key) {
        Some(Message::RequestEvents { year: key.0, month: key.1 })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_model() -> Model {
        Model::new("primary".to_string())
    }

    #[test]
    fn quit_sets_should_quit() {
        let mut model = default_model();
        let result = update(&mut model, Message::Quit);
        assert!(model.should_quit);
        assert!(result.is_none());
    }

    #[test]
    fn move_right_advances_one_day() {
        let mut model = default_model();
        let original = model.selected_date;
        update(&mut model, Message::MoveRight);
        assert_eq!(model.selected_date, original + Duration::days(1));
    }

    #[test]
    fn move_left_goes_back_one_day() {
        let mut model = default_model();
        let original = model.selected_date;
        update(&mut model, Message::MoveLeft);
        assert_eq!(model.selected_date, original - Duration::days(1));
    }

    #[test]
    fn move_down_advances_one_week() {
        let mut model = default_model();
        let original = model.selected_date;
        update(&mut model, Message::MoveDown);
        assert_eq!(model.selected_date, original + Duration::days(7));
    }

    #[test]
    fn move_up_goes_back_one_week() {
        let mut model = default_model();
        let original = model.selected_date;
        update(&mut model, Message::MoveUp);
        assert_eq!(model.selected_date, original - Duration::days(7));
    }

    #[test]
    fn go_to_today_resets_to_today() {
        let mut model = default_model();
        // Move far from today
        for _ in 0..60 {
            update(&mut model, Message::MoveRight);
        }
        assert_ne!(model.selected_date, model.today);

        update(&mut model, Message::GoToToday);
        assert_eq!(model.selected_date, model.today);
        assert_eq!(model.current_year, model.today.year());
        assert_eq!(model.current_month, model.today.month());
    }

    #[test]
    fn next_month_advances_month() {
        let mut model = default_model();
        let original_month = model.current_month;
        let original_year = model.current_year;

        update(&mut model, Message::NextMonth);

        if original_month == 12 {
            assert_eq!(model.current_month, 1);
            assert_eq!(model.current_year, original_year + 1);
        } else {
            assert_eq!(model.current_month, original_month + 1);
            assert_eq!(model.current_year, original_year);
        }
    }

    #[test]
    fn prev_month_goes_back_month() {
        let mut model = default_model();
        let original_month = model.current_month;
        let original_year = model.current_year;

        update(&mut model, Message::PrevMonth);

        if original_month == 1 {
            assert_eq!(model.current_month, 12);
            assert_eq!(model.current_year, original_year - 1);
        } else {
            assert_eq!(model.current_month, original_month - 1);
            assert_eq!(model.current_year, original_year);
        }
    }

    #[test]
    fn toggle_sidebar_flips_visibility() {
        let mut model = default_model();
        assert!(!model.sidebar_visible);

        update(&mut model, Message::ToggleSidebar);
        assert!(model.sidebar_visible);

        update(&mut model, Message::ToggleSidebar);
        assert!(!model.sidebar_visible);
    }

    #[test]
    fn toggle_sidebar_off_resets_focus_to_calendar() {
        let mut model = default_model();
        model.sidebar_visible = true;
        model.focus = Focus::EventList;

        update(&mut model, Message::ToggleSidebar);
        assert_eq!(model.focus, Focus::Calendar);
    }

    #[test]
    fn toggle_focus_switches_between_calendar_and_event_list() {
        let mut model = default_model();
        model.sidebar_visible = true;

        update(&mut model, Message::ToggleFocus);
        assert_eq!(model.focus, Focus::EventList);

        update(&mut model, Message::ToggleFocus);
        assert_eq!(model.focus, Focus::Calendar);
    }

    #[test]
    fn toggle_focus_does_nothing_when_sidebar_hidden() {
        let mut model = default_model();
        assert!(!model.sidebar_visible);

        update(&mut model, Message::ToggleFocus);
        assert_eq!(model.focus, Focus::Calendar);
    }

    #[test]
    fn events_loaded_updates_cache() {
        let mut model = default_model();
        update(
            &mut model,
            Message::EventsLoaded {
                year: 2026,
                month: 2,
                calendar_name: "Test Calendar".to_string(),
                events: vec![],
            },
        );
        assert!(model.events_cache.contains_key(&(2026, 2)));
        assert_eq!(model.calendar_name, Some("Test Calendar".to_string()));
        assert!(!model.loading);
    }

    #[test]
    fn events_load_failed_sets_error() {
        let mut model = default_model();
        model.loading = true;
        update(&mut model, Message::EventsLoadFailed { error: "Network error".to_string() });
        assert!(!model.loading);
        assert_eq!(model.error_message, Some("Network error".to_string()));
    }

    #[test]
    fn request_events_sets_loading() {
        let mut model = default_model();
        update(&mut model, Message::RequestEvents { year: 2026, month: 2 });
        assert!(model.loading);
        assert!(model.error_message.is_none());
    }

    #[test]
    fn close_detail_resets_state() {
        let mut model = default_model();
        model.detail_modal_open = true;
        model.detail_scroll_offset = 10;

        update(&mut model, Message::CloseDetail);
        assert!(!model.detail_modal_open);
        assert_eq!(model.detail_scroll_offset, 0);
    }

    #[test]
    fn days_in_month_returns_correct_values() {
        assert_eq!(days_in_month(2026, 1), 31);
        assert_eq!(days_in_month(2026, 2), 28);
        assert_eq!(days_in_month(2028, 2), 29); // leap year
        assert_eq!(days_in_month(2026, 4), 30);
        assert_eq!(days_in_month(2026, 12), 31);
    }

    #[test]
    fn check_month_change_returns_request_when_not_cached() {
        let model = default_model();
        let result = check_month_change(&model);
        assert!(matches!(result, Some(Message::RequestEvents { .. })));
    }

    #[test]
    fn check_month_change_returns_none_when_cached() {
        let mut model = default_model();
        model.events_cache.insert((model.current_year, model.current_month), vec![]);
        let result = check_month_change(&model);
        assert!(result.is_none());
    }
}
