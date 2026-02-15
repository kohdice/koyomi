use std::io::{self, Write};

use koyomi_core::calendar::{CalendarEvents, Event, EventStatus};

/// Renders calendar events as pretty-printed JSON.
///
/// When `details` is `true`, the full [`CalendarEvents`] structure is
/// serialized.  When `false`, a simplified view is produced that omits
/// internal details such as email addresses and reminder overrides.
///
/// # Errors
///
/// Returns an `io::Error` if writing to `writer` or JSON serialization fails.
pub fn render<W: Write>(writer: &mut W, events: &CalendarEvents, details: bool) -> io::Result<()> {
    if details {
        serde_json::to_writer_pretty(&mut *writer, events)?;
    } else {
        let simplified = SimplifiedResponse::from(events);
        serde_json::to_writer_pretty(&mut *writer, &simplified)?;
    }
    writeln!(writer)?;
    Ok(())
}

#[derive(serde::Serialize)]
struct SimplifiedResponse {
    calendar: String,
    events: Vec<SimplifiedEvent>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SimplifiedEvent {
    summary: Option<String>,
    status: Option<EventStatus>,
    organizer: Option<String>,
    location: Option<String>,
    start: Option<String>,
    end: Option<String>,
    description: Option<String>,
    attendees: Vec<String>,
    conference_data: Option<SimplifiedConference>,
    html_link: Option<String>,
}

#[derive(serde::Serialize)]
struct SimplifiedConference {
    name: String,
    url: Option<String>,
}

impl From<&CalendarEvents> for SimplifiedResponse {
    fn from(events: &CalendarEvents) -> Self {
        Self {
            calendar: events.calendar.clone(),
            events: events.events.iter().map(SimplifiedEvent::from).collect(),
        }
    }
}

impl From<&Event> for SimplifiedEvent {
    fn from(event: &Event) -> Self {
        Self {
            summary: event.summary.clone(),
            status: event.status,
            organizer: event
                .organizer
                .as_ref()
                .and_then(|o| o.display_name.clone().or_else(|| o.email.clone())),
            location: event.location.clone(),
            start: event
                .start
                .as_ref()
                .and_then(|dt| dt.date_time.clone().or_else(|| dt.date.clone())),
            end: event.end.as_ref().and_then(|dt| dt.date_time.clone().or_else(|| dt.date.clone())),
            description: event.description.clone(),
            attendees: event
                .attendees
                .iter()
                .filter(|a| !a.resource)
                .filter_map(|a| a.display_name.clone().or_else(|| a.email.clone()))
                .collect(),
            conference_data: event.conference_data.as_ref().and_then(|cd| {
                cd.conference_solution.as_ref().map(|cs| SimplifiedConference {
                    name: cs.name.clone(),
                    url: cd
                        .entry_points
                        .iter()
                        .find(|ep| ep.entry_point_type == "video")
                        .map(|ep| ep.uri.clone()),
                })
            }),
            html_link: event.html_link.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use koyomi_core::calendar::{
        Attendee, ConferenceData, ConferenceSolution, EntryPoint, EventDateTime, Organizer,
    };

    fn sample_events() -> CalendarEvents {
        CalendarEvents {
            calendar: "Test Calendar".to_string(),
            events: vec![Event {
                summary: Some("Meeting".to_string()),
                status: Some(EventStatus::Confirmed),
                organizer: Some(Organizer {
                    email: Some("org@example.com".to_string()),
                    display_name: Some("Organizer".to_string()),
                    is_self: Some(true),
                }),
                location: Some("Room A".to_string()),
                start: Some(EventDateTime {
                    date: None,
                    date_time: Some("2025-12-09T10:00:00+09:00".to_string()),
                    time_zone: Some("Asia/Tokyo".to_string()),
                }),
                end: Some(EventDateTime {
                    date: None,
                    date_time: Some("2025-12-09T11:00:00+09:00".to_string()),
                    time_zone: Some("Asia/Tokyo".to_string()),
                }),
                description: Some("Weekly sync".to_string()),
                attendees: vec![Attendee {
                    email: Some("att@example.com".to_string()),
                    display_name: Some("Attendee".to_string()),
                    response_status: None,
                    resource: false,
                }],
                reminders: None,
                conference_data: Some(ConferenceData {
                    entry_points: vec![EntryPoint {
                        entry_point_type: "video".to_string(),
                        uri: "https://meet.google.com/abc-defg-hij".to_string(),
                    }],
                    conference_solution: Some(ConferenceSolution {
                        name: "Google Meet".to_string(),
                    }),
                }),
                html_link: Some("https://calendar.google.com/event?eid=abc".to_string()),
            }],
        }
    }

    #[test]
    fn render_simplified_produces_valid_json() {
        let events = sample_events();
        let mut buf = Vec::new();
        render(&mut buf, &events, false).unwrap();

        let value: serde_json::Value = serde_json::from_slice(&buf).unwrap();
        assert_eq!(value["calendar"], "Test Calendar");
        assert_eq!(value["events"][0]["summary"], "Meeting");
        assert_eq!(value["events"][0]["organizer"], "Organizer");
        assert_eq!(value["events"][0]["conferenceData"]["name"], "Google Meet");
        assert_eq!(
            value["events"][0]["conferenceData"]["url"],
            "https://meet.google.com/abc-defg-hij"
        );
    }

    #[test]
    fn render_detailed_produces_valid_json() {
        let events = sample_events();
        let mut buf = Vec::new();
        render(&mut buf, &events, true).unwrap();

        let value: serde_json::Value = serde_json::from_slice(&buf).unwrap();
        assert_eq!(value["calendar"], "Test Calendar");

        let organizer = &value["events"][0]["organizer"];
        assert_eq!(organizer["email"], "org@example.com");
        assert_eq!(organizer["displayName"], "Organizer");
    }

    #[test]
    fn render_empty_events() {
        let events = CalendarEvents { calendar: "Empty".to_string(), events: vec![] };
        let mut buf = Vec::new();
        render(&mut buf, &events, false).unwrap();

        let value: serde_json::Value = serde_json::from_slice(&buf).unwrap();
        assert_eq!(value["calendar"], "Empty");
        assert_eq!(value["events"], serde_json::json!([]));
    }

    #[test]
    fn render_simplified_excludes_resource_attendees() {
        let events = CalendarEvents {
            calendar: "Test".to_string(),
            events: vec![Event {
                summary: Some("Meeting".to_string()),
                status: None,
                organizer: None,
                location: Some("Room A".to_string()),
                start: None,
                end: None,
                description: None,
                attendees: vec![
                    Attendee {
                        email: Some("room-a@resource.calendar.google.com".to_string()),
                        display_name: Some("Room A (10)".to_string()),
                        response_status: None,
                        resource: true,
                    },
                    Attendee {
                        email: Some("user@example.com".to_string()),
                        display_name: Some("User".to_string()),
                        response_status: None,
                        resource: false,
                    },
                ],
                reminders: None,
                conference_data: None,
                html_link: None,
            }],
        };

        let mut buf = Vec::new();
        render(&mut buf, &events, false).unwrap();

        let value: serde_json::Value = serde_json::from_slice(&buf).unwrap();
        let attendees = value["events"][0]["attendees"].as_array().unwrap();
        assert_eq!(attendees.len(), 1);
        assert_eq!(attendees[0], "User");
    }

    #[test]
    fn render_simplified_organizer_falls_back_to_email() {
        let events = CalendarEvents {
            calendar: "Test".to_string(),
            events: vec![Event {
                summary: None,
                status: None,
                organizer: Some(Organizer {
                    email: Some("org@example.com".to_string()),
                    display_name: None,
                    is_self: None,
                }),
                location: None,
                start: None,
                end: None,
                description: None,
                attendees: vec![],
                reminders: None,
                conference_data: None,
                html_link: None,
            }],
        };

        let mut buf = Vec::new();
        render(&mut buf, &events, false).unwrap();

        let value: serde_json::Value = serde_json::from_slice(&buf).unwrap();
        assert_eq!(value["events"][0]["organizer"], "org@example.com");
    }

    #[test]
    fn render_simplified_conference_data_includes_video_url() {
        let events = CalendarEvents {
            calendar: "Test".to_string(),
            events: vec![Event {
                summary: None,
                status: None,
                organizer: None,
                location: None,
                start: None,
                end: None,
                description: None,
                attendees: vec![],
                reminders: None,
                conference_data: Some(ConferenceData {
                    entry_points: vec![
                        EntryPoint {
                            entry_point_type: "phone".to_string(),
                            uri: "tel:+81-3-4545-0450".to_string(),
                        },
                        EntryPoint {
                            entry_point_type: "video".to_string(),
                            uri: "https://meet.google.com/xyz-abcd-efg".to_string(),
                        },
                    ],
                    conference_solution: Some(ConferenceSolution {
                        name: "Google Meet".to_string(),
                    }),
                }),
                html_link: None,
            }],
        };

        let mut buf = Vec::new();
        render(&mut buf, &events, false).unwrap();

        let value: serde_json::Value = serde_json::from_slice(&buf).unwrap();
        let cd = &value["events"][0]["conferenceData"];
        assert_eq!(cd["name"], "Google Meet");
        assert_eq!(cd["url"], "https://meet.google.com/xyz-abcd-efg");
    }
}
