use serde::de::Deserializer;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEvents {
    pub calendar: String,
    pub events: Vec<Event>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub id: Option<String>,
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

#[derive(Debug, Clone, Serialize)]
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

impl<'de> Deserialize<'de> for EventDateTime {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Raw {
            date_time: Option<String>,
            time_zone: Option<String>,
            date: Option<String>,
        }

        let raw = Raw::deserialize(deserializer)?;

        if let Some(date_time) = raw.date_time {
            chrono::DateTime::parse_from_rfc3339(&date_time).map_err(|e| {
                serde::de::Error::custom(format!("invalid dateTime '{date_time}': {e}"))
            })?;
            Ok(EventDateTime::DateTime { date_time, time_zone: raw.time_zone })
        } else if let Some(date) = raw.date {
            chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%d")
                .map_err(|e| serde::de::Error::custom(format!("invalid date '{date}': {e}")))?;
            Ok(EventDateTime::Date { date })
        } else {
            Err(serde::de::Error::custom(
                "EventDateTime requires either 'dateTime' or 'date' field",
            ))
        }
    }
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
    #[serde(other, rename = "unknown")]
    Unknown,
}

impl ReminderMethod {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            ReminderMethod::Email => "email",
            ReminderMethod::Popup => "popup",
            ReminderMethod::Unknown => "unknown",
        }
    }
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
    #[serde(other, rename = "unknown")]
    Unknown,
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
    /// Unknown status value from the API (forward-compatibility)
    #[serde(other, rename = "unknown")]
    Unknown,
}

impl EventStatus {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            EventStatus::Confirmed => "confirmed",
            EventStatus::Tentative => "tentative",
            EventStatus::Cancelled => "cancelled",
            EventStatus::Unknown => "unknown",
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
    #[serde(other, rename = "unknown")]
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let events = CalendarEvents {
            calendar: "Test Calendar".to_string(),
            events: vec![],
            truncated: false,
        };

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

    #[test]
    fn event_date_time_deserialize_neither_date_nor_datetime() {
        let json = r#"{"timeZone": "Asia/Tokyo"}"#;
        let result = serde_json::from_str::<EventDateTime>(json);

        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("dateTime"), "Error message should mention 'dateTime': {error}");
        assert!(error.contains("date"), "Error message should mention 'date': {error}");
    }

    #[test]
    fn event_status_unknown_value_deserializes_to_unknown() {
        let result = serde_json::from_str::<EventStatus>("\"newStatusFromApi\"").unwrap();
        assert_eq!(result, EventStatus::Unknown);
    }

    #[test]
    fn event_status_unknown_as_str() {
        assert_eq!(EventStatus::Unknown.as_str(), "unknown");
    }

    #[test]
    fn response_status_unknown_value_deserializes_to_unknown() {
        let result = serde_json::from_str::<ResponseStatus>("\"newResponseStatus\"").unwrap();
        assert_eq!(result, ResponseStatus::Unknown);
    }

    #[test]
    fn reminder_method_unknown_value_deserializes_to_unknown() {
        let result = serde_json::from_str::<ReminderMethod>("\"sms\"").unwrap();
        assert_eq!(result, ReminderMethod::Unknown);
    }

    #[test]
    fn entry_point_type_unknown_value_deserializes_to_unknown() {
        let result = serde_json::from_str::<EntryPointType>("\"newType\"").unwrap();
        assert_eq!(result, EntryPointType::Unknown);
    }

    #[test]
    fn event_date_time_rejects_invalid_rfc3339() {
        let json = r#"{"dateTime": "not-a-date"}"#;
        let result = serde_json::from_str::<EventDateTime>(json);
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("invalid dateTime"), "Expected 'invalid dateTime' in: {error}");
    }

    #[test]
    fn event_date_time_rejects_invalid_date() {
        let json = r#"{"date": "2025-13-40"}"#;
        let result = serde_json::from_str::<EventDateTime>(json);
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("invalid date"), "Expected 'invalid date' in: {error}");
    }

    #[test]
    fn event_date_time_accepts_valid_rfc3339() {
        let json = r#"{"dateTime": "2025-12-09T10:00:00+09:00", "timeZone": "Asia/Tokyo"}"#;
        let result = serde_json::from_str::<EventDateTime>(json);
        assert!(result.is_ok());
    }

    #[test]
    fn event_date_time_accepts_valid_date() {
        let json = r#"{"date": "2025-12-09"}"#;
        let result = serde_json::from_str::<EventDateTime>(json);
        assert!(result.is_ok());
    }
}
