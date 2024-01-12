#![allow(dead_code, unused_variables)]
mod api;
mod env;
mod index_access;
mod json;
mod operator;
mod parade_index;
mod tokenizers;
mod writer;

use parade_writer::WriterStatus;
use pgrx::bgworkers::{BackgroundWorker, BackgroundWorkerBuilder, SignalWakeFlags};
use pgrx::*;
use shared::logs::ParadeLogsGlobal;
use shared::telemetry;
use std::thread;
use std::time::Duration;

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
    pgrx::log!("starting insert worker");
    let writer = writer::Writer::new();
    let mut server = writer::Server::new(writer).expect("error starting writer server");

    pgrx::log!("setting insert worker addr");
    // Retrieve the assigned port and assign to global state.
    WRITER_STATUS.exclusive().set_addr(server.addr());

    // Handle an edge case where Postgres has been shut down very quickly. In this case, the
    // shutdown worker will have already sent the shutdown message, but we haven't started the
    // server, so we'll have missed it. We should check ourselves for the SIGTERM signal, and
    // just shutdown early if it's been received.
    if BackgroundWorker::sigterm_received() {
        log!("insert worker received sigterm before starting server, shutting down early");
        return;
    }

    pgrx::log!("starting insert worker server");
    server.start().expect("writer server crashed");
}

// #[pg_guard]
// #[no_mangle]
// pub extern "C" fn pg_bm25_insert_worker(_arg: pg_sys::Datum) {
//     // Bind to port 0 to let the OS choose a free port.
//     // Check if there was an error starting the server, and return early if so.
//     let mut socket_addr: Option<SocketAddr> = None;
//     let server = {
//         // Note that we do not derefence the WRITER to mutate it, due to PGRX shared struct rules.
//         // We also acquire its lock with `.exclusive` inside an enclosing block to ensure that
//         // it is dropped after we are done with it.
//         let mut writer_status = WRITER_STATUS.exclusive();
//         match Server::http("0.0.0.0:0") {
//             Err(error) => {
//                 log!("error starting pg_bm25 server: {error:?}");
//                 writer_status.set_error(WriterInitError::ServerBindError);
//                 return;
//             }
//             Ok(server) => {
//                 match server.server_addr() {
//                     // We want to set the socket address on the addr field of the writer client,
//                     // so that connection processes that share it know where to send their requests.
//                     tiny_http::ListenAddr::IP(addr) => {
//                         socket_addr.replace(addr);
//                         writer_status.set_addr(addr);
//                     }
//                     // It's not clear when tiny_http would choose to use a Unix socket address,
//                     // but we have to handle the enum variant, so we'll consider this outcome
//                     // an irrecovereable error, although its not expected to happen.
//                     tiny_http::ListenAddr::Unix(_) => {
//                         writer_status.set_error(WriterInitError::ServerUnixPortError);
//                         log!("paradedb bm25 writer started server with a unix port, which is not supported");
//                         return;
//                     }
//                 };
//                 server
//             }
//         }
//     };

// // Retrieve the assigned port and assign to global state.

// // Handle an edge case where Postgres has been shut down very quickly. In this case, the
// // shutdown worker will have already sent the shutdown message, but we haven't started the
// // server, so we'll have missed it. We should check ourselves for the SIGTERM signal, and
// // just shutdown early if it's been received.
// if BackgroundWorker::sigterm_received() {
//     log!("insert worker received sigterm before starting server, shutting down early");
//     return;
// }

//     // We initialized the tiny_http server above, which is the actual HTTP server that reads
//     // requests. The ParadeWriterServer is more like the "server implementation", and actually
//     // handles the requests and produces responses.
//     let mut writer_server = ParadeWriterServer::new();

//     pgrx::log!(
//         "initialized writer server at {}",
//         socket_addr.map(|a| a.to_string()).unwrap_or("".into())
//     );
//     for mut request in server.incoming_requests() {
//         let writer_request = ParadeWriterRequest::try_from(&mut request);
//         pgrx::log!("got writer request: {writer_request:?}");
//         match &writer_request {
//             // Handle any kind of error parsing the request.
//             Err(e) => {
//                 pgrx::log!("unexpecting error on writer server while parsing client request");
//                 let response =  ParadeWriterResponse::Error(format!("error parsing parade writer request: {e:?}"));
//                 request
//                     .respond(Response::from_data(response))
//                     .unwrap_or_else(|e| log!("parade index writer encountered an unexpected error responding to client: {e:?}"));
//             }
//             // The expected path, the request was successfully parsed and we delegate to the
//             // server instance to handle it.
//             Ok(req) => writer_server.handle(req, |response| {
//                 request
//                     .respond(Response::from_data(response))
//                     .unwrap_or_else(|e| log!("parade index writer encountered an unexpected error responding to client: {e:?}"));
//             }),
//         };

//         if writer_server.should_exit() {
//             log!("pg_bm25 server received shutdown request, shutting down.");
//             return;
//         }
//     }
// }

#[pg_guard]
#[no_mangle]
pub extern "C" fn pg_bm25_shutdown_worker(_arg: pg_sys::Datum) {
    // These are the signals we want to receive.  If we don't attach the SIGTERM handler, then
    // we'll never be able to exit via an external notification.
    BackgroundWorker::attach_signal_handlers(SignalWakeFlags::SIGTERM);

    log!("started shutdown background worker");
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
