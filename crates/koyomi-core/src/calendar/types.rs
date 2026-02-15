use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEvents {
    pub calendar: String,
    pub events: Vec<Event>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub summary: Option<String>,
    pub status: Option<EventStatus>,
    pub organizer: Option<Organizer>,
    pub location: Option<String>,
    pub start: Option<EventDateTime>,
    pub end: Option<EventDateTime>,
    pub description: Option<String>,
    #[serde(default)]
    pub attendees: Vec<Attendee>,
    pub reminders: Option<Reminders>,
    pub conference_data: Option<ConferenceData>,
    pub html_link: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EventDateTime {
    #[serde(rename_all = "camelCase")]
    DateTime {
        date_time: String,
        time_zone: Option<String>,
    },
    Date {
        date: String,
    },
}

impl EventDateTime {
    /// Returns the most specific time representation as a string.
    ///
    /// For timed events, returns the `dateTime` value.
    /// For all-day events, returns the `date` value.
    #[must_use]
    pub fn to_display_string(&self) -> &str {
        match self {
            EventDateTime::DateTime { date_time, .. } => date_time,
            EventDateTime::Date { date } => date,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Organizer {
    pub email: Option<String>,
    pub display_name: Option<String>,
    /// Google API uses the bare keyword `"self"`, not `"isSelf"`
    #[serde(rename = "self")]
    pub is_self: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attendee {
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub response_status: Option<ResponseStatus>,
    /// Whether this attendee is a resource (e.g. a meeting room)
    #[serde(default)]
    pub resource: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reminders {
    pub use_default: bool,
    #[serde(default)]
    pub overrides: Vec<ReminderOverride>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReminderMethod {
    Email,
    Popup,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReminderOverride {
    pub method: ReminderMethod,
    pub minutes: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConferenceData {
    #[serde(default)]
    pub entry_points: Vec<EntryPoint>,
    pub conference_solution: Option<ConferenceSolution>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EntryPointType {
    Video,
    Phone,
    Sip,
    More,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryPoint {
    pub entry_point_type: EntryPointType,
    pub uri: String,
}

/// Conference solution (e.g., Google Meet)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConferenceSolution {
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EventStatus {
    Confirmed,
    /// The event is tentatively confirmed
    Tentative,
    Cancelled,
}

impl EventStatus {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            EventStatus::Confirmed => "confirmed",
            EventStatus::Tentative => "tentative",
            EventStatus::Cancelled => "cancelled",
        }
    }
}

/// Attendee's response status to an event invitation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ResponseStatus {
    /// The attendee has not responded
    NeedsAction,
    Declined,
    /// The attendee has tentatively accepted
    Tentative,
    Accepted,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum EventPeriod {
    /// Today only
    #[default]
    Day,
    /// This week (from today to 7 days later)
    Week,
    /// This month (from today to 30 days later)
    Month,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_period_default_is_day() {
        let period = EventPeriod::default();
        assert_eq!(period, EventPeriod::Day);
    }

    #[test]
    fn event_status_serializes_lowercase() {
        assert_eq!(serde_json::to_string(&EventStatus::Confirmed).unwrap(), "\"confirmed\"");
        assert_eq!(serde_json::to_string(&EventStatus::Tentative).unwrap(), "\"tentative\"");
        assert_eq!(serde_json::to_string(&EventStatus::Cancelled).unwrap(), "\"cancelled\"");
    }

    #[test]
    fn event_status_deserializes_lowercase() {
        assert_eq!(
            serde_json::from_str::<EventStatus>("\"confirmed\"").unwrap(),
            EventStatus::Confirmed
        );
        assert_eq!(
            serde_json::from_str::<EventStatus>("\"tentative\"").unwrap(),
            EventStatus::Tentative
        );
        assert_eq!(
            serde_json::from_str::<EventStatus>("\"cancelled\"").unwrap(),
            EventStatus::Cancelled
        );
    }

    #[test]
    fn response_status_serializes_camel_case() {
        assert_eq!(serde_json::to_string(&ResponseStatus::NeedsAction).unwrap(), "\"needsAction\"");
        assert_eq!(serde_json::to_string(&ResponseStatus::Declined).unwrap(), "\"declined\"");
        assert_eq!(serde_json::to_string(&ResponseStatus::Tentative).unwrap(), "\"tentative\"");
        assert_eq!(serde_json::to_string(&ResponseStatus::Accepted).unwrap(), "\"accepted\"");
    }

    #[test]
    fn response_status_deserializes_camel_case() {
        assert_eq!(
            serde_json::from_str::<ResponseStatus>("\"needsAction\"").unwrap(),
            ResponseStatus::NeedsAction
        );
        assert_eq!(
            serde_json::from_str::<ResponseStatus>("\"declined\"").unwrap(),
            ResponseStatus::Declined
        );
        assert_eq!(
            serde_json::from_str::<ResponseStatus>("\"tentative\"").unwrap(),
            ResponseStatus::Tentative
        );
        assert_eq!(
            serde_json::from_str::<ResponseStatus>("\"accepted\"").unwrap(),
            ResponseStatus::Accepted
        );
    }

    #[test]
    fn event_deserialize_from_json() {
        let json = r#"{
            "summary": "Test Event",
            "status": "confirmed",
            "organizer": {
                "email": "test@example.com",
                "displayName": "Test User",
                "self": true
            },
            "location": "Conference Room",
            "start": {
                "dateTime": "2025-12-09T10:00:00+09:00",
                "timeZone": "Asia/Tokyo"
            },
            "end": {
                "dateTime": "2025-12-09T11:00:00+09:00",
                "timeZone": "Asia/Tokyo"
            },
            "description": "Test description",
            "attendees": [
                {
                    "email": "attendee@example.com",
                    "displayName": "Attendee",
                    "responseStatus": "accepted"
                }
            ],
            "conferenceData": {
                "entryPoints": [
                    {
                        "entryPointType": "video",
                        "uri": "https://meet.google.com/abc-defg-hij"
                    }
                ],
                "conferenceSolution": {
                    "name": "Google Meet"
                }
            },
            "htmlLink": "https://www.google.com/calendar/event?eid=abc123"
        }"#;

        let event: Event = serde_json::from_str(json).unwrap();

        assert_eq!(event.summary, Some("Test Event".to_string()));
        assert_eq!(event.status, Some(EventStatus::Confirmed));
        assert!(event.organizer.is_some());

        let organizer = event.organizer.unwrap();
        assert_eq!(organizer.email, Some("test@example.com".to_string()));
        assert_eq!(organizer.display_name, Some("Test User".to_string()));
        assert_eq!(organizer.is_self, Some(true));

        assert_eq!(event.location, Some("Conference Room".to_string()));

        let start = event.start.unwrap();
        match &start {
            EventDateTime::DateTime { date_time, time_zone } => {
                assert_eq!(date_time, "2025-12-09T10:00:00+09:00");
                assert_eq!(time_zone.as_deref(), Some("Asia/Tokyo"));
            }
            EventDateTime::Date { .. } => panic!("Expected DateTime variant"),
        }

        assert_eq!(event.attendees.len(), 1);
        assert_eq!(event.attendees[0].email, Some("attendee@example.com".to_string()));
        assert_eq!(event.attendees[0].response_status, Some(ResponseStatus::Accepted));
        assert!(!event.attendees[0].resource);

        let conference = event.conference_data.unwrap();
        assert_eq!(conference.entry_points.len(), 1);
        assert_eq!(conference.entry_points[0].entry_point_type, EntryPointType::Video);
        assert_eq!(conference.conference_solution.unwrap().name, "Google Meet");
    }

    #[test]
    fn event_deserialize_all_day_event() {
        let json = r#"{
            "summary": "All Day Event",
            "status": "confirmed",
            "start": {
                "date": "2025-12-10"
            },
            "end": {
                "date": "2025-12-11"
            },
            "htmlLink": "https://www.google.com/calendar/event?eid=def456"
        }"#;

        let event: Event = serde_json::from_str(json).unwrap();

        assert_eq!(event.summary, Some("All Day Event".to_string()));

        let start = event.start.unwrap();
        match &start {
            EventDateTime::Date { date } => {
                assert_eq!(date, "2025-12-10");
            }
            EventDateTime::DateTime { .. } => panic!("Expected Date variant"),
        }
    }

    #[test]
    fn event_deserialize_minimal() {
        let json = r#"{
            "htmlLink": "https://www.google.com/calendar/event?eid=min123"
        }"#;

        let event: Event = serde_json::from_str(json).unwrap();

        assert!(event.summary.is_none());
        assert!(event.status.is_none());
        assert!(event.organizer.is_none());
        assert!(event.start.is_none());
        assert!(event.attendees.is_empty());
        assert!(event.conference_data.is_none());
        assert_eq!(
            event.html_link,
            Some("https://www.google.com/calendar/event?eid=min123".to_string())
        );
    }

    #[test]
    fn calendar_events_serialize() {
        let events = CalendarEvents { calendar: "Test Calendar".to_string(), events: vec![] };

        let json = serde_json::to_string(&events).unwrap();
        assert!(json.contains("\"calendar\":\"Test Calendar\""));
        assert!(json.contains("\"events\":[]"));
    }

    #[test]
    fn attendee_resource_flag_deserializes() {
        let json = r#"[
            {
                "email": "room@resource.calendar.google.com",
                "displayName": "Room A (10)",
                "responseStatus": "accepted",
                "resource": true
            },
            {
                "email": "user@example.com",
                "displayName": "User",
                "responseStatus": "accepted",
                "resource": false
            },
            {
                "email": "user2@example.com",
                "responseStatus": "needsAction"
            }
        ]"#;

        let attendees: Vec<Attendee> = serde_json::from_str(json).unwrap();

        assert_eq!(attendees.len(), 3);
        assert!(attendees[0].resource);
        assert!(!attendees[1].resource);
        assert!(!attendees[2].resource);
    }

    #[test]
    fn reminders_deserialize() {
        let json = r#"{
            "useDefault": false,
            "overrides": [
                {"method": "email", "minutes": 1440},
                {"method": "popup", "minutes": 10}
            ]
        }"#;

        let reminders: Reminders = serde_json::from_str(json).unwrap();

        assert!(!reminders.use_default);
        assert_eq!(reminders.overrides.len(), 2);
        assert_eq!(reminders.overrides[0].method, ReminderMethod::Email);
        assert_eq!(reminders.overrides[0].minutes, 1440);
    }
}
