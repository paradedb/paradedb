// Copyright (c) 2023-2026 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

//! Tolerance for transient database connectivity faults.
//!
//! Under Antithesis the `paradedb` container is stopped, killed, and network-
//! partitioned mid-run. This module classifies an error as either a *transient*
//! connectivity fault (which the workload should ride out by reconnecting) or a
//! *real* logical/SQL error (which must surface), and provides [`tolerate_transient`]
//! to retry an operation through transient faults for a bounded grace window.
//!
//! When this is enabled, bug detection is expected to come from Antithesis properties / postgres-side
//! checks, not from stressgres exit codes.

use anyhow::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Backoff bounds for retries within the reconnect grace window.
const INITIAL_RECONNECT_BACKOFF: Duration = Duration::from_millis(500);
const MAX_RECONNECT_BACKOFF: Duration = Duration::from_secs(5);

/// Substrings that identify a transient connectivity failure when all we have is a
/// stringified error (e.g. one already flattened by `format_postgres_error`).
const TRANSIENT_ERROR_NEEDLES: &[&str] = &[
    "network is unreachable",
    "no route to host",
    "connection refused",
    "connection reset",
    "connection closed",
    "broken pipe",
    "server closed the connection",
    "terminating connection",
    "error connecting to server",
    "error communicating with the server",
    "error performing tls handshake",
    "unexpected eof",
    "os error 101",     // ENETUNREACH
    "(sqlstate: 08",    // connection exception class
    "(sqlstate: 57p01", // admin_shutdown
    "(sqlstate: 57p02", // crash_shutdown
    "(sqlstate: 57p03", // cannot_connect_now
];

fn message_looks_transient(msg: &str) -> bool {
    let msg = msg.to_ascii_lowercase();
    TRANSIENT_ERROR_NEEDLES
        .iter()
        .any(|needle| msg.contains(needle))
}

/// Classifies a `postgres::Error` as a transient connectivity failure (as opposed
/// to a logical/SQL error, which represents a real bug we want to surface).
///
/// The discriminator is whether the error carries a SQLSTATE, not what its message
/// says. A SQLSTATE means the statement actually reached the server and it answered,
/// so only the connection-class codes are transient and everything else is a real
/// logical/SQL error. No SQLSTATE means the failure happened in the client/transport
/// before the server answered — connect refused/reset, socket dropped, a TLS
/// handshake against a server that is down or restarting, protocol desync on a dying
/// connection — which under fault injection are exactly the faults we ride out. This
/// keys off the transport-vs-server distinction rather than string-matching each new
/// libpq/driver phrasing (e.g. "error performing TLS handshake", which has no
/// SQLSTATE and no `is_closed()` signal, so needle matching alone would miss it).
fn is_transient_connection_error(e: &postgres::Error) -> bool {
    match e.as_db_error() {
        Some(db) => {
            let code = db.code().code();
            // Class 08 = connection exception; 57P0x = operator/crash shutdown and
            // "cannot connect now" (server starting up / shutting down).
            code.starts_with("08") || matches!(code, "57P01" | "57P02" | "57P03" | "57P05")
        }
        // Client-side/transport failure: no answer from the server, so it never got
        // far enough to be a logical bug. Treat it as transient.
        None => true,
    }
}

/// Classifies an `anyhow::Error` as a transient connectivity failure by walking its
/// cause chain for a `postgres::Error`/IO error, falling back to string matching for
/// errors that have already been flattened to a message.
fn is_transient_error(err: &anyhow::Error) -> bool {
    for cause in err.chain() {
        if let Some(pg) = cause.downcast_ref::<postgres::Error>() {
            return is_transient_connection_error(pg);
        }
        if let Some(pg) = cause.downcast_ref::<Arc<postgres::Error>>() {
            return is_transient_connection_error(pg);
        }
        if cause.downcast_ref::<std::io::Error>().is_some() {
            return true;
        }
    }
    message_looks_transient(&err.to_string())
}

