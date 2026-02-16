use std::collections::HashMap;

use chrono::{Datelike, NaiveDate};
use koyomi_core::calendar::Event;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Calendar,
    EventList,
    EventDetail,
}

pub struct Model {
    pub current_year: i32,
    pub current_month: u32,
    pub selected_date: NaiveDate,
    pub today: NaiveDate,
    pub tz: koyomi_core::calendar::TimeZone,

    pub events_cache: HashMap<(i32, u32), Vec<Event>>,
    pub calendar_name: Option<String>,
    pub calendar_id: String,

    pub focus: Focus,
    pub event_modal_open: bool,
    pub event_list_index: usize,
    pub detail_scroll_offset: u16,

    pub error_message: Option<String>,

    pub should_quit: bool,
}

impl Model {
    pub fn new(calendar_id: String, tz: koyomi_core::calendar::TimeZone) -> Self {
        let today = tz.today();
        Self {
            current_year: today.year(),
            current_month: today.month(),
            selected_date: today,
            today,
            tz,
            events_cache: HashMap::new(),
            calendar_name: None,
            calendar_id,
            focus: Focus::Calendar,
            event_modal_open: false,
            event_list_index: 0,
            detail_scroll_offset: 0,
            error_message: None,
            should_quit: false,
        }
    }

    pub fn is_current_month_loading(&self) -> bool {
        !self.events_cache.contains_key(&(self.current_year, self.current_month))
    }

    pub fn current_month_events(&self) -> &[Event] {
        self.events_cache
            .get(&(self.current_year, self.current_month))
            .map(Vec::as_slice)
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use koyomi_core::calendar::TimeZone;

    use super::*;

    #[test]
    fn model_new_initializes_with_today() {
        let tz = TimeZone::Jst;
        let model = Model::new("primary".to_string(), tz);
        let today = tz.today();

        assert_eq!(model.current_year, today.year());
        assert_eq!(model.current_month, today.month());
        assert_eq!(model.selected_date, today);
        assert_eq!(model.today, today);
        assert_eq!(model.calendar_id, "primary");
        assert_eq!(model.focus, Focus::Calendar);
        assert!(!model.event_modal_open);
        assert!(!model.should_quit);
        assert!(model.events_cache.is_empty());
        assert!(model.calendar_name.is_none());
        assert!(model.error_message.is_none());
    }

    #[test]
    fn current_month_events_returns_empty_when_no_cache() {
        let model = Model::new("primary".to_string(), TimeZone::Jst);
        assert!(model.current_month_events().is_empty());
    }

    #[test]
    fn is_current_month_loading_true_when_empty_cache() {
        let model = Model::new("primary".to_string(), TimeZone::Jst);
        assert!(model.is_current_month_loading());
    }

    #[test]
    fn is_current_month_loading_false_when_cached() {
        let mut model = Model::new("primary".to_string(), TimeZone::Jst);
        model.events_cache.insert((model.current_year, model.current_month), vec![]);
        assert!(!model.is_current_month_loading());
    }

    #[test]
    fn is_current_month_loading_true_when_different_month_cached() {
        let mut model = Model::new("primary".to_string(), TimeZone::Jst);
        let other_month = if model.current_month == 12 { 1 } else { model.current_month + 1 };
        let other_year =
            if model.current_month == 12 { model.current_year + 1 } else { model.current_year };
        model.events_cache.insert((other_year, other_month), vec![]);
        assert!(model.is_current_month_loading());
    }
}
