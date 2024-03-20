use super::TermPoll;
use crate::telemetry::postgres::PostgresDirectoryStore;
use crate::telemetry::posthog::PosthogStore;
use crate::telemetry::{TelemetryController, TelemetrySender};
use pgrx::bgworkers::{self, BackgroundWorker, BackgroundWorkerBuilder, SignalWakeFlags};
use pgrx::{pg_guard, pg_sys, FromDatum, IntoDatum};
use std::process;
use std::time::Duration;

pub enum ParadeExtension {
    PgSearch = 1,
    PgAnalytics = 2,
}

impl ParadeExtension {
    fn name(&self) -> String {
        match self {
            Self::PgSearch => "pg_bm25",
            Self::PgAnalytics => "pg_analytics",
        }
        .into()
    }

    fn from_i32(n: i32) -> Option<Self> {
        match n {
            1 => Some(Self::PgSearch),
            2 => Some(Self::PgAnalytics),
            _ => None,
        }
    }
}

pub struct SigtermHandler {}

impl SigtermHandler {
    fn new() -> Self {
        BackgroundWorker::attach_signal_handlers(
            SignalWakeFlags::SIGHUP | SignalWakeFlags::SIGTERM,
        );
        Self {}
    }
}

impl TermPoll for SigtermHandler {
    fn term_poll(&self) -> bool {
        BackgroundWorker::sigterm_received() || BackgroundWorker::sighup_received()
    }
}

#[pg_guard]
#[no_mangle]
pub fn setup_telemetry_background_worker(extension: ParadeExtension) {
    // A background worker to read and send telemetry data to PostHog.
    BackgroundWorkerBuilder::new(&format!("{}_telemetry_worker", extension.name()))
        // Must be the name of a function in this file.
        .set_function("telemetry_worker")
        // Must be the name of the extension it will be loaded from.
        .set_library(&extension.name())
        // We pass the extension name to retrieve the associated data directory to read telemetry data from.
        .set_argument((extension as i32).into_datum())
        // Necessary for using plog!.
        // Also, it doesn't seem like bgworkers will start without this.
        .enable_spi_access()
        // RecoveryFinished is the last available stage for bgworker startup.
        // We wait until as late as possible so that we can make sure the
        // paradedb.logs table is created, for the sake of using plog!.
        .set_start_time(bgworkers::BgWorkerStartTime::RecoveryFinished)
        .load();
}

#[pg_guard]
#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn telemetry_worker(extension_name_datum: pg_sys::Datum) {
    let extension_i32 = unsafe { i32::from_datum(extension_name_datum, false) }
        .expect("extension enum i32 not passed to bgworker");
    let extension = ParadeExtension::from_i32(extension_i32).unwrap_or_else(|| panic!("unexpected extension i32 passed to bgworker {extension_i32}"));
    let extension_name = extension.name();

    pgrx::log!(
        "starting {extension_name} telemetry worker at PID {}",
        process::id()
    );

    // These are the signals we want to receive. If we don't attach the SIGTERM handler, then
    // we'll never be able to exit via an external notification.
    let sigterm_handler = SigtermHandler::new();
    let telemetry_store = match PosthogStore::from_env().map(Box::new) {
        Ok(store) => store,
        Err(err) => {
            pgrx::warning!("could not initialize posthog, exiting telemetry worker: {err}");
            return;
        }
    };
    let directory_store = PostgresDirectoryStore::new(&extension_name)
        .map(Box::new)
        .unwrap();
    let sender = TelemetrySender {
        extension_name: extension_name.to_string(),
        telemetry_store,
        directory_store,
    };
    let controller = TelemetryController {
        sender,
        directory_check_interval: Duration::from_secs(30), // 12 hours
        // directory_check_interval: Duration::from_secs(12 * 3600), // 12 hours
        sleep_interval: Duration::from_secs(1),
        term_poll: Box::new(sigterm_handler),
    };

    pgrx::log!("starting {extension_name} telemetry event loop");
    controller.run().unwrap()
}
