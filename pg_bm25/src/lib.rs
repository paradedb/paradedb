#![allow(unused_imports, unused_variables, dead_code, unreachable_code)]
use core::fmt;
use std::error::Error;
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::str::Utf8Error;
use std::time::Duration;
use std::{default, thread};

use http::{Method, Request};
use parade_index::index::ParadeIndexKey;
use parade_writer::ParadeWriter;
use pgrx::bgworkers::{
    BackgroundWorker, BackgroundWorkerBuilder, BackgroundWorkerStatus, SignalWakeFlags,
};
use pgrx::*;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use shared::logs::ParadeLogsGlobal;
use shared::{plog, telemetry};
use tiny_http::{Response, Server};

use crate::parade_writer::error::WriterError;
use crate::parade_writer::io::{ParadeWriterRequest, ParadeWriterResponse};

mod api;
mod index_access;
mod json;
mod operator;
mod parade_index;
mod parade_writer;
mod tokenizers;

// This is a flag that can be set by the user in a session to enable logs.
// You need to initialize this in every extension that uses `plog!`.
static PARADE_LOGS_GLOBAL: ParadeLogsGlobal = ParadeLogsGlobal::new("pg_bm25");

// This is global shared state for the writer background worker.
static WRITER: PgLwLock<ParadeWriter> = PgLwLock::new();

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
    // pg_shmem_init!(WRITER_SERVER_ADDRESS);
    // pg_shmem_init!(WRITER_INIT_ERROR);
    pg_shmem_init!(WRITER);

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
}

#[pg_guard]
#[no_mangle]
pub extern "C" fn pg_bm25_insert_worker(_arg: pg_sys::Datum) {
    // Bind to port 0 to let the OS choose a free port.
    let server_result = Server::http("0.0.0.0:0");
    // Check if there was an error starting the server, and return early if so.
    if let Err(server_error) = &server_result {
        log!("error starting pg_bm25 server: {server_error:?}");
        WRITER.exclusive().set_error(WriterError::ServerBindError);
        return;
    }

    // We've checked above that there's no error, so it's safe to unwrap.
    let server = server_result.unwrap();

    // Retrieve the assigned port and assign to global state.
    match server.server_addr() {
        // Note that we do not derefence the WRITER to mutate it, due to PGRX shared struct rules.
        tiny_http::ListenAddr::IP(addr) => {
            WRITER.exclusive().set_addr(addr);
        }
        tiny_http::ListenAddr::Unix(_) => {
            WRITER
                .exclusive()
                .set_error(WriterError::ServerUnixPortError);
            log!("paradedb bm25 writer started server with a unix port, which is not supported");
            return;
        }
    };

    // Handle an edge case where Postgres has been shut down very quickly. In this case, the
    // shutdown worker will have already sent the shutdown message, but we haven't started the
    // server, so we'll have missed it. We should check ourselves for the SIGTERM signal, and
    // just shutdown early if it's been received.
    if BackgroundWorker::sigterm_received() {
        log!("insert worker received sigterm before starting server, shutting down early");
        return;
    }

    // We'll set a should_exit flag instead of just returing early so that we can
    // send an acknowledgement to the client.
    let mut should_exit = false;
    for mut request in server.incoming_requests() {
        let response = match ParadeWriterRequest::try_from(&mut request) {
            Err(e) => ParadeWriterResponse::RequestParseError(e),
            Ok(ParadeWriterRequest::Shutdown) => {
                should_exit = true;
                ParadeWriterResponse::ShutdownOk
            }
            Ok(t) => {
                log!("server received unimplemented type: {t:?}");
                unimplemented!("");
            }
        };

        request
            .respond(Response::from_data(response))
            .unwrap_or_else(|e| log!("unexpected error responding to client: {e:?}"));

        if should_exit {
            log!("pg_bm25 server received shutdown request, shutting down.");
            return;
        }
    }
}

#[pg_guard]
#[no_mangle]
pub extern "C" fn pg_bm25_shutdown_worker(_arg: pg_sys::Datum) {
    log!("started shutdown background worker");
    // These are the signals we want to receive.  If we don't attach the SIGTERM handler, then
    // we'll never be able to exit via an external notification.
    BackgroundWorker::attach_signal_handlers(SignalWakeFlags::SIGTERM);

    // Check every second to see if we've received SIGTERM.
    while BackgroundWorker::wait_latch(Some(Duration::from_secs(1))) {}

    // We've received SIGTERM. Send a shutdown message to the HTTP server.
    let writer = *WRITER.share();
    writer
        .shutdown()
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
        vec![]
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
