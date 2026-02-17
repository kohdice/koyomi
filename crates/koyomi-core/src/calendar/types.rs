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

#[derive(Debug, Clone)]
pub enum EventDateTime {
    DateTime { date_time: chrono::DateTime<chrono::FixedOffset>, time_zone: Option<String> },
    Date { date: chrono::NaiveDate },
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

        if let Some(date_time_str) = raw.date_time {
            let date_time = chrono::DateTime::parse_from_rfc3339(&date_time_str).map_err(|e| {
                serde::de::Error::custom(format!("invalid dateTime '{date_time_str}': {e}"))
            })?;
            Ok(EventDateTime::DateTime { date_time, time_zone: raw.time_zone })
        } else if let Some(date_str) = raw.date {
            let date = chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
                .map_err(|e| serde::de::Error::custom(format!("invalid date '{date_str}': {e}")))?;
            Ok(EventDateTime::Date { date })
        } else {
            Err(serde::de::Error::custom(
                "EventDateTime requires either 'dateTime' or 'date' field",
            ))
        }
    }
}

impl Serialize for EventDateTime {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        match self {
            EventDateTime::DateTime { date_time, time_zone } => {
                let mut map = serializer.serialize_map(None)?;
                map.serialize_entry("dateTime", &date_time.to_rfc3339())?;
                if let Some(tz) = time_zone {
                    map.serialize_entry("timeZone", tz)?;
                }
                map.end()
            }
            EventDateTime::Date { date } => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("date", &date.format("%Y-%m-%d").to_string())?;
                map.end()
            }
        }
    }
}

impl EventDateTime {
    /// Parse a user-supplied date/time string into `EventDateTime`.
    ///
    /// Tries RFC 3339 first (timed event), then `YYYY-MM-DD` (all-day event).
    ///
    /// # Errors
    ///
    /// Returns an error message if the input matches neither format.
    pub fn parse(s: &str) -> Result<Self, String> {
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
            return Ok(EventDateTime::DateTime { date_time: dt, time_zone: None });
        }

        if let Ok(date) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
            return Ok(EventDateTime::Date { date });
        }

        Err(format!(
            "invalid date/time '{s}': expected RFC 3339 (e.g. 2026-02-17T10:00:00+09:00) or YYYY-MM-DD (e.g. 2026-02-17)"
        ))
    }

    /// Returns the most specific time representation as a formatted string.
    ///
    /// For timed events, returns the RFC 3339 `dateTime` value.
    /// For all-day events, returns the `date` value in `YYYY-MM-DD` format.
    #[must_use]
    pub fn to_display_string(&self) -> String {
        match self {
            EventDateTime::DateTime { date_time, .. } => date_time.to_rfc3339(),
            EventDateTime::Date { date } => date.format("%Y-%m-%d").to_string(),
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

    /// Parse a status string (case-insensitive) into `EventStatus`.
    ///
    /// # Errors
    ///
    /// Returns an error message if the input is not a recognized status value.
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "confirmed" => Ok(EventStatus::Confirmed),
            "tentative" => Ok(EventStatus::Tentative),
            "cancelled" => Ok(EventStatus::Cancelled),
            _ => Err(format!(
                "invalid status '{s}': expected 'confirmed', 'tentative', or 'cancelled'"
            )),
        }
    }
}

/// Parse a comma-separated list of email addresses into `Vec<Attendee>`.
///
/// Each email is trimmed of whitespace. Empty input returns an empty Vec.
pub fn parse_attendees(s: &str) -> Vec<Attendee> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    trimmed
        .split(',')
        .map(|email| Attendee {
            email: Some(email.trim().to_string()),
            display_name: None,
            response_status: None,
            resource: false,
        })
        .collect()
}

