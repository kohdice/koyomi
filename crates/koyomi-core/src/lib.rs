pub mod auth;
pub mod calendar;
pub mod client;
pub mod config;
pub mod error;

pub use auth::{login, logout};
pub use client::Client;
pub use error::{CalendarError, Error, Result};
