use crate::telemetry::postgres::PostgresDirectoryStore;
use crate::telemetry::posthog::PosthogStore;
use crate::telemetry::{TelemetryController, TelemetrySender};
use pgrx::bgworkers::{self, BackgroundWorker, BackgroundWorkerBuilder, SignalWakeFlags};
use pgrx::{pg_guard, pg_sys, IntoDatum};
use std::ffi::CStr;
use std::process;
use std::time::Duration;

use super::{TelemetryError, TermPoll};

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
pub fn setup_telemetry_background_worker(extension_name: &str) {
    // A background worker to read and send telemetry data to PostHog.
    BackgroundWorkerBuilder::new(&format!("{}_telemetry_worker", extension_name))
        // Must be the name of a function in this file.
        .set_function("telemetry_worker")
        // Must be the name of the extension it will be loaded from.
        .set_library(extension_name)
        // We pass the extension name to retrieve the associated data directory to read telemetry data from.
        .set_argument(extension_name.into_datum())
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
    let extension_name = detoast_string(extension_name_datum).expect("Failed to convert to string");
    pgrx::log!(
        "starting {extension_name} telemetry worker at PID {}",
        process::id()
    );

    // These are the signals we want to receive. If we don't attach the SIGTERM handler, then
    // we'll never be able to exit via an external notification.
    let sigterm_handler = SigtermHandler::new();
    let telemetry_store = PosthogStore::from_env().map(Box::new).unwrap();
    let directory_store = PostgresDirectoryStore::new(&extension_name)
        .map(Box::new)
        .unwrap();
    let sender = TelemetrySender {
        telemetry_store,
        directory_store,
    };
    let controller = TelemetryController {
        sender,
        directory_check_interval: Duration::from_secs(12 * 3600), // 12 hours
        sleep_interval: Duration::from_secs(1),
        term_poll: Box::new(sigterm_handler),
    };

    pgrx::log!("starting {extension_name} telemetry event loop");
    controller.run().unwrap()
}

fn detoast_string(datum: pg_sys::Datum) -> Result<String, TelemetryError> {
    // Convert Datum to CString
    let c_str = unsafe {
        let text_ptr =
            pg_sys::pg_detoast_datum(datum.cast_mut_ptr::<pg_sys::varlena>()) as *mut pg_sys::text;
        CStr::from_ptr(pg_sys::text_to_cstring(text_ptr))
    };
    // Convert CStr to Rust String
    c_str
        .to_str()
        .map(|s| s.to_string())
        .map_err(TelemetryError::DetoastExtensionName)
}
