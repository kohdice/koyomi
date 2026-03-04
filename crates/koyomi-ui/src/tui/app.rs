use std::collections::HashMap;
use std::io::Stdout;
use std::time::Duration;

use chrono::Datelike;
use crossterm::event::{self, Event, KeyEvent};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use tokio::sync::mpsc;
use tracing::debug;

use super::calendar_grid;
use super::event_handler;
use super::message::Message;
use super::model::{Model, PendingAction};
use super::update::update;
use super::view::view;

const EVENT_POLL_TIMEOUT: Duration = Duration::from_millis(250);

/// Number of months to fetch in a single API request (center month +/- 6 = 13)
const PREFETCH_MONTHS: u32 = 13;

pub(super) struct App {
    model: Model,
    terminal: Terminal<CrosstermBackend<Stdout>>,
    client: koyomi_core::Client,
    fetch_in_progress: bool,
    pending_fetch: Option<(i32, u32)>,
    api_in_progress: bool,
}

impl App {
    pub(super) fn new(
        terminal: Terminal<CrosstermBackend<Stdout>>,
        client: koyomi_core::Client,
        calendar_id: String,
        tz: koyomi_core::calendar::TimeZone,
    ) -> Self {
        Self {
            model: Model::new(calendar_id, tz),
            terminal,
            client,
            fetch_in_progress: false,
            pending_fetch: None,
            api_in_progress: false,
        }
    }