/// Parse a reminders string into `Reminders`.
///
/// Accepts `"default"` (or empty) for default reminders,
/// or `"method:minutes,method:minutes"` format for overrides
/// (e.g. `"popup:10,email:1440"`).
///
/// # Errors
///
/// Returns an error message if the format is invalid.
pub fn parse_reminders(s: &str) -> Result<Reminders, String> {
    let trimmed = s.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("default") {
        return Ok(Reminders { use_default: true, overrides: Vec::new() });
    }

    let mut overrides = Vec::new();
    for part in trimmed.split(',') {
        let part = part.trim();
        let Some((method_str, minutes_str)) = part.split_once(':') else {
            return Err(format!(
                "invalid reminder '{part}': expected 'method:minutes' (e.g. 'popup:10')"
            ));
        };

        let method = match method_str.trim().to_lowercase().as_str() {
            "popup" => ReminderMethod::Popup,
            "email" => ReminderMethod::Email,
            other => {
                return Err(format!(
                    "invalid reminder method '{other}': expected 'popup' or 'email'"
                ));
            }
        };

        let minutes: i32 = minutes_str.trim().parse().map_err(|_| {
            format!("invalid minutes '{minutes_str}': expected a number (e.g. 10, 1440)")
        })?;

        overrides.push(ReminderOverride { method, minutes });
    }

    Ok(Reminders { use_default: false, overrides })
}

#[derive(Debug, Clone, Serialize)]
pub struct InsertEventBody {
    pub summary: String,
    pub start: EventDateTime,
    pub end: EventDateTime,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<EventStatus>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub attendees: Vec<Attendee>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reminders: Option<Reminders>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct PatchEventBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<EventDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<EventDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<EventStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attendees: Option<Vec<Attendee>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reminders: Option<Reminders>,
}

