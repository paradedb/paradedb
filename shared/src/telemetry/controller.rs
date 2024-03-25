use pgrx::pg_sys::{PG_MAJORVERSION_NUM, PG_MINORVERSION_NUM};
use std::{
    thread,
    time::{Duration, Instant, SystemTime},
};

use super::{
    event::TelemetryEvent, DirectoryStore, TelemetryConfigStore, TelemetryError, TelemetryStore,
    TermPoll,
};

pub struct TelemetrySender {
    pub directory_store: Box<dyn DirectoryStore>,
    pub telemetry_store: Box<dyn TelemetryStore>,
    pub config_store: Box<dyn TelemetryConfigStore>,
}

impl TelemetrySender {
    pub fn send(&self, uuid: &str, event: &TelemetryEvent) -> Result<(), TelemetryError> {
        let conn = self.telemetry_store.get_connection()?;
        if self.config_store.telemetry_enabled()? {
            conn.send(uuid, event)
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
        let os_info = os_info::get();
        let event = TelemetryEvent::Deployment {
            timestamp: SystemTime::now(),
            arch: os_info.architecture().unwrap_or_default().to_string(),
            extension_name: self.config_store.extension_name()?,
            extension_version: env!("CARGO_PKG_VERSION").to_string(),
            os_type: os_info.os_type().to_string(),
            os_version: os_info.version().to_string(),
            postgres_version: format!("{PG_MAJORVERSION_NUM}.{PG_MINORVERSION_NUM}"),
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
            extension_name: self.config_store.extension_name()?,
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