    pub(super) async fn run(mut self) -> anyhow::Result<()> {
        let (msg_tx, mut msg_rx) = mpsc::unbounded_channel::<Message>();
        let (key_tx, mut key_rx) = mpsc::unbounded_channel::<KeyEvent>();

        let err_tx = msg_tx.clone();
        tokio::task::spawn_blocking(move || {
            loop {
                match event::poll(EVENT_POLL_TIMEOUT) {
                    Ok(true) => match event::read() {
                        Ok(Event::Key(key)) => {
                            if key_tx.send(key).is_err() {
                                break;
                            }
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                        Err(e) => {
                            tracing::warn!("Terminal event read error: {e}");
                            let _ = err_tx.send(Message::EventsLoadFailed {
                                error: format!("Terminal read error: {e}"),
                            });
                            break;
                        }
                        _ => {}
                    },
                    Ok(false) => {
                        if key_tx.is_closed() {
                            break;
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(e) => {
                        tracing::warn!("Terminal event poll error: {e}");
                        let _ = err_tx.send(Message::EventsLoadFailed {
                            error: format!("Terminal poll error: {e}"),
                        });
                        break;
                    }
                }
            }
        });

        let initial_msg = Message::RequestEvents;
        self.handle_message_chain(initial_msg, &msg_tx);

        loop {
            self.terminal.draw(|f| view(&self.model, f))?;

            if self.model.should_quit {
                break;
            }

            tokio::select! {
                Some(key) = key_rx.recv() => {
                    if let Some(msg) = event_handler::handle_key_event(&self.model, key) {
                        self.handle_message_chain(msg, &msg_tx);
                    }
                }
                Some(msg) = msg_rx.recv() => {
                    self.handle_message_chain(msg, &msg_tx);
                }
                else => {
                    tracing::warn!("All event channels closed; exiting main loop");
                    break;
                }
            }
        }

        Ok(())
    }

    fn handle_message_chain(&mut self, initial_msg: Message, tx: &mpsc::UnboundedSender<Message>) {
        let mut msg = Some(initial_msg);

        while let Some(m) = msg {
            let should_fetch = matches!(&m, Message::RequestEvents);
            let fetch_completed =
                matches!(&m, Message::EventsLoaded { .. } | Message::EventsLoadFailed { .. });

            if fetch_completed {
                self.fetch_in_progress = false;
            }

            if matches!(
                &m,
                Message::DeleteSuccess
                    | Message::DeleteFailed { .. }
                    | Message::SaveSuccess
                    | Message::SaveFailed { .. }
            ) {
                self.api_in_progress = false;
            }

            msg = update(&mut self.model, m);

            if fetch_completed
                && let Some((year, month)) = self.pending_fetch.take()
                && !self.model.events_cache.contains_key(&(year, month))
            {
                self.fetch_in_progress = true;
                self.spawn_bulk_fetch(year, month, tx.clone());
            }

            if should_fetch {
                if !self.fetch_in_progress {
                    self.fetch_in_progress = true;
                    self.pending_fetch = None;
                    self.spawn_bulk_fetch(
                        self.model.current_year,
                        self.model.current_month,
                        tx.clone(),
                    );
                } else {
                    self.pending_fetch = Some((self.model.current_year, self.model.current_month));
                }
            }

            if !self.api_in_progress
                && let Some(action) = self.model.pending_action.take()
            {
                self.api_in_progress = true;
                match action {
                    PendingAction::Delete { event_id } => {
                        self.spawn_delete_event(event_id, tx.clone());
                    }
                    PendingAction::Insert { body } => {
                        self.spawn_insert_event(body, tx.clone());
                    }
                    PendingAction::Patch { event_id, body } => {
                        self.spawn_patch_event(event_id, body, tx.clone());
                    }
                }
            }
        }
    }

    fn spawn_bulk_fetch(
        &self,
        center_year: i32,
        center_month: u32,
        tx: mpsc::UnboundedSender<Message>,
    ) {
        let client = self.client.clone();
        let calendar_id = self.model.calendar_id.clone();
        let tz = self.model.tz;
        let (start_year, start_month) = prefetch_range(center_year, center_month);

        debug!(
            "Spawning bulk fetch: center={center_year}/{center_month}, \
             range start={start_year}/{start_month}, months={PREFETCH_MONTHS}"
        );

        tokio::spawn(async move {
            let token = match koyomi_core::get_valid_token(&client).await {
                Ok(t) => t,
                Err(e) => {
                    let _ = tx.send(Message::EventsLoadFailed { error: format!("{e:#}") });
                    return;
                }
            };

            let (time_min, time_max) = match koyomi_core::calendar::time_range::for_month_range(
                start_year,
                start_month,
                PREFETCH_MONTHS,
                tz,
            ) {
                Ok(range) => range,
                Err(e) => {
                    if tx.send(Message::EventsLoadFailed { error: format!("{e:#}") }).is_err() {
                        debug!("Message channel closed; dropping EventsLoadFailed");
                    }
                    return;
                }
            };
            let config = match koyomi_core::calendar::ListEventsConfig::new(
                calendar_id,
                time_min,
                time_max,
                koyomi_core::calendar::MAX_RESULTS_LIMIT,
            ) {
                Ok(config) => config,
                Err(e) => {
                    if tx.send(Message::EventsLoadFailed { error: format!("{e:#}") }).is_err() {
                        debug!("Message channel closed; dropping EventsLoadFailed");
                    }
                    return;
                }
            };

            match client.list_events(&token, &config).await {
                Ok(result) => {
                    let events_by_month = distribute_events(
                        result.events,
                        start_year,
                        start_month,
                        PREFETCH_MONTHS,
                        tz,
                    );
                    if tx
                        .send(Message::EventsLoaded {
                            calendar_name: result.calendar,
                            events_by_month,
                        })
                        .is_err()
                    {
                        debug!("Message channel closed; dropping EventsLoaded");
                    }
                }
                Err(e) => {
                    if tx.send(Message::EventsLoadFailed { error: format!("{e:#}") }).is_err() {
                        debug!("Message channel closed; dropping EventsLoadFailed");
                    }
                }
            }
        });
    }

    fn spawn_delete_event(&self, event_id: String, tx: mpsc::UnboundedSender<Message>) {
        let client = self.client.clone();
        let calendar_id = self.model.calendar_id.clone();

        debug!("Spawning delete event: {event_id}");

        tokio::spawn(async move {
            let token = match koyomi_core::get_valid_token(&client).await {
                Ok(t) => t,
                Err(e) => {
                    let _ = tx.send(Message::DeleteFailed { error: format!("{e:#}") });
                    return;
                }
            };

            let config = match koyomi_core::calendar::DeleteEventConfig::new(calendar_id, event_id)
            {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx.send(Message::DeleteFailed { error: format!("{e:#}") });
                    return;
                }
            };

            match client.delete_event(&token, &config).await {
                Ok(()) => {
                    let _ = tx.send(Message::DeleteSuccess);
                }
                Err(e) => {
                    let _ = tx.send(Message::DeleteFailed { error: format!("{e:#}") });
                }
            }
        });
    }

    fn spawn_insert_event(
        &self,
        body: koyomi_core::calendar::InsertEventBody,
        tx: mpsc::UnboundedSender<Message>,
    ) {
        let client = self.client.clone();
        let calendar_id = self.model.calendar_id.clone();

        debug!("Spawning insert event: {}", body.summary);

        tokio::spawn(async move {
            let token = match koyomi_core::get_valid_token(&client).await {
                Ok(t) => t,
                Err(e) => {
                    let _ = tx.send(Message::SaveFailed { error: format!("{e:#}") });
                    return;
                }
            };

            let config = match koyomi_core::calendar::InsertEventConfig::new(calendar_id, body) {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx.send(Message::SaveFailed { error: format!("{e:#}") });
                    return;
                }
            };

            match client.insert_event(&token, &config).await {
                Ok(_) => {
                    let _ = tx.send(Message::SaveSuccess);
                }
                Err(e) => {
                    let _ = tx.send(Message::SaveFailed { error: format!("{e:#}") });
                }
            }
        });
    }

    fn spawn_patch_event(
        &self,
        event_id: String,
        body: koyomi_core::calendar::PatchEventBody,
        tx: mpsc::UnboundedSender<Message>,
    ) {
        let client = self.client.clone();
        let calendar_id = self.model.calendar_id.clone();

        debug!("Spawning patch event: {event_id}");

        tokio::spawn(async move {
            let token = match koyomi_core::get_valid_token(&client).await {
                Ok(t) => t,
                Err(e) => {
                    let _ = tx.send(Message::SaveFailed { error: format!("{e:#}") });
                    return;
                }
            };

            let config =
                match koyomi_core::calendar::PatchEventConfig::new(calendar_id, event_id, body) {
                    Ok(c) => c,
                    Err(e) => {
                        let _ = tx.send(Message::SaveFailed { error: format!("{e:#}") });
                        return;
                    }
                };

            match client.patch_event(&token, &config).await {
                Ok(_) => {
                    let _ = tx.send(Message::SaveSuccess);
                }
                Err(e) => {
                    let _ = tx.send(Message::SaveFailed { error: format!("{e:#}") });
                }
            }
        });
    }
}

/// Calculate the start year/month for a prefetch range centered on the given month.
fn prefetch_range(center_year: i32, center_month: u32) -> (i32, u32) {
    let half = (PREFETCH_MONTHS / 2) as i32;
    let mut year = center_year;
    let mut month = center_month as i32 - half;
    while month < 1 {
        month += 12;
        year -= 1;
    }
    (year, month as u32)
}

/// Distribute events into monthly buckets based on each event's start date.
///
/// All months in the range get an entry (empty `Vec` for months with no events),
/// so that `is_current_month_loading()` returns `false` for the entire range.
fn distribute_events(
    events: Vec<koyomi_core::calendar::Event>,
    start_year: i32,
    start_month: u32,
    months: u32,
    tz: koyomi_core::calendar::TimeZone,
) -> HashMap<(i32, u32), Vec<koyomi_core::calendar::Event>> {
    let mut map: HashMap<(i32, u32), Vec<koyomi_core::calendar::Event>> = HashMap::new();

    let mut y = start_year;
    let mut m = start_month;
    for _ in 0..months {
        map.insert((y, m), Vec::new());
        m += 1;
        if m > 12 {
            m = 1;
            y += 1;
        }
    }

    for event in events {
        if let Some(start) = &event.start {
            let date = calendar_grid::event_date(start, tz);
            let key = (date.year(), date.month());
            if let Some(bucket) = map.get_mut(&key) {
                bucket.push(event);
                continue;
            }
        }
        debug!(
            summary = event.summary.as_deref().unwrap_or("<no summary>"),
            "Dropping event: no start date or outside prefetch range"
        );
    }

    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefetch_range_normal() {
        let (year, month) = prefetch_range(2026, 2);
        assert_eq!(year, 2025);
        assert_eq!(month, 8);
    }

    #[test]
    fn prefetch_range_year_boundary() {
        let (year, month) = prefetch_range(2026, 3);
        assert_eq!(year, 2025);
        assert_eq!(month, 9);
    }

    #[test]
    fn prefetch_range_january_center() {
        let (year, month) = prefetch_range(2026, 1);
        assert_eq!(year, 2025);
        assert_eq!(month, 7);
    }

    #[test]
    fn prefetch_range_december_center() {
        let (year, month) = prefetch_range(2025, 12);
        assert_eq!(year, 2025);
        assert_eq!(month, 6);
    }

    #[test]
    fn distribute_events_empty() {
        let map = distribute_events(vec![], 2025, 8, 13, koyomi_core::calendar::TimeZone::Jst);
        assert_eq!(map.len(), 13);
        for events in map.values() {
            assert!(events.is_empty());
        }
    }

    #[test]
    fn distribute_events_assigns_to_correct_month() {
        use koyomi_core::calendar::{Event, EventDateTime};

        let event = Event {
            id: None,
            summary: Some("Test".to_string()),
            status: None,
            organizer: None,
            location: None,
            start: Some(EventDateTime::DateTime {
                date_time: chrono::DateTime::parse_from_rfc3339("2025-10-15T10:00:00+09:00")
                    .unwrap(),
                time_zone: Some("Asia/Tokyo".to_string()),
            }),
            end: None,
            description: None,
            attendees: Vec::new(),
            reminders: None,
            conference_data: None,
            html_link: None,
        };

        let map = distribute_events(vec![event], 2025, 8, 13, koyomi_core::calendar::TimeZone::Jst);
        assert_eq!(map.get(&(2025, 10)).unwrap().len(), 1);
        assert!(map.get(&(2025, 8)).unwrap().is_empty());
    }

    #[test]
    fn distribute_events_ignores_out_of_range() {
        use koyomi_core::calendar::{Event, EventDateTime};

        let event = Event {
            id: None,
            summary: Some("Out of range".to_string()),
            status: None,
            organizer: None,
            location: None,
            start: Some(EventDateTime::DateTime {
                date_time: chrono::DateTime::parse_from_rfc3339("2024-01-15T10:00:00+09:00")
                    .unwrap(),
                time_zone: Some("Asia/Tokyo".to_string()),
            }),
            end: None,
            description: None,
            attendees: Vec::new(),
            reminders: None,
            conference_data: None,
            html_link: None,
        };

        let map = distribute_events(vec![event], 2025, 8, 13, koyomi_core::calendar::TimeZone::Jst);
        for events in map.values() {
            assert!(events.is_empty());
        }
    }

    #[test]
    fn distribute_events_all_day_event() {
        use koyomi_core::calendar::{Event, EventDateTime};

        let event = Event {
            id: None,
            summary: Some("All day".to_string()),
            status: None,
            organizer: None,
            location: None,
            start: Some(EventDateTime::Date {
                date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            }),
            end: None,
            description: None,
            attendees: Vec::new(),
            reminders: None,
            conference_data: None,
            html_link: None,
        };

        let map = distribute_events(vec![event], 2025, 8, 13, koyomi_core::calendar::TimeZone::Jst);
        assert_eq!(map.get(&(2026, 1)).unwrap().len(), 1);
    }

    #[test]
    fn distribute_events_drops_event_with_no_start() {
        use koyomi_core::calendar::Event;

        let event = Event {
            id: None,
            summary: Some("No start".to_string()),
            status: None,
            organizer: None,
            location: None,
            start: None,
            end: None,
            description: None,
            attendees: Vec::new(),
            reminders: None,
            conference_data: None,
            html_link: None,
        };

        let map = distribute_events(vec![event], 2025, 8, 13, koyomi_core::calendar::TimeZone::Jst);
        for events in map.values() {
            assert!(events.is_empty());
        }
    }
}
