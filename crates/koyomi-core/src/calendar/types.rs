use serde::{Deserialize, Serialize};

/// Response containing calendar name and events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEventsResponse {
    pub calendar: String,
    pub events: Vec<Event>,
}

/// Calendar event with all relevant fields
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    #[serde(rename = "conferenceData")]
    pub conference_data: Option<ConferenceData>,
    #[serde(rename = "htmlLink")]
    pub html_link: Option<String>,
}

/// Event date/time representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventDateTime {
    pub date: Option<String>,
    #[serde(rename = "dateTime")]
    pub date_time: Option<String>,
    #[serde(rename = "timeZone")]
    pub time_zone: Option<String>,
}

/// Event organizer information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organizer {
    pub email: Option<String>,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    #[serde(rename = "self")]
    pub is_self: Option<bool>,
}

/// Event attendee information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attendee {
    pub email: Option<String>,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    #[serde(rename = "responseStatus")]
    pub response_status: Option<ResponseStatus>,
}

/// Reminder settings for an event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reminders {
    #[serde(rename = "useDefault")]
    pub use_default: bool,
    #[serde(default)]
    pub overrides: Vec<ReminderOverride>,
}

/// Individual reminder override
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReminderOverride {
    pub method: String,
    pub minutes: i32,
}

/// Conference/meeting information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConferenceData {
    #[serde(rename = "entryPoints", default)]
    pub entry_points: Vec<EntryPoint>,
    #[serde(rename = "conferenceSolution")]
    pub conference_solution: Option<ConferenceSolution>,
}

/// Entry point for joining a conference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryPoint {
    #[serde(rename = "entryPointType")]
    pub entry_point_type: String,
    pub uri: String,
}

/// Conference solution (e.g., Google Meet)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConferenceSolution {
    pub name: String,
}

/// Event status indicating whether the event is confirmed, tentative, or cancelled
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EventStatus {
    /// The event is confirmed
    Confirmed,
    /// The event is tentatively confirmed
    Tentative,
    /// The event is cancelled
    Cancelled,
}

impl EventStatus {
    /// Returns the status as a lowercase string
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
    /// The attendee has declined the invitation
    Declined,
    /// The attendee has tentatively accepted
    Tentative,
    /// The attendee has accepted the invitation
    Accepted,
}

/// Time period for event listing
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
        assert_eq!(start.date_time, Some("2025-12-09T10:00:00+09:00".to_string()));
        assert_eq!(start.time_zone, Some("Asia/Tokyo".to_string()));

        assert_eq!(event.attendees.len(), 1);
        assert_eq!(event.attendees[0].email, Some("attendee@example.com".to_string()));
        assert_eq!(event.attendees[0].response_status, Some(ResponseStatus::Accepted));

        let conference = event.conference_data.unwrap();
        assert_eq!(conference.entry_points.len(), 1);
        assert_eq!(conference.entry_points[0].entry_point_type, "video");
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
        assert_eq!(start.date, Some("2025-12-10".to_string()));
        assert!(start.date_time.is_none());
        assert!(start.time_zone.is_none());
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
    fn calendar_events_response_serialize() {
        let response =
            CalendarEventsResponse { calendar: "Test Calendar".to_string(), events: vec![] };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"calendar\":\"Test Calendar\""));
        assert!(json.contains("\"events\":[]"));
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
        assert_eq!(reminders.overrides[0].method, "email");
        assert_eq!(reminders.overrides[0].minutes, 1440);
    }
}
