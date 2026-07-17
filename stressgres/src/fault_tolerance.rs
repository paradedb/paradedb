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
//! Under deterministic simulation testing the `paradedb` container is stopped, killed, and
//! network-partitioned mid-run. This module classifies an error as either a *transient*
//! connectivity fault (which the workload should ride out by reconnecting) or a
//! *real* logical/SQL error (which must surface), and provides [`tolerate_transient`]
//! to retry an operation through transient faults for a bounded grace window.
//!
//! The grace defaults to zero, under which any error fails the run, so this is inert
//! unless a caller opts in with `--reconnect-grace`.
//!
//! Under fault injection the grace should be set larger than the run itself, so a connectivity
//! fault can never fail the run: the DST harness searches for the fault schedule that breaks
//! us, so any window shorter than the run is one it can outlast. Liveness is instead
//! asserted from the outside: the suites' recovery-liveness command heals every
//! fault, then narrows the [`GraceWindow`] via its poke file to a window that *can*
//! expire. See `stressgres/suites/antithesis/`.
//!
//! When a non-zero grace is enabled, bug detection is expected to come from the DST harness's
//! properties / postgres-side checks, not from stressgres exit codes.

use anyhow::Result;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Backoff bounds for retries within the reconnect grace window.
const INITIAL_RECONNECT_BACKOFF: Duration = Duration::from_millis(500);
const MAX_RECONNECT_BACKOFF: Duration = Duration::from_secs(5);

/// How long a continuous transient fault may last before the run fails.
///
/// The window has a `baseline`, fixed at startup, and an optional `file` that overrides
/// it for as long as the file exists — the poke channel an external supervisor uses to
/// narrow the window at runtime (see the module docs). Deleting the file restores the
/// baseline, so the supervisor never has to know what it was.
///
/// It is only read on the error path, so a healthy run never touches it.
#[derive(Clone, Debug)]
pub(crate) struct GraceWindow {
    baseline: Duration,
    file: Option<PathBuf>,
}

impl GraceWindow {
    /// A window that never changes. `Duration::ZERO` is the default (fail on the first
    /// error).
    pub(crate) fn fixed(baseline: Duration) -> Self {
        Self {
            baseline,
            file: None,
        }
    }

    /// A window that `file` may narrow or widen at any time. A missing, unreadable, or
    /// malformed file falls back to `baseline` — a supervisor that has not run yet, or
    /// a half-written file, must not be able to fail the run.
    pub(crate) fn pokeable(baseline: Duration, file: PathBuf) -> Self {
        Self {
            baseline,
            file: Some(file),
        }
    }

    /// Parses a count of milliseconds, as written to the poke file.
    fn parse_ms(s: &str) -> Result<Duration> {
        let s = s.trim();
        Ok(Duration::from_millis(s.parse::<u64>().map_err(|_| {
            anyhow::anyhow!("expected a count of milliseconds, got `{s}`")
        })?))
    }

    /// The window in force right now.
    fn current(&self) -> Duration {
        let Some(file) = self.file.as_ref() else {
            return self.baseline;
        };
        match std::fs::read_to_string(file) {
            Ok(contents) => Self::parse_ms(&contents).unwrap_or_else(|e| {
                eprintln!(
                    "stressgres: ignoring malformed grace file {}: {e:#}",
                    file.display()
                );
                self.baseline
            }),
            // Absent (the supervisor has not run, or has restored the baseline), or we
            // lost a race with its atomic rename.
            Err(_) => self.baseline,
        }
    }
}

/// Substrings that identify a transient connectivity failure when all we have is a
/// stringified error (e.g. one already flattened by `format_postgres_error`).
///
/// The `(sqlstate: ...` needles must mirror the connection-class codes in
/// [`is_transient_connection_error`]; this is the fallback for errors that have lost
/// their structured `postgres::Error`, so keep the two lists in sync.
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
    "(sqlstate: 57p05", // idle_session_timeout
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
            // Class 08 = connection exception; 57P0x = operator/crash shutdown,
            // "cannot connect now" (server starting up / shutting down), and idle-session
            // timeout — all cases where the connection is gone and reconnecting is right.
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