impl PatchEventBody {
    #[must_use]
    pub fn has_fields(&self) -> bool {
        self.summary.is_some()
            || self.start.is_some()
            || self.end.is_some()
            || self.description.is_some()
            || self.location.is_some()
            || self.status.is_some()
            || self.attendees.is_some()
            || self.reminders.is_some()
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
    fn event_date_time_parse_rfc3339() {
        let result = EventDateTime::parse("2026-02-17T10:00:00+09:00");
        assert!(result.is_ok());
        match result.unwrap() {
            EventDateTime::DateTime { date_time, time_zone } => {
                assert_eq!(
                    date_time,
                    chrono::DateTime::parse_from_rfc3339("2026-02-17T10:00:00+09:00").unwrap()
                );
                assert!(time_zone.is_none());
            }
            EventDateTime::Date { .. } => panic!("Expected DateTime variant"),
        }
    }

    #[test]
    fn event_date_time_parse_date_only() {
        let result = EventDateTime::parse("2026-02-17");
        assert!(result.is_ok());
        match result.unwrap() {
            EventDateTime::Date { date } => {
                assert_eq!(date, chrono::NaiveDate::from_ymd_opt(2026, 2, 17).unwrap());
            }
            EventDateTime::DateTime { .. } => panic!("Expected Date variant"),
        }
    }

    #[test]
    fn event_date_time_parse_rfc3339_utc() {
        let result = EventDateTime::parse("2026-02-17T01:00:00Z");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), EventDateTime::DateTime { .. }));
    }

    #[test]
    fn event_date_time_parse_rejects_invalid() {
        let result = EventDateTime::parse("not-a-date");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid date/time"));
    }

    #[test]
    fn event_date_time_parse_rejects_partial_time() {
        let result = EventDateTime::parse("2026-02-17T10:00");
        assert!(result.is_err());
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
                let expected =
                    chrono::DateTime::parse_from_rfc3339("2025-12-09T10:00:00+09:00").unwrap();
                assert_eq!(*date_time, expected);
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
                let expected = chrono::NaiveDate::from_ymd_opt(2025, 12, 10).unwrap();
                assert_eq!(*date, expected);
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

    #[test]
    fn insert_event_body_serializes_required_fields() {
        let body = InsertEventBody {
            summary: "Meeting".to_string(),
            start: EventDateTime::DateTime {
                date_time: chrono::DateTime::parse_from_rfc3339("2026-02-17T10:00:00+09:00")
                    .unwrap(),
                time_zone: Some("Asia/Tokyo".to_string()),
            },
            end: EventDateTime::DateTime {
                date_time: chrono::DateTime::parse_from_rfc3339("2026-02-17T11:00:00+09:00")
                    .unwrap(),
                time_zone: Some("Asia/Tokyo".to_string()),
            },
            description: None,
            location: None,
            status: None,
            attendees: Vec::new(),
            reminders: None,
        };

        let json = serde_json::to_value(&body).unwrap();
        assert_eq!(json["summary"], "Meeting");
        assert!(json["start"]["dateTime"].is_string());
        assert!(json["end"]["dateTime"].is_string());
        assert!(json.get("description").is_none());
        assert!(json.get("location").is_none());
        assert!(json.get("status").is_none());
        assert!(json.get("attendees").is_none());
        assert!(json.get("reminders").is_none());
    }

    #[test]
    fn insert_event_body_serializes_optional_fields() {
        let body = InsertEventBody {
            summary: "Meeting".to_string(),
            start: EventDateTime::Date {
                date: chrono::NaiveDate::from_ymd_opt(2026, 2, 17).unwrap(),
            },
            end: EventDateTime::Date {
                date: chrono::NaiveDate::from_ymd_opt(2026, 2, 18).unwrap(),
            },
            description: Some("A description".to_string()),
            location: Some("Room A".to_string()),
            status: Some(EventStatus::Tentative),
            attendees: vec![Attendee {
                email: Some("user@example.com".to_string()),
                display_name: None,
                response_status: None,
                resource: false,
            }],
            reminders: Some(Reminders {
                use_default: false,
                overrides: vec![ReminderOverride { method: ReminderMethod::Popup, minutes: 10 }],
            }),
        };

        let json = serde_json::to_value(&body).unwrap();
        assert_eq!(json["description"], "A description");
        assert_eq!(json["location"], "Room A");
        assert_eq!(json["start"]["date"], "2026-02-17");
        assert_eq!(json["status"], "tentative");
        assert_eq!(json["attendees"][0]["email"], "user@example.com");
        assert_eq!(json["reminders"]["useDefault"], false);
        assert_eq!(json["reminders"]["overrides"][0]["method"], "popup");
        assert_eq!(json["reminders"]["overrides"][0]["minutes"], 10);
    }

    #[test]
    fn patch_event_body_skips_none_fields() {
        let body = PatchEventBody { summary: Some("Updated".to_string()), ..Default::default() };

        let json = serde_json::to_value(&body).unwrap();
        assert_eq!(json["summary"], "Updated");
        assert!(json.get("start").is_none());
        assert!(json.get("end").is_none());
        assert!(json.get("description").is_none());
        assert!(json.get("location").is_none());
    }

    #[test]
    fn patch_event_body_empty_serializes_to_empty_object() {
        let body = PatchEventBody::default();

        let json = serde_json::to_value(&body).unwrap();
        assert_eq!(json, serde_json::json!({}));
    }

    #[test]
    fn patch_event_body_has_fields() {
        let empty = PatchEventBody::default();
        assert!(!empty.has_fields());

        let with_summary =
            PatchEventBody { summary: Some("Test".to_string()), ..Default::default() };
        assert!(with_summary.has_fields());

        let with_location =
            PatchEventBody { location: Some("Room".to_string()), ..Default::default() };
        assert!(with_location.has_fields());

        let with_status =
            PatchEventBody { status: Some(EventStatus::Tentative), ..Default::default() };
        assert!(with_status.has_fields());

        let with_attendees = PatchEventBody { attendees: Some(vec![]), ..Default::default() };
        assert!(with_attendees.has_fields());

        let with_reminders = PatchEventBody {
            reminders: Some(Reminders { use_default: true, overrides: vec![] }),
            ..Default::default()
        };
        assert!(with_reminders.has_fields());
    }

    // --- EventStatus::parse tests ---

    #[test]
    fn event_status_parse_confirmed() {
        assert_eq!(EventStatus::parse("confirmed").unwrap(), EventStatus::Confirmed);
    }

    #[test]
    fn event_status_parse_tentative() {
        assert_eq!(EventStatus::parse("tentative").unwrap(), EventStatus::Tentative);
    }

    #[test]
    fn event_status_parse_cancelled() {
        assert_eq!(EventStatus::parse("cancelled").unwrap(), EventStatus::Cancelled);
    }

    #[test]
    fn event_status_parse_case_insensitive() {
        assert_eq!(EventStatus::parse("CONFIRMED").unwrap(), EventStatus::Confirmed);
        assert_eq!(EventStatus::parse("Tentative").unwrap(), EventStatus::Tentative);
        assert_eq!(EventStatus::parse("CANCELLED").unwrap(), EventStatus::Cancelled);
    }

    #[test]
    fn event_status_parse_rejects_invalid() {
        let result = EventStatus::parse("invalid");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid status"));
    }

    // --- parse_attendees tests ---

    #[test]
    fn parse_attendees_single_email() {
        let attendees = parse_attendees("user@example.com");
        assert_eq!(attendees.len(), 1);
        assert_eq!(attendees[0].email, Some("user@example.com".to_string()));
        assert!(!attendees[0].resource);
    }

    #[test]
    fn parse_attendees_multiple_emails() {
        let attendees = parse_attendees("a@x.com, b@y.com, c@z.com");
        assert_eq!(attendees.len(), 3);
        assert_eq!(attendees[0].email, Some("a@x.com".to_string()));
        assert_eq!(attendees[1].email, Some("b@y.com".to_string()));
        assert_eq!(attendees[2].email, Some("c@z.com".to_string()));
    }

    #[test]
    fn parse_attendees_empty_string() {
        let attendees = parse_attendees("");
        assert!(attendees.is_empty());
    }

    #[test]
    fn parse_attendees_whitespace_only() {
        let attendees = parse_attendees("   ");
        assert!(attendees.is_empty());
    }

    #[test]
    fn parse_attendees_trims_whitespace() {
        let attendees = parse_attendees("  a@x.com ,  b@y.com  ");
        assert_eq!(attendees.len(), 2);
        assert_eq!(attendees[0].email, Some("a@x.com".to_string()));
        assert_eq!(attendees[1].email, Some("b@y.com".to_string()));
    }

    // --- parse_reminders tests ---

    #[test]
    fn parse_reminders_default() {
        let reminders = parse_reminders("default").unwrap();
        assert!(reminders.use_default);
        assert!(reminders.overrides.is_empty());
    }

    #[test]
    fn parse_reminders_default_case_insensitive() {
        let reminders = parse_reminders("DEFAULT").unwrap();
        assert!(reminders.use_default);
    }

    #[test]
    fn parse_reminders_empty_string() {
        let reminders = parse_reminders("").unwrap();
        assert!(reminders.use_default);
    }

    #[test]
    fn parse_reminders_single_override() {
        let reminders = parse_reminders("popup:10").unwrap();
        assert!(!reminders.use_default);
        assert_eq!(reminders.overrides.len(), 1);
        assert_eq!(reminders.overrides[0].method, ReminderMethod::Popup);
        assert_eq!(reminders.overrides[0].minutes, 10);
    }

    #[test]
    fn parse_reminders_multiple_overrides() {
        let reminders = parse_reminders("popup:10,email:1440").unwrap();
        assert!(!reminders.use_default);
        assert_eq!(reminders.overrides.len(), 2);
        assert_eq!(reminders.overrides[0].method, ReminderMethod::Popup);
        assert_eq!(reminders.overrides[0].minutes, 10);
        assert_eq!(reminders.overrides[1].method, ReminderMethod::Email);
        assert_eq!(reminders.overrides[1].minutes, 1440);
    }

    #[test]
    fn parse_reminders_rejects_invalid_format() {
        let result = parse_reminders("invalid");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expected 'method:minutes'"));
    }

    #[test]
    fn parse_reminders_rejects_invalid_method() {
        let result = parse_reminders("sms:10");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid reminder method"));
    }

    #[test]
    fn parse_reminders_rejects_invalid_minutes() {
        let result = parse_reminders("popup:abc");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid minutes"));
    }
}
