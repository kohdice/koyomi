use std::collections::HashMap;

use chrono::{Datelike, NaiveDate};
use koyomi_core::calendar::{
    Event, InsertEventBody, PatchEventBody, ReminderMethod, ReminderOverride, Reminders,
};

use super::calendar_grid;
use super::text_input::TextInput;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Focus {
    Calendar,
    EventList,
    EventDetail,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum FormMode {
    Add,
    Edit { event_id: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FormField {
    Summary,
    Start,
    End,
    Description,
    Location,
    Status,
    Attendees,
    Reminders,
}

impl FormField {
    pub(super) fn next(self) -> Self {
        match self {
            FormField::Summary => FormField::Start,
            FormField::Start => FormField::End,
            FormField::End => FormField::Description,
            FormField::Description => FormField::Location,
            FormField::Location => FormField::Status,
            FormField::Status => FormField::Attendees,
            FormField::Attendees => FormField::Reminders,
            FormField::Reminders => FormField::Summary,
        }
    }

    pub(super) fn prev(self) -> Self {
        match self {
            FormField::Summary => FormField::Reminders,
            FormField::Start => FormField::Summary,
            FormField::End => FormField::Start,
            FormField::Description => FormField::End,
            FormField::Location => FormField::Description,
            FormField::Status => FormField::Location,
            FormField::Attendees => FormField::Status,
            FormField::Reminders => FormField::Attendees,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ReminderPresetValue {
    NoReminder,
    UseDefault,
    Popup(i32),
}

pub(super) struct ReminderPreset {
    pub(super) label: &'static str,
    pub(super) value: ReminderPresetValue,
}

impl ReminderPreset {
    pub(super) fn to_reminders(&self) -> Option<Reminders> {
        match self.value {
            ReminderPresetValue::NoReminder => {
                Some(Reminders { use_default: false, overrides: Vec::new() })
            }
            ReminderPresetValue::UseDefault => {
                Some(Reminders { use_default: true, overrides: Vec::new() })
            }
            ReminderPresetValue::Popup(m) => Some(Reminders {
                use_default: false,
                overrides: vec![ReminderOverride { method: ReminderMethod::Popup, minutes: m }],
            }),
        }
    }
}

pub(super) const REMINDER_PRESETS: [ReminderPreset; 11] = [
    ReminderPreset { label: "なし", value: ReminderPresetValue::NoReminder },
    ReminderPreset { label: "デフォルト", value: ReminderPresetValue::UseDefault },
    ReminderPreset { label: "予定の開始時刻", value: ReminderPresetValue::Popup(0) },
    ReminderPreset { label: "5分前", value: ReminderPresetValue::Popup(5) },
    ReminderPreset { label: "10分前", value: ReminderPresetValue::Popup(10) },
    ReminderPreset { label: "15分前", value: ReminderPresetValue::Popup(15) },
    ReminderPreset { label: "30分前", value: ReminderPresetValue::Popup(30) },
    ReminderPreset { label: "1時間前", value: ReminderPresetValue::Popup(60) },
    ReminderPreset { label: "2時間前", value: ReminderPresetValue::Popup(120) },
    ReminderPreset { label: "1日前", value: ReminderPresetValue::Popup(1440) },
    ReminderPreset { label: "2日前", value: ReminderPresetValue::Popup(2880) },
];

pub(super) const DEFAULT_REMINDER_PRESET_INDEX: usize = 1;

pub(super) fn reminder_preset_index_from(reminders: &Option<Reminders>) -> usize {
    match reminders {
        None => DEFAULT_REMINDER_PRESET_INDEX,
        Some(r) if r.use_default => 1,
        Some(r) if r.overrides.is_empty() => 0,
        Some(r) if r.overrides.len() == 1 && r.overrides[0].method == ReminderMethod::Popup => {
            let minutes = r.overrides[0].minutes;
            REMINDER_PRESETS
                .iter()
                .position(|p| p.value == ReminderPresetValue::Popup(minutes))
                .unwrap_or(DEFAULT_REMINDER_PRESET_INDEX)
        }
        Some(_) => DEFAULT_REMINDER_PRESET_INDEX,
    }
}

#[derive(Debug, Clone)]
pub(super) struct EventFormState {
    pub(super) mode: FormMode,
    pub(super) fields: [TextInput; 8],
    pub(super) focused_field: FormField,
    pub(super) validation_error: Option<String>,
    pub(super) reminder_preset_index: usize,
    pub(super) reminder_changed: bool,
    pub(super) initial_attendees: String,
    pub(super) initial_description: String,
    pub(super) initial_location: String,
    pub(super) initial_status: String,
}

#[derive(Debug, Clone)]
pub(super) struct DeleteConfirmState {
    pub(super) event_id: String,
    pub(super) event_summary: String,
}

#[derive(Debug)]
pub(super) enum PendingAction {
    Insert { body: InsertEventBody },
    Patch { event_id: String, body: PatchEventBody },
    Delete { event_id: String },
}

pub(super) struct Model {
    pub(super) current_year: i32,
    pub(super) current_month: u32,
    pub(super) selected_date: NaiveDate,
    pub(super) today: NaiveDate,
    pub(super) tz: koyomi_core::calendar::TimeZone,

    pub(super) events_cache: HashMap<(i32, u32), Vec<Event>>,
    pub(super) calendar_name: Option<String>,
    pub(super) calendar_id: String,

    pub(super) focus: Focus,
    pub(super) event_modal_open: bool,
    pub(super) event_list_index: usize,
    pub(super) detail_scroll_offset: u16,

    pub(super) event_form: Option<EventFormState>,
    pub(super) delete_confirm: Option<DeleteConfirmState>,
    pub(super) status_message: Option<String>,
    pub(super) pending_action: Option<PendingAction>,

    pub(super) error_message: Option<String>,

    pub(super) should_quit: bool,
}

impl Model {
    pub(super) fn new(calendar_id: String, tz: koyomi_core::calendar::TimeZone) -> Self {
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
            event_form: None,
            delete_confirm: None,
            status_message: None,
            pending_action: None,
            error_message: None,
            should_quit: false,
        }
    }

    pub(super) fn is_current_month_loading(&self) -> bool {
        !self.events_cache.contains_key(&(self.current_year, self.current_month))
    }

    pub(super) fn current_month_events(&self) -> &[Event] {
        self.events_cache
            .get(&(self.current_year, self.current_month))
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    pub(super) fn selected_event(&self) -> Option<&Event> {
        let events = calendar_grid::events_for_date(
            self.current_month_events(),
            self.selected_date,
            self.tz,
        );
        events.into_iter().nth(self.event_list_index)
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
        assert!(model.event_form.is_none());
        assert!(model.delete_confirm.is_none());
        assert!(model.status_message.is_none());
        assert!(model.pending_action.is_none());
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
