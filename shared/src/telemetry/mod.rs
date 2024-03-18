mod bgworker;
mod data;
mod posthog;

pub use bgworker::setup_telemetry_background_worker;
pub use posthog::PosthogClient;
use std::{env::VarError, str::Utf8Error};
use thiserror::Error;
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

pub trait HttpClient {
    fn post(&self, url: String, body: String) -> Result<(), TelemetryError>;
}

// Implement the trait for `reqwest::blocking::Client`
impl HttpClient for reqwest::blocking::Client {
    fn post(&self, url: String, body: String) -> Result<(), TelemetryError> {
        self.post(url)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .map(|_| ())
            .map_err(TelemetryError::from)
    }
}
