use chrono::{Datelike, NaiveDate, TimeDelta};
use koyomi_core::calendar::{Event, EventDateTime};

/// Build a month grid as rows of 7 columns (Sunday-start).
///
/// Each cell is `Some(NaiveDate)` for dates in or adjacent to the month,
/// or `None` if the cell is empty (only at the very end).
/// The grid always produces exactly 6 rows for consistent layout.
pub fn build_month_grid(year: i32, month: u32) -> Vec<[Option<NaiveDate>; 7]> {
    let first_day =
        NaiveDate::from_ymd_opt(year, month, 1).expect("year/month should be validated");

    let start_weekday = first_day.weekday();
    let offset = start_weekday.num_days_from_sunday() as i64;
    let grid_start = first_day - TimeDelta::days(offset);

    const ROW_COUNT: usize = 6;
    let mut rows = Vec::with_capacity(ROW_COUNT);

    for week in 0..ROW_COUNT {
        let mut row = [None; 7];
        for day in 0..7u32 {
            let date = grid_start + TimeDelta::days((week * 7 + day as usize) as i64);
            row[day as usize] = Some(date);
        }
        rows.push(row);
    }

    rows
}

/// Extract a `NaiveDate` from an `EventDateTime`, converting to the given timezone.
pub fn event_date(edt: &EventDateTime, tz: koyomi_core::calendar::TimeZone) -> Option<NaiveDate> {
    match edt {
        EventDateTime::DateTime { date_time, .. } => {
            let offset = tz.fixed_offset();
            match chrono::DateTime::parse_from_rfc3339(date_time) {
                Ok(dt) => Some(dt.with_timezone(&offset).date_naive()),
                Err(e) => {
                    tracing::debug!("Failed to parse RFC 3339 datetime '{date_time}': {e}");
                    None
                }
            }
        }
        EventDateTime::Date { date } => match NaiveDate::parse_from_str(date, "%Y-%m-%d") {
            Ok(d) => Some(d),
            Err(e) => {
                tracing::debug!("Failed to parse date '{date}': {e}");
                None
            }
        },
    }
}

/// Filter events that occur on the given date.
pub fn events_for_date(
    events: &[Event],
    date: NaiveDate,
    tz: koyomi_core::calendar::TimeZone,
) -> Vec<&Event> {
    events
        .iter()
        .filter(|e| e.start.as_ref().and_then(|s| event_date(s, tz)).is_some_and(|d| d == date))
        .collect()
}

/// Format the start time of an event for display in calendar cells.
pub fn format_event_time(event: &Event) -> String {
    match &event.start {
        Some(EventDateTime::DateTime { date_time, .. }) => {
            match chrono::DateTime::parse_from_rfc3339(date_time) {
                Ok(dt) => format!("{}", dt.format("%H:%M")),
                Err(e) => {
                    tracing::debug!("format_event_time: failed to parse '{date_time}': {e}");
                    String::new()
                }
            }
        }
        Some(EventDateTime::Date { .. }) => "All day".to_string(),
        None => String::new(),
    }
}

/// Format event time in compact form for calendar grid cells.
///
/// Returns `"HH:"` (hour + colon, 3 chars) for timed events,
/// `"00:"` for all-day events, or an empty string if no start time.
pub fn format_event_time_compact(event: &Event) -> String {
    match &event.start {
        Some(EventDateTime::DateTime { date_time, .. }) => {
            match chrono::DateTime::parse_from_rfc3339(date_time) {
                Ok(dt) => format!("{}:", dt.format("%H")),
                Err(e) => {
                    tracing::debug!(
                        "format_event_time_compact: failed to parse '{date_time}': {e}"
                    );
                    String::new()
                }
            }
        }
        Some(EventDateTime::Date { .. }) => "00:".to_string(),
        None => String::new(),
    }
}

/// Check if a date belongs to the given month.
pub fn is_current_month(date: NaiveDate, year: i32, month: u32) -> bool {
    date.year() == year && date.month() == month
}

#[cfg(test)]
mod tests {
    use chrono::Weekday;

    use super::*;

    #[test]
    fn build_month_grid_february_2026() {
        let grid = build_month_grid(2026, 2);
        assert_eq!(grid.len(), 6);

        // Feb 1, 2026 is a Sunday
        let first_cell = grid[0][0].unwrap();
        assert_eq!(first_cell, NaiveDate::from_ymd_opt(2026, 2, 1).unwrap());

        // Feb 28 should be Saturday (index 6 of row 3)
        let feb_28 = NaiveDate::from_ymd_opt(2026, 2, 28).unwrap();
        assert_eq!(feb_28.weekday(), Weekday::Sat);
    }

