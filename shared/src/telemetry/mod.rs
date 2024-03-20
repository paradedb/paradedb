mod bgworker;
mod event;
mod postgres;
mod posthog;

use self::event::TelemetryEvent;
pub use bgworker::{setup_telemetry_background_worker, ParadeExtension};
use std::{
    env::VarError,
    path::PathBuf,
    str::Utf8Error,
    thread,
    time::{Duration, Instant},
};
use thiserror::Error;

pub trait TelemetrySettings {
    fn enabled(&self) -> bool;
}

pub trait TelemetryStore {
    type Error;

    fn get_connection(
        &self,
    ) -> Result<Box<dyn TelemetryConnection<Error = Self::Error>>, Self::Error>;
}

pub trait TelemetryConnection {
    type Error;

    fn send(&self, uuid: &str, event: &TelemetryEvent) -> Result<(), Self::Error>;
}

pub trait DirectoryStore {
    type Error;

    fn root_path(&self) -> Result<PathBuf, Self::Error>;
    fn extension_path(&self) -> Result<PathBuf, Self::Error>;
    fn extension_size(&self) -> Result<u64, Self::Error>;
    fn extension_uuid(&self) -> Result<String, Self::Error>;
    fn extension_uuid_path(&self) -> Result<PathBuf, Self::Error>;
}

pub trait TermPoll {
    fn term_poll(&self) -> bool;
}

pub struct TelemetrySender {
    pub extension_name: String,
    pub directory_store: Box<dyn DirectoryStore<Error = TelemetryError>>,
    pub telemetry_store: Box<dyn TelemetryStore<Error = TelemetryError>>,
    pub settings_store: Box<dyn TelemetrySettings>,
}

impl TelemetrySender {
    pub fn send(&self, uuid: &str, event: &TelemetryEvent) -> Result<(), TelemetryError> {
        let conn = self.telemetry_store.get_connection()?;
        if self.settings_store.enabled() {
            conn.send(&uuid, &event)
        } else {
            pgrx::log!(
                "paradedb telemetry is disabled, not sending event: {}",
                event.name()
            );
            Ok(())
        }
    }
    pub fn send_deployment(&self) -> Result<(), TelemetryError> {
        if self.directory_store.extension_uuid_path()?.exists() {
            pgrx::log!("extension has been deployed before, skipping deployment telemetry");
            return Ok(());
        }
        let uuid = self.directory_store.extension_uuid()?;
        let event = TelemetryEvent::Deployment {
            extension: self.extension_name.clone(),
        };

        self.send(&uuid, &event)
    }

    pub fn send_directory_check(&self) -> Result<(), TelemetryError> {
        let uuid = self.directory_store.extension_uuid()?;
        let size = self.directory_store.extension_size()?;
        let path = self.directory_store.extension_path()?;
        let event = TelemetryEvent::DirectoryStatus {
            path,
            size,
            extension: self.extension_name.clone(),
        };

        self.send(&uuid, &event)
    }
}

pub struct TelemetryController {
    pub sender: TelemetrySender,
    pub directory_check_interval: Duration,
    pub sleep_interval: Duration,
    pub term_poll: Box<dyn TermPoll>,
}

impl TelemetryController {
    pub fn run(&self) -> Result<(), TelemetryError> {
        let mut last_action_time = Instant::now();
        self.sender.send_deployment()?;
        loop {
            // Sleep for a short period to remain responsive to SIGTERM
            thread::sleep(self.sleep_interval);

            // Check if the wait_duration has passed since the last time we sent telemetry data
            if Instant::now().duration_since(last_action_time) >= self.directory_check_interval {
                self.sender.send_directory_check()?;
                last_action_time = Instant::now();
            }

            // Check for shutdown
            if self.term_poll.term_poll() {
                return Ok(());
            }
        }
    }
}

#[derive(Error, Debug)]
pub enum TelemetryError {
    #[error("could not de-toast extension name for telemetry: {0}")]
    DetoastExtensionName(#[source] Utf8Error),
    #[error("could not check telemetry file for handled status: {0}")]
    HandledCheck(#[source] std::io::Error),
    #[error("could not read PGDATA variable for telemetry director: {0}")]
    NoPgData(#[source] VarError),
    #[error("could not read telemetry config: {0}")]
    ConfigEnv(#[source] envy::Error),
    #[error("could not send telemetry request: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("could not write uuid file: {0}")]
    WriteUuid(#[source] std::io::Error),
    #[error("could not read uuid file: {0}")]
    ReadUuid(#[source] std::io::Error),
    #[error("could not parse uuid file: {0}")]
    ParseUuid(#[source] uuid::Error),
    #[error("missing posthog api key")]
    PosthogApiKey,
    #[error("missing posthog api key")]
    PosthogHost,
    #[error("unknown extension name: {0}")]
    UnknownExtension(String),
}
