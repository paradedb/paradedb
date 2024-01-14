mod api;
mod env;
mod index_access;
mod operator;
mod parade_index;
mod tokenizers;
mod writer;

use pgrx::bgworkers::{BackgroundWorker, BackgroundWorkerBuilder, SignalWakeFlags};
use pgrx::*;
use shared::logs::ParadeLogsGlobal;
use shared::telemetry;
use std::net::SocketAddr;
use std::time::Duration;
use std::{process, thread};

#[derive(Copy, Clone, Default)]
pub struct WriterStatus {
    pub addr: Option<SocketAddr>,
}

impl WriterStatus {
    pub fn addr(&self) -> SocketAddr {
        self.addr
            .expect("could not access writer status, writer server may not have started.")
    }
    pub fn set_addr(&mut self, addr: SocketAddr) {
        self.addr = Some(addr);
    }
}

unsafe impl PGRXSharedMemory for WriterStatus {}

// This is a flag that can be set by the user in a session to enable logs.
// You need to initialize this in every extension that uses `plog!`.
static PARADE_LOGS_GLOBAL: ParadeLogsGlobal = ParadeLogsGlobal::new("pg_bm25");

// This is global shared state for the writer background worker.
static WRITER_STATUS: PgLwLock<WriterStatus> = PgLwLock::new();

pgrx::pg_module_magic!();

extension_sql_file!("../sql/_bootstrap.sql");

// Initializes option parsing and telemetry
#[allow(clippy::missing_safety_doc)]
#[allow(non_snake_case)]
#[pg_guard]
pub unsafe extern "C" fn _PG_init() {
    index_access::options::init();
    telemetry::posthog::init("pg_bm25");
    PARADE_LOGS_GLOBAL.init();

    // Set up the writer bgworker shared satate.
    pg_shmem_init!(WRITER_STATUS);

    // We call this in a helper function to the bgworker initialization
    // can be used in test suites.
    setup_background_workers();
}

#[pg_guard]
pub fn setup_background_workers() {
    // A background worker to perform the insert work for the Tantivy index.
    BackgroundWorkerBuilder::new("pg_bm25_insert_worker")
        // Must be the name of a function in this file.
        .set_function("pg_bm25_insert_worker")
        // Must be the name of this library.
        .set_library("pg_bm25")
        // The argument will be unused. You just need to pass something.
        .set_argument(0.into_datum())
        // Necessary for using plog!.
        // Also, it doesn't seem like bgworkers will start without this.
        .enable_spi_access()
        // RecoveryFinished is the last available stage for bgworker startup.
        // We wait until as late as possible so that we can make sure the
        // paradedb.logs table is created, for the sake of using plog!.
        .set_start_time(bgworkers::BgWorkerStartTime::RecoveryFinished)
        .load();

    // A background worker with the job of shutting down the insert worker.
    // The insert worker cannot efficiently check for shutdown signals as well
    // as waiting for incoming http requests, so we start a second worker
    // who will listen for Postgres shutdown signals, and then send a special
    // HTTP request to the insert background worker, allowing it to shut down.
    BackgroundWorkerBuilder::new("pg_bm25_shutdown_worker")
        // Must be the name of a function in this file.
        .set_function("pg_bm25_shutdown_worker")
        // Must be the name of this library.
        .set_library("pg_bm25")
        // The argument will be unused. You just need to pass something.
        .set_argument(0.into_datum())
        // Necessary for using plog!.
        // Also, it doesn't seem like bgworkers will start without this.
        .enable_spi_access()
        .load();

    // Add a short delay to allow the HTTP server to start. This is a temporary
    // fix for the sake of the test suite. We should add a specific lock for this.
    thread::sleep(Duration::from_millis(1000));
}

#[pg_guard]
#[no_mangle]
pub extern "C" fn pg_bm25_insert_worker(_arg: pg_sys::Datum) {
    pgrx::log!("starting pg_bm25 insert worker at PID {}", process::id());
    let writer = writer::Writer::new();
    let mut server = writer::Server::new(writer).expect("error starting writer server");

    // Retrieve the assigned port and assign to global state.
    // Note that we do not derefence the WRITER to mutate it, due to PGRX shared struct rules.
    // We also acquire its lock with `.exclusive` inside an enclosing block to ensure that
    // it is dropped after we are done with it.
    {
        WRITER_STATUS.exclusive().set_addr(server.addr());
    }

    // Handle an edge case where Postgres has been shut down very quickly. In this case, the
    // shutdown worker will have already sent the shutdown message, but we haven't started the
    // server, so we'll have missed it. We should check ourselves for the SIGTERM signal, and
    // just shutdown early if it's been received.
    if BackgroundWorker::sigterm_received() {
        log!("insert worker received sigterm before starting server, shutting down early");
        return;
    }

    server.start().expect("writer server crashed");
}

#[pg_guard]
#[no_mangle]
pub extern "C" fn pg_bm25_shutdown_worker(_arg: pg_sys::Datum) {
    pgrx::log!("starting pg_bm25 shutdown worker at PID {}", process::id());
    // These are the signals we want to receive.  If we don't attach the SIGTERM handler, then
    // we'll never be able to exit via an external notification.
    BackgroundWorker::attach_signal_handlers(SignalWakeFlags::SIGTERM);

    // Check every second to see if we've received SIGTERM.
    while BackgroundWorker::wait_latch(Some(Duration::from_secs(1))) {}

    // We've received SIGTERM. Send a shutdown message to the HTTP server.
    let mut writer_client: writer::Client<writer::WriterRequest> =
        writer::Client::new(WRITER_STATUS.share().addr());

    writer_client
        .stop_server()
        .unwrap_or_else(|e| log!("error shutting down bm25 writer from background worker: {e:?}"));
}

/// This module is required by `cargo pgrx test` invocations.
/// It must be visible at the root of your extension crate.
#[cfg(test)]
pub mod pg_test {
    pub fn setup(_options: Vec<&str>) {
        // perform one-off initialization when the pg_test framework starts
    }

    pub fn postgresql_conf_options() -> Vec<&'static str> {
        // return any postgresql.conf settings that are required for your tests
        vec!["shared_preload_libraries='pg_bm25.so'"]
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    #[pgrx::pg_test]
    fn test_parade_logs() {
        shared::test_plog!("pg_bm25");
    }
}
