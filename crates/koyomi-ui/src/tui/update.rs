use chrono::{Datelike, NaiveDate, TimeDelta};

use super::calendar_grid;
use super::message::Message;
use super::model::{Focus, Model};

pub(super) fn update(model: &mut Model, msg: Message) -> Option<Message> {
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

        Message::OpenEventModal => {
            let events = calendar_grid::events_for_date(
                model.current_month_events(),
                model.selected_date,
                model.tz,
            );
            if !events.is_empty() {
                model.event_modal_open = true;
                model.focus = Focus::EventList;
                model.event_list_index = 0;
                model.detail_scroll_offset = 0;
            }
            None
        }
        Message::CloseEventModal => {
            model.event_modal_open = false;
            model.focus = Focus::Calendar;
            model.event_list_index = 0;
            model.detail_scroll_offset = 0;
            None
        }
        Message::ModalToggleFocus => {
            model.focus = match model.focus {
                Focus::EventList => Focus::EventDetail,
                Focus::EventDetail => Focus::EventList,
                Focus::Calendar => Focus::Calendar,
            };
            None
        }
        Message::EventListUp => {
            if model.event_list_index > 0 {
                model.event_list_index -= 1;
                model.detail_scroll_offset = 0;
            }
            None
        }
        Message::EventListDown => {
            let events = calendar_grid::events_for_date(
                model.current_month_events(),
                model.selected_date,
                model.tz,
            );
            let max_index = events.len().saturating_sub(1);
            if model.event_list_index < max_index {
                model.event_list_index += 1;
                model.detail_scroll_offset = 0;
            }
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

        Message::RequestEvents => {
            model.error_message = None;
            None
        }
        Message::RefreshEvents => {
            model.events_cache.remove(&(model.current_year, model.current_month));
            Some(Message::RequestEvents)
        }
        Message::EventsLoaded { calendar_name, events_by_month } => {
            model.events_cache.extend(events_by_month);
            model.calendar_name = Some(calendar_name);
            model.error_message = None;
            None
        }
        Message::EventsLoadFailed { error } => {
            model.error_message = Some(error);
            None
        }
    }
}