fn interruptible_sleep(alive: &AtomicBool, dur: Duration) {
    let start = Instant::now();
    while start.elapsed() < dur {
        if !alive.load(Ordering::Relaxed) {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[derive(Default)]
pub(crate) struct TransientProgress {
    recovered: bool,
}

impl TransientProgress {
    /// Mark that this retry attempt reached the database. If the attempt later
    /// fails with another transient error, it starts a fresh grace window.
    pub(crate) fn mark_recovered(&mut self) {
        self.recovered = true;
    }
}

/// Runs `op`, tolerating transient connectivity faults for up to `grace`: while `op`
/// keeps failing with a transient error (a dropped/refused socket, server restarting,
/// etc.) it is retried with capped backoff. A real (non-transient) error is returned
/// immediately. The connection is only declared dropped — and the error surfaced —
/// once it has stayed broken for `grace` continuously. If `op` calls
/// [`TransientProgress::mark_recovered`] before returning a later transient error,
/// the grace window is restarted because the database recovered between faults.
///
/// With `grace == 0` (the default outside Antithesis) any error fails immediately,
/// preserving the historical "an error fails the run" behaviour.
///
/// This is the single place reconnection lives. Callers express *what* to do (probe
/// the version, run setup, run one job iteration); reopening a dead connection is
/// the caller's job inside `op`, and re-running the whole `op` is what makes this
/// safe for transactional work — a lost transaction is simply replayed from scratch.
///
/// Returns `Ok(None)` if `alive` went false while we were waiting out a fault.
pub(crate) fn tolerate_transient<T>(
    alive: &AtomicBool,
    grace: Duration,
    mut op: impl FnMut(&mut TransientProgress) -> Result<T>,
) -> Result<Option<T>> {
    let mut backoff = INITIAL_RECONNECT_BACKOFF;
    let mut down_since: Option<Instant> = None;
    loop {
        let mut progress = TransientProgress::default();
        match op(&mut progress) {
            Ok(value) => return Ok(Some(value)),
            Err(e) if !is_transient_error(&e) => return Err(e),
            Err(e) => {
                if !alive.load(Ordering::Relaxed) {
                    return Ok(None);
                }
                if progress.recovered {
                    down_since = None;
                    backoff = INITIAL_RECONNECT_BACKOFF;
                }
                let down_since = *down_since.get_or_insert_with(Instant::now);
                if down_since.elapsed() >= grace {
                    // Grace window exhausted (immediate when grace == 0): the fault is
                    // no longer "transient" as far as the run is concerned and we should fail.
                    return Err(if grace.is_zero() {
                        e
                    } else {
                        e.context(format!(
                            "database unreachable for {:?}, past the {grace:?} grace window",
                            down_since.elapsed()
                        ))
                    });
                }
                eprintln!("stressgres: transient database fault, retrying: {e:#}");
                interruptible_sleep(alive, backoff);
                backoff = (backoff * 2).min(MAX_RECONNECT_BACKOFF);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;
    use std::io;

    #[test]
    fn message_matching_flags_connectivity_faults() {
        for msg in [
            "error connecting to server: Network is unreachable (os error 101)",
            "server closed the connection unexpectedly",
            "connection reset by peer",
            "db error: FATAL: terminating connection due to administrator command (SQLState: 57P01)",
            "error: could not receive data from server (SQLState: 08006)",
            // No SQLSTATE, no `is_closed()` signal: only the transport-level phrasing
            // marks this as connectivity noise (server down/restarting mid-handshake).
            "error performing TLS handshake: unexpected EOF",
        ] {
            assert!(message_looks_transient(msg), "should be transient: {msg}");
        }
    }

    #[test]
    fn message_matching_ignores_real_errors() {
        for msg in [
            "ERROR: division by zero (SQLState: 22012)",
            "duplicate key value violates unique constraint (SQLState: 23505)",
            "ERROR: relation \"foo\" does not exist (SQLState: 42P01)",
            "Job assertion failed: expected 5 but got 3",
        ] {
            assert!(!message_looks_transient(msg), "should be real: {msg}");
        }
    }

    #[test]
    fn anyhow_string_errors_are_classified() {
        assert!(is_transient_error(&anyhow!(
            "error connecting to server: Network is unreachable (os error 101)"
        )));
        assert!(!is_transient_error(&anyhow!(
            "Job assertion failed: expected 5 but got 3"
        )));
    }

    #[test]
    fn io_errors_are_transient() {
        let err = anyhow::Error::new(io::Error::new(io::ErrorKind::ConnectionReset, "reset"));
        assert!(is_transient_error(&err));
    }

    #[test]
    fn recovery_marker_restarts_grace_window() {
        let alive = AtomicBool::new(true);
        let mut attempts = 0;

        let result = tolerate_transient(&alive, Duration::from_millis(250), |progress| {
            attempts += 1;
            match attempts {
                1 => Err(anyhow!("connection closed")),
                2 => {
                    progress.mark_recovered();
                    Err(anyhow!("connection closed"))
                }
                _ => Ok("ok"),
            }
        })
        .unwrap();

        assert_eq!(result, Some("ok"));
        assert_eq!(attempts, 3);
    }
}