    #[test]
    fn build_month_grid_january_2026() {
        let grid = build_month_grid(2026, 1);
        assert_eq!(grid.len(), 6);

        // Jan 1, 2026 is a Thursday — grid starts on the previous Sunday (Dec 28, 2025)
        let first_cell = grid[0][0].unwrap();
        assert_eq!(first_cell, NaiveDate::from_ymd_opt(2025, 12, 28).unwrap());

        // Jan 1 should be in position [0][4] (Thursday)
        let jan_1 = grid[0][4].unwrap();
        assert_eq!(jan_1, NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
    }

    #[test]
    fn build_month_grid_leap_year_february() {
        let grid = build_month_grid(2028, 2);
        assert_eq!(grid.len(), 6);

        // Feb 29, 2028 should exist
        let has_feb_29 = grid
            .iter()
            .flatten()
            .any(|d| *d == Some(NaiveDate::from_ymd_opt(2028, 2, 29).unwrap()));
        assert!(has_feb_29);
    }

    #[test]
    fn event_date_parses_datetime() {
        use koyomi_core::calendar::TimeZone;
        let edt = EventDateTime::DateTime {
            date_time: "2026-02-16T10:00:00+09:00".to_string(),
            time_zone: Some("Asia/Tokyo".to_string()),
        };
        let date = event_date(&edt, TimeZone::Jst).unwrap();
        assert_eq!(date, NaiveDate::from_ymd_opt(2026, 2, 16).unwrap());
    }

    #[test]
    fn event_date_parses_all_day() {
        use koyomi_core::calendar::TimeZone;
        let edt = EventDateTime::Date { date: "2026-02-16".to_string() };
        let date = event_date(&edt, TimeZone::Jst).unwrap();
        assert_eq!(date, NaiveDate::from_ymd_opt(2026, 2, 16).unwrap());
    }

    #[test]
    fn event_date_converts_timezone_across_date_boundary() {
        use koyomi_core::calendar::TimeZone;
        // UTC 23:30 on Feb 16 → JST 08:30 on Feb 17
        let edt = EventDateTime::DateTime {
            date_time: "2026-02-16T23:30:00+00:00".to_string(),
            time_zone: Some("UTC".to_string()),
        };
        let date_utc = event_date(&edt, TimeZone::Utc).unwrap();
        assert_eq!(date_utc, NaiveDate::from_ymd_opt(2026, 2, 16).unwrap());
        let date_jst = event_date(&edt, TimeZone::Jst).unwrap();
        assert_eq!(date_jst, NaiveDate::from_ymd_opt(2026, 2, 17).unwrap());
    }

    #[test]
    fn is_current_month_checks_correctly() {
        let date = NaiveDate::from_ymd_opt(2026, 2, 16).unwrap();
        assert!(is_current_month(date, 2026, 2));
        assert!(!is_current_month(date, 2026, 3));
        assert!(!is_current_month(date, 2025, 2));
    }

    fn test_event(start: Option<EventDateTime>, summary: Option<&str>) -> Event {
        Event {
            id: None,
            summary: summary.map(String::from),
            status: None,
            organizer: None,
            location: None,
            start,
            end: None,
            description: None,
            attendees: Vec::new(),
            reminders: None,
            conference_data: None,
            html_link: None,
        }
    }

    #[test]
    fn format_event_time_compact_timed_event() {
        let event = test_event(
            Some(EventDateTime::DateTime {
                date_time: "2026-02-16T09:30:00+09:00".to_string(),
                time_zone: Some("Asia/Tokyo".to_string()),
            }),
            Some("Meeting"),
        );
        assert_eq!(format_event_time_compact(&event), "09:");
    }

    #[test]
    fn format_event_time_compact_all_day_event() {
        let event = test_event(
            Some(EventDateTime::Date { date: "2026-02-16".to_string() }),
            Some("Holiday"),
        );
        assert_eq!(format_event_time_compact(&event), "00:");
    }

    #[test]
    fn format_event_time_compact_no_start() {
        let event = test_event(None, None);
        assert_eq!(format_event_time_compact(&event), "");
    }
}