fn move_date(model: &mut Model, days: i64) {
    if let Some(new_date) = model.selected_date.checked_add_signed(TimeDelta::days(days)) {
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
        .unwrap_or_else(|| {
            tracing::warn!("Could not compute days in month for {year}/{month}; defaulting to 28");
            28
        })
}

fn check_month_change(model: &Model) -> Option<Message> {
    let key = (model.current_year, model.current_month);
    if !model.events_cache.contains_key(&key) { Some(Message::RequestEvents) } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_model() -> Model {
        Model::new("primary".to_string(), koyomi_core::calendar::TimeZone::Jst)
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
        assert_eq!(model.selected_date, original + TimeDelta::days(1));
    }

    #[test]
    fn move_left_goes_back_one_day() {
        let mut model = default_model();
        let original = model.selected_date;
        update(&mut model, Message::MoveLeft);
        assert_eq!(model.selected_date, original - TimeDelta::days(1));
    }

    #[test]
    fn move_down_advances_one_week() {
        let mut model = default_model();
        let original = model.selected_date;
        update(&mut model, Message::MoveDown);
        assert_eq!(model.selected_date, original + TimeDelta::days(7));
    }

    #[test]
    fn move_up_goes_back_one_week() {
        let mut model = default_model();
        let original = model.selected_date;
        update(&mut model, Message::MoveUp);
        assert_eq!(model.selected_date, original - TimeDelta::days(7));
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
    fn open_event_modal_sets_state() {
        let mut model = default_model();
        let date = model.selected_date;
        model.events_cache.insert(
            (model.current_year, model.current_month),
            vec![koyomi_core::calendar::Event {
                id: None,
                summary: Some("Test".to_string()),
                status: None,
                organizer: None,
                location: None,
                start: Some(koyomi_core::calendar::EventDateTime::Date { date }),
                end: None,
                description: None,
                attendees: Vec::new(),
                reminders: None,
                conference_data: None,
                html_link: None,
            }],
        );

        update(&mut model, Message::OpenEventModal);
        assert!(model.event_modal_open);
        assert_eq!(model.focus, Focus::EventList);
        assert_eq!(model.event_list_index, 0);
        assert_eq!(model.detail_scroll_offset, 0);
    }

    #[test]
    fn open_event_modal_does_nothing_without_events() {
        let mut model = default_model();
        model.events_cache.insert((model.current_year, model.current_month), vec![]);

        update(&mut model, Message::OpenEventModal);
        assert!(!model.event_modal_open);
        assert_eq!(model.focus, Focus::Calendar);
    }

    #[test]
    fn close_event_modal_resets_state() {
        let mut model = default_model();
        model.event_modal_open = true;
        model.focus = Focus::EventList;
        model.event_list_index = 3;
        model.detail_scroll_offset = 10;

        update(&mut model, Message::CloseEventModal);
        assert!(!model.event_modal_open);
        assert_eq!(model.focus, Focus::Calendar);
        assert_eq!(model.event_list_index, 0);
        assert_eq!(model.detail_scroll_offset, 0);
    }

    #[test]
    fn modal_toggle_focus_switches_between_list_and_detail() {
        let mut model = default_model();
        model.focus = Focus::EventList;

        update(&mut model, Message::ModalToggleFocus);
        assert_eq!(model.focus, Focus::EventDetail);

        update(&mut model, Message::ModalToggleFocus);
        assert_eq!(model.focus, Focus::EventList);
    }

    #[test]
    fn events_loaded_updates_cache() {
        let mut model = default_model();
        let mut events_by_month = std::collections::HashMap::new();
        events_by_month.insert((2026, 2), vec![]);
        events_by_month.insert((2026, 3), vec![]);
        update(
            &mut model,
            Message::EventsLoaded { calendar_name: "Test Calendar".to_string(), events_by_month },
        );
        assert!(model.events_cache.contains_key(&(2026, 2)));
        assert!(model.events_cache.contains_key(&(2026, 3)));
        assert_eq!(model.calendar_name, Some("Test Calendar".to_string()));
    }

    #[test]
    fn events_load_failed_sets_error() {
        let mut model = default_model();
        update(&mut model, Message::EventsLoadFailed { error: "Network error".to_string() });
        assert_eq!(model.error_message, Some("Network error".to_string()));
    }

    #[test]
    fn request_events_clears_error_message() {
        let mut model = default_model();
        model.error_message = Some("previous error".to_string());
        update(&mut model, Message::RequestEvents);
        assert!(model.error_message.is_none());
    }

    #[test]
    fn event_list_up_resets_scroll_offset() {
        let mut model = default_model();
        model.event_list_index = 1;
        model.detail_scroll_offset = 5;

        update(&mut model, Message::EventListUp);
        assert_eq!(model.event_list_index, 0);
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
        assert!(matches!(result, Some(Message::RequestEvents)));
    }

    #[test]
    fn check_month_change_returns_none_when_cached() {
        let mut model = default_model();
        model.events_cache.insert((model.current_year, model.current_month), vec![]);
        let result = check_month_change(&model);
        assert!(result.is_none());
    }

    fn make_test_event(date: NaiveDate) -> koyomi_core::calendar::Event {
        koyomi_core::calendar::Event {
            id: None,
            summary: Some("Test".to_string()),
            status: None,
            organizer: None,
            location: None,
            start: Some(koyomi_core::calendar::EventDateTime::Date { date }),
            end: None,
            description: None,
            attendees: Vec::new(),
            reminders: None,
            conference_data: None,
            html_link: None,
        }
    }

    #[test]
    fn event_list_down_advances_index() {
        let mut model = default_model();
        let date = model.selected_date;
        model.events_cache.insert(
            (model.current_year, model.current_month),
            vec![make_test_event(date), make_test_event(date), make_test_event(date)],
        );
        model.event_modal_open = true;
        model.focus = Focus::EventList;

        update(&mut model, Message::EventListDown);
        assert_eq!(model.event_list_index, 1);
        assert_eq!(model.detail_scroll_offset, 0);
    }

    #[test]
    fn event_list_down_stops_at_last_index() {
        let mut model = default_model();
        let date = model.selected_date;
        model.events_cache.insert(
            (model.current_year, model.current_month),
            vec![make_test_event(date), make_test_event(date)],
        );
        model.event_list_index = 1;

        update(&mut model, Message::EventListDown);
        assert_eq!(model.event_list_index, 1);
    }

    #[test]
    fn event_list_down_does_nothing_with_single_event() {
        let mut model = default_model();
        let date = model.selected_date;
        model
            .events_cache
            .insert((model.current_year, model.current_month), vec![make_test_event(date)]);

        update(&mut model, Message::EventListDown);
        assert_eq!(model.event_list_index, 0);
    }

    #[test]
    fn detail_scroll_down_increments() {
        let mut model = default_model();
        model.detail_scroll_offset = 0;

        update(&mut model, Message::DetailScrollDown);
        assert_eq!(model.detail_scroll_offset, 1);
    }

    #[test]
    fn detail_scroll_up_decrements() {
        let mut model = default_model();
        model.detail_scroll_offset = 5;

        update(&mut model, Message::DetailScrollUp);
        assert_eq!(model.detail_scroll_offset, 4);
    }

    #[test]
    fn detail_scroll_up_saturates_at_zero() {
        let mut model = default_model();
        model.detail_scroll_offset = 0;

        update(&mut model, Message::DetailScrollUp);
        assert_eq!(model.detail_scroll_offset, 0);
    }

    #[test]
    fn advance_month_clamps_day_jan31_to_feb28() {
        let mut model = default_model();
        model.selected_date = NaiveDate::from_ymd_opt(2026, 1, 31).unwrap();
        model.current_year = 2026;
        model.current_month = 1;
        model.events_cache.insert((2026, 1), vec![]);

        update(&mut model, Message::NextMonth);
        assert_eq!(model.selected_date, NaiveDate::from_ymd_opt(2026, 2, 28).unwrap());
    }

    #[test]
    fn advance_month_clamps_day_jan31_to_feb29_leap_year() {
        let mut model = default_model();
        model.selected_date = NaiveDate::from_ymd_opt(2028, 1, 31).unwrap();
        model.current_year = 2028;
        model.current_month = 1;
        model.events_cache.insert((2028, 1), vec![]);

        update(&mut model, Message::NextMonth);
        assert_eq!(model.selected_date, NaiveDate::from_ymd_opt(2028, 2, 29).unwrap());
    }

    #[test]
    fn advance_month_clamps_day_mar31_to_apr30() {
        let mut model = default_model();
        model.selected_date = NaiveDate::from_ymd_opt(2026, 3, 31).unwrap();
        model.current_year = 2026;
        model.current_month = 3;
        model.events_cache.insert((2026, 3), vec![]);

        update(&mut model, Message::NextMonth);
        assert_eq!(model.selected_date, NaiveDate::from_ymd_opt(2026, 4, 30).unwrap());
    }

    #[test]
    fn advance_month_preserves_day_when_valid() {
        let mut model = default_model();
        model.selected_date = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
        model.current_year = 2026;
        model.current_month = 1;
        model.events_cache.insert((2026, 1), vec![]);

        update(&mut model, Message::NextMonth);
        assert_eq!(model.selected_date, NaiveDate::from_ymd_opt(2026, 2, 15).unwrap());
    }

    #[test]
    fn modal_toggle_focus_calendar_stays_calendar() {
        let mut model = default_model();
        model.focus = Focus::Calendar;

        update(&mut model, Message::ModalToggleFocus);
        assert_eq!(model.focus, Focus::Calendar);
    }

    #[test]
    fn next_month_december_to_january_crosses_year() {
        let mut model = default_model();
        model.selected_date = NaiveDate::from_ymd_opt(2025, 12, 15).unwrap();
        model.current_year = 2025;
        model.current_month = 12;
        model.events_cache.insert((2025, 12), vec![]);

        update(&mut model, Message::NextMonth);
        assert_eq!(model.current_year, 2026);
        assert_eq!(model.current_month, 1);
        assert_eq!(model.selected_date, NaiveDate::from_ymd_opt(2026, 1, 15).unwrap());
    }

    #[test]
    fn prev_month_january_to_december_crosses_year() {
        let mut model = default_model();
        model.selected_date = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
        model.current_year = 2026;
        model.current_month = 1;
        model.events_cache.insert((2026, 1), vec![]);

        update(&mut model, Message::PrevMonth);
        assert_eq!(model.current_year, 2025);
        assert_eq!(model.current_month, 12);
        assert_eq!(model.selected_date, NaiveDate::from_ymd_opt(2025, 12, 15).unwrap());
    }

    #[test]
    fn refresh_events_removes_cache_and_returns_request() {
        let mut model = default_model();
        model.events_cache.insert((model.current_year, model.current_month), vec![]);
        assert!(!model.is_current_month_loading());

        let result = update(&mut model, Message::RefreshEvents);
        assert!(model.is_current_month_loading());
        assert!(matches!(result, Some(Message::RequestEvents)));
    }
}
