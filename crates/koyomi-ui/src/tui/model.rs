use std::collections::HashMap;

use chrono::{Datelike, Local, NaiveDate};
use koyomi_core::calendar::Event;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Calendar,
    EventList,
}

pub struct Model {
    pub current_year: i32,
    pub current_month: u32,
    pub selected_date: NaiveDate,
    pub today: NaiveDate,

    pub events_cache: HashMap<(i32, u32), Vec<Event>>,
    pub calendar_name: Option<String>,
    pub calendar_id: String,

    pub focus: Focus,
    pub sidebar_visible: bool,
    pub event_list_index: usize,
    pub detail_modal_open: bool,
    pub detail_scroll_offset: u16,

    pub loading: bool,
    pub error_message: Option<String>,

    pub should_quit: bool,
}

impl Model {
    pub fn new(calendar_id: String) -> Self {
        let today = Local::now().date_naive();
        Self {
            current_year: today.year(),
            current_month: today.month(),
            selected_date: today,
            today,
            events_cache: HashMap::new(),
            calendar_name: None,
            calendar_id,
            focus: Focus::Calendar,
            sidebar_visible: false,
            event_list_index: 0,
            detail_modal_open: false,
            detail_scroll_offset: 0,
            loading: false,
            error_message: None,
            should_quit: false,
        }
    }

    pub fn selected_date_events(&self) -> &[Event] {
        self.events_cache
            .get(&(self.current_year, self.current_month))
            .map(Vec::as_slice)
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_new_initializes_with_today() {
        let model = Model::new("primary".to_string());
        let today = Local::now().date_naive();

        assert_eq!(model.current_year, today.year());
        assert_eq!(model.current_month, today.month());
        assert_eq!(model.selected_date, today);
        assert_eq!(model.today, today);
        assert_eq!(model.calendar_id, "primary");
        assert_eq!(model.focus, Focus::Calendar);
        assert!(!model.sidebar_visible);
        assert!(!model.detail_modal_open);
        assert!(!model.loading);
        assert!(!model.should_quit);
        assert!(model.events_cache.is_empty());
        assert!(model.calendar_name.is_none());
        assert!(model.error_message.is_none());
    }

    #[test]
    fn selected_date_events_returns_empty_when_no_cache() {
        let model = Model::new("primary".to_string());
        assert!(model.selected_date_events().is_empty());
    }
}
