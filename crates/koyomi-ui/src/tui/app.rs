use std::io::Stdout;
use std::time::Duration;

use crossterm::event::{self, Event, KeyEvent};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use tokio::sync::mpsc;
use tracing::debug;

use super::event_handler;
use super::message::Message;
use super::model::Model;
use super::update::update;
use super::view::view;

const EVENT_POLL_TIMEOUT: Duration = Duration::from_millis(250);

pub struct App {
    model: Model,
    terminal: Terminal<CrosstermBackend<Stdout>>,
    client: koyomi_core::Client,
    token: koyomi_core::StoredToken,
}

impl App {
    pub fn new(
        terminal: Terminal<CrosstermBackend<Stdout>>,
        client: koyomi_core::Client,
        token: koyomi_core::StoredToken,
        calendar_id: String,
    ) -> Self {
        Self { model: Model::new(calendar_id), terminal, client, token }
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        let (msg_tx, mut msg_rx) = mpsc::unbounded_channel::<Message>();
        let (key_tx, mut key_rx) = mpsc::unbounded_channel::<KeyEvent>();

        tokio::task::spawn_blocking(move || {
            loop {
                match event::poll(EVENT_POLL_TIMEOUT) {
                    Ok(true) => match event::read() {
                        Ok(Event::Key(key)) => {
                            if key_tx.send(key).is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                        _ => {}
                    },
                    Ok(false) => {
                        if key_tx.is_closed() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        let initial_msg = Message::RequestEvents {
            year: self.model.current_year,
            month: self.model.current_month,
        };
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
            }
        }

        Ok(())
    }

    fn handle_message_chain(&mut self, initial_msg: Message, tx: &mpsc::UnboundedSender<Message>) {
        let mut msg = Some(initial_msg);

        while let Some(m) = msg {
            let request_data = if let Message::RequestEvents { year, month } = &m {
                Some((*year, *month))
            } else {
                None
            };

            msg = update(&mut self.model, m);

            if let Some((year, month)) = request_data {
                self.spawn_event_fetch(year, month, tx.clone());
            }
        }
    }

    fn spawn_event_fetch(&self, year: i32, month: u32, tx: mpsc::UnboundedSender<Message>) {
        let client = self.client.clone();
        let token = self.token.clone();
        let calendar_id = self.model.calendar_id.clone();

        debug!("Spawning event fetch for {year}/{month}");

        tokio::spawn(async move {
            let period = koyomi_core::calendar::EventPeriod::YearMonth { year, month };
            let config =
                match koyomi_core::calendar::ListEventsConfig::new(calendar_id, period, 250) {
                    Ok(config) => config,
                    Err(e) => {
                        let _ = tx.send(Message::EventsLoadFailed { error: format!("{e:#}") });
                        return;
                    }
                };

            match client.list_events(&token, &config).await {
                Ok(result) => {
                    let _ = tx.send(Message::EventsLoaded {
                        year,
                        month,
                        calendar_name: result.calendar,
                        events: result.events,
                    });
                }
                Err(e) => {
                    let _ = tx.send(Message::EventsLoadFailed { error: format!("{e:#}") });
                }
            }
        });
    }
}
