use std::{
    thread,
    time::{Duration, Instant},
};

use super::{
    event::TelemetryEvent, DirectoryStore, TelemetryConfigStore, TelemetryError, TelemetryStore,
    TermPoll,
};

pub struct TelemetrySender {
    pub directory_store: Box<dyn DirectoryStore<Error = TelemetryError>>,
    pub telemetry_store: Box<dyn TelemetryStore<Error = TelemetryError>>,
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
        let event = TelemetryEvent::Deployment {
            extension: self.config_store.extension_name()?,
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
            extension: self.config_store.extension_name()?,
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