/// Runs `op`, tolerating transient connectivity faults for as long as `grace` allows:
/// while `op` keeps failing with a transient error (a dropped/refused socket, server
/// restarting, etc.) it is retried with capped backoff. A real (non-transient) error is
/// returned immediately. The fault is only surfaced once the connection has stayed
/// broken for the whole grace window continuously.
///
/// The window is re-read on every failed attempt, and the clock restarts whenever it
/// changes or `op` calls [`TransientProgress::mark_recovered`]. That reset is what lets
/// an external poke narrow the window mid-fault: it asks "can you reconnect within N
/// seconds of now", not "were you already down N seconds ago".
///
/// Reopening a dead connection is the caller's job inside `op`; re-running the whole
/// `op` is what keeps this safe for transactional work — a lost transaction is replayed.
///
/// Returns `Ok(None)` if `alive` went false while we were waiting out a fault.
pub(crate) fn tolerate_transient<T>(
    alive: &AtomicBool,
    grace: &GraceWindow,
    mut op: impl FnMut(&mut TransientProgress) -> Result<T>,
) -> Result<Option<T>> {
    let mut backoff = INITIAL_RECONNECT_BACKOFF;
    let mut down_since: Option<Instant> = None;
    let mut last_grace: Option<Duration> = None;
    loop {
        let mut progress = TransientProgress::default();
        match op(&mut progress) {
            Ok(value) => return Ok(Some(value)),
            Err(e) if !is_transient_error(&e) => return Err(e),
            Err(e) => {
                let grace = grace.current();
                // Checked before `alive`, so that a zero grace fails the run on the
                // first error even during shutdown. With a real grace we would rather
                // abandon a fault we are waiting out than block teardown for the whole
                // window.
                if grace.is_zero() {
                    return Err(e);
                }
                if !alive.load(Ordering::Relaxed) {
                    return Ok(None);
                }

                // A narrower window means the supervisor healed the faults and started
                // the recovery clock while we were down, which is the only situation in
                // which the liveness check can actually fail. If the harness never
                // reaches it, the check passed vacuously and proved nothing.
                crate::dst::poke_narrowed_window!(last_grace.is_some_and(|last| grace < last));
                // No symmetric "widened during a fault" assertion: the only widening is the
                // supervisor restoring the baseline, which happens deep in the healed quiet
                // period with no active fault to observe it. That path is pinned by the
                // `widening_the_window_also_restarts_the_clock` test instead.
                if progress.recovered || last_grace.is_some_and(|last| last != grace) {
                    down_since = None;
                    backoff = INITIAL_RECONNECT_BACKOFF;
                }
                last_grace = Some(grace);

                let down_since = *down_since.get_or_insert_with(Instant::now);
                if down_since.elapsed() >= grace {
                    // Grace window exhausted: the fault is no longer "transient" as far
                    // as the run is concerned and we should fail.
                    return Err(e.context(format!(
                        "database unreachable for {:?}, past the {grace:?} grace window",
                        down_since.elapsed()
                    )));
                }
                crate::dst::retried_transient_fault!();
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

        let grace = fixed(Duration::from_millis(250));
        let result = tolerate_transient(&alive, &grace, |progress| {
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

    #[test]
    fn zero_grace_fails_on_the_first_transient_error() {
        let alive = AtomicBool::new(true);
        let grace = fixed(Duration::ZERO);
        let mut attempts = 0;

        let err = tolerate_transient(&alive, &grace, |_| {
            attempts += 1;
            Err::<(), _>(anyhow!("connection refused"))
        })
        .unwrap_err();

        assert_eq!(attempts, 1);
        // No `.context()` wrapping: the caller sees exactly the error it would have
        // seen before fault tolerance existed.
        assert_eq!(err.to_string(), "connection refused");
    }

    /// A grace no test will exhaust.
    const LONG_GRACE: Duration = Duration::from_secs(3600);

    fn fixed(d: Duration) -> GraceWindow {
        GraceWindow::fixed(d)
    }

    #[test]
    fn a_long_grace_rides_out_transient_faults() {
        let alive = AtomicBool::new(true);
        let grace = fixed(LONG_GRACE);
        let mut attempts = 0;

        let result = tolerate_transient(&alive, &grace, |_| {
            attempts += 1;
            if attempts < 4 {
                Err(anyhow!("connection refused"))
            } else {
                Ok("ok")
            }
        })
        .unwrap();

        assert_eq!(result, Some("ok"));
    }

    /// `main` propagates any error from a job, including one raised as the suite is
    /// shutting down. The grace-0 default must not start swallowing those.
    #[test]
    fn zero_grace_fails_even_once_the_suite_is_shutting_down() {
        let alive = AtomicBool::new(false);
        let grace = fixed(Duration::ZERO);

        let err = tolerate_transient(&alive, &grace, |_| {
            Err::<(), _>(anyhow!("connection refused"))
        })
        .unwrap_err();

        assert_eq!(err.to_string(), "connection refused");
    }

    /// With a real grace we would rather abandon a fault we are riding out than block
    /// teardown for the whole window.
    #[test]
    fn a_long_grace_gives_up_once_the_suite_is_shutting_down() {
        let alive = AtomicBool::new(false);

        let result = tolerate_transient(&alive, &fixed(LONG_GRACE), |_| {
            Err::<(), _>(anyhow!("connection refused"))
        })
        .unwrap();

        assert_eq!(result, None);
    }

    #[test]
    fn real_errors_surface_through_a_long_grace() {
        let alive = AtomicBool::new(true);
        let grace = fixed(LONG_GRACE);

        let err = tolerate_transient(&alive, &grace, |_| {
            Err::<(), _>(anyhow!(
                "duplicate key value violates unique constraint (SQLState: 23505)"
            ))
        })
        .unwrap_err();

        assert!(err.to_string().contains("duplicate key"));
    }

    #[test]
    fn poke_file_grammar_round_trips() {
        assert_eq!(GraceWindow::parse_ms("0").unwrap(), Duration::ZERO);
        assert_eq!(
            GraceWindow::parse_ms("45000\n").unwrap(),
            Duration::from_secs(45)
        );
        assert!(GraceWindow::parse_ms("").is_err());
        assert!(GraceWindow::parse_ms("infinite").is_err());
    }

    #[test]
    fn a_missing_or_malformed_poke_file_falls_back_to_the_baseline() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("grace");

        let grace = GraceWindow::pokeable(LONG_GRACE, path.clone());
        assert_eq!(grace.current(), LONG_GRACE, "missing file");

        std::fs::write(&path, "not-a-duration").unwrap();
        assert_eq!(grace.current(), LONG_GRACE, "malformed file");

        std::fs::write(&path, "45000").unwrap();
        assert_eq!(grace.current(), Duration::from_secs(45), "valid file");

        // The supervisor restores the baseline by removing the file, so it never has to
        // know what the baseline was.
        std::fs::remove_file(&path).unwrap();
        assert_eq!(grace.current(), LONG_GRACE, "file removed");
    }

    /// The liveness check: faults are healed, the supervisor narrows the window, and the
    /// run must fail if the workload cannot reconnect inside it — measured from the
    /// poke, *not* from when the (arbitrarily long) fault began.
    #[test]
    fn a_poke_restarts_the_clock_rather_than_failing_on_a_stale_down_since() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("grace");
        let grace = GraceWindow::pokeable(LONG_GRACE, path.clone());

        let alive = AtomicBool::new(true);
        let mut attempts = 0;

        let err = tolerate_transient(&alive, &grace, |_| {
            attempts += 1;
            // Ride out an arbitrarily long fault, then get poked down to 200ms. The
            // backoff between attempts already exceeds 200ms, so had the poke not
            // restarted the clock we would fail on attempt 3 — the very attempt that
            // observes it — instead of being given a fresh window.
            if attempts == 3 {
                std::fs::write(&path, "200").unwrap();
            }
            Err::<(), _>(anyhow!("connection refused"))
        })
        .unwrap_err();

        assert_eq!(
            attempts, 4,
            "the poke should grant a fresh window, failing only on the next attempt"
        );
        assert!(
            err.to_string().contains("grace window"),
            "expected the grace-exhausted context, got: {err:#}"
        );
    }

    #[test]
    fn widening_the_window_also_restarts_the_clock() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("grace");
        std::fs::write(&path, "200").unwrap();
        let grace = GraceWindow::pokeable(LONG_GRACE, path.clone());

        let alive = AtomicBool::new(true);
        let mut attempts = 0;

        // The supervisor's quiet period ends and it restores the baseline while we are
        // still down. By then we have been down longer than the 200ms window it is
        // replacing, so only restarting the clock keeps us from failing on a window that
        // is no longer in force.
        let result = tolerate_transient(&alive, &grace, |_| {
            attempts += 1;
            match attempts {
                1 => Err(anyhow!("connection refused")),
                2 => {
                    std::fs::remove_file(&path).unwrap();
                    Err(anyhow!("connection refused"))
                }
                3 => Err(anyhow!("connection refused")),
                _ => Ok("ok"),
            }
        })
        .unwrap();

        assert_eq!(result, Some("ok"));
        assert_eq!(attempts, 4);
    }
}
